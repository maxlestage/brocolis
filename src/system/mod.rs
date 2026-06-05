//! Intégration système : configuration automatique du proxy et de la
//! confiance du certificat racine, restaurée à l'arrêt.
//!
//! `brocolis auto` couvre nativement deux familles d'OS :
//!
//! - **macOS** (`macos.rs`) :
//!     1. ajoute le CA au trousseau Système (`security add-trusted-cert`),
//!     2. règle les proxies web/HTTPS système (`networksetup`),
//!     3. politique entreprise Firefox + `.mobileconfig` alternatif,
//!     4. restaure tout à l'arrêt.
//!
//! - **Linux** (`linux.rs`) — Debian/Ubuntu, Fedora/RHEL et Arch détectés
//!   via leurs outils respectifs :
//!     1. ajoute le CA au magasin de confiance distro
//!        (`update-ca-certificates` / `update-ca-trust` / `trust extract-compat`),
//!     2. politique entreprise Firefox dans `/etc/firefox/policies/`,
//!     3. variables `http_proxy`/`https_proxy` dans `/etc/profile.d/`
//!        (effet sur les nouvelles sessions shell),
//!     4. restaure tout à l'arrêt.
//!
//! Les étapes modifient l'état système et exigent donc les droits
//! administrateur : il faut lancer `sudo brocolis auto`. Sans `sudo`,
//! le proxy démarre quand même mais ne touche à rien (équivalent
//! `--no-system`) — l'utilisateur configure son client à la main.
//!
//! Sur les autres OS (Windows, BSD, …), la configuration système
//! automatique n'est pas prise en charge : seul le proxy démarre.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;

#[allow(unused_imports)]
use std::path::Path;

/// État d'un proxy `networksetup` (web ou sécurisé).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyState {
    pub enabled: bool,
    pub server: String,
    pub port: String,
}

/// Analyse la sortie de `networksetup -getwebproxy "<service>"`.
///
/// Exemple :
/// ```text
/// Enabled: No
/// Server:
/// Port: 0
/// Authenticated Proxy Enabled: 0
/// ```
pub fn parse_getproxy(out: &str) -> ProxyState {
    let mut enabled = false;
    let mut server = String::new();
    let mut port = String::new();
    for line in out.lines() {
        let (k, v) = match line.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };
        match k {
            "Enabled" => enabled = v.eq_ignore_ascii_case("yes"),
            "Server" => server = v.to_string(),
            "Port" => port = v.to_string(),
            _ => {}
        }
    }
    ProxyState {
        enabled,
        server,
        port,
    }
}

/// Politique entreprise Firefox demandant l'import des racines du système
/// d'exploitation (`ImportEnterpriseRoots`). Combinée à l'ajout du CA au
/// magasin de confiance du système, elle fait que Firefox lui fait
/// confiance sans aucune action de l'utilisateur.
pub fn firefox_policy_json() -> &'static str {
    "{\n  \"policies\": {\n    \"Certificates\": {\n      \"ImportEnterpriseRoots\": true\n    }\n  }\n}\n"
}

/// Profil de configuration macOS (`.mobileconfig`) — alternative qui **ne
/// modifie pas** le bundle de Firefox.app. Il pousse les préférences gérées
/// `EnterprisePoliciesEnabled` + `Certificates.ImportEnterpriseRoots` pour
/// `org.mozilla.firefox`. Note : sur macOS récent, son installation via
/// `profiles`/double-clic demande une approbation manuelle dans Réglages
/// Système (contrainte macOS).
pub fn firefox_mobileconfig() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>PayloadContent</key>
  <array>
    <dict>
      <key>PayloadType</key>
      <string>com.apple.ManagedClient.preferences</string>
      <key>PayloadVersion</key>
      <integer>1</integer>
      <key>PayloadIdentifier</key>
      <string>com.maxlestage.brocolis.firefox</string>
      <key>PayloadUUID</key>
      <string>4F0E8A10-2C3D-4E5F-9A1B-BR0C0L15F1RE</string>
      <key>PayloadDisplayName</key>
      <string>Brocolis Firefox</string>
      <key>PayloadContent</key>
      <dict>
        <key>org.mozilla.firefox</key>
        <dict>
          <key>Forced</key>
          <array>
            <dict>
              <key>mcx_preference_settings</key>
              <dict>
                <key>EnterprisePoliciesEnabled</key>
                <true/>
                <key>Certificates</key>
                <dict>
                  <key>ImportEnterpriseRoots</key>
                  <true/>
                </dict>
              </dict>
            </dict>
          </array>
        </dict>
      </dict>
    </dict>
  </array>
  <key>PayloadDisplayName</key>
  <string>Brocolis — Confiance CA Firefox</string>
  <key>PayloadIdentifier</key>
  <string>com.maxlestage.brocolis</string>
  <key>PayloadType</key>
  <string>Configuration</string>
  <key>PayloadUUID</key>
  <string>1B2C3D4E-5F60-4712-8A9B-BR0C0L15C0NF</string>
  <key>PayloadVersion</key>
  <integer>1</integer>
  <key>PayloadScope</key>
  <string>System</string>
</dict>
</plist>
"#
}

/// Configuration système active, restaurée par [`SystemProxy::revert`].
/// [`SystemProxy::engage`] renvoie `Ok(None)` si rien n'a été modifié (OS
/// non pris en charge, ou droits administrateur absents).
pub struct SystemProxy {
    #[allow(dead_code)]
    backend: Backend,
}

#[allow(dead_code)]
enum Backend {
    Inactive,
    #[cfg(target_os = "macos")]
    MacOs {
        service: String,
        saved_web: ProxyState,
        saved_secure: ProxyState,
        /// Le CA a-t-il été ajouté par nous (donc à retirer à l'arrêt) ?
        ca_added: bool,
        /// Fichiers `policies.json` Firefox créés par nos soins (à retirer
        /// à l'arrêt), un par variante de Firefox détectée.
        firefox_policies: Vec<std::path::PathBuf>,
        /// Profil `.mobileconfig` généré (à retirer à l'arrêt).
        mobileconfig: Option<std::path::PathBuf>,
    },
    #[cfg(target_os = "linux")]
    Linux {
        /// Famille de distribution détectée (pour rejouer la mise à jour
        /// du magasin de confiance à l'arrêt).
        distro: linux::Distro,
        /// Chemin du CA déposé dans le magasin de confiance.
        trust_file: std::path::PathBuf,
        /// Le CA a-t-il été ajouté par nous (donc à retirer à l'arrêt) ?
        ca_added: bool,
        /// `policies.json` Firefox créé par nos soins (à retirer à l'arrêt).
        firefox_policy: Option<std::path::PathBuf>,
        /// Fichier `/etc/profile.d/brocolis-proxy.sh` créé par nos soins.
        env_proxy: Option<std::path::PathBuf>,
    },
}

impl SystemProxy {
    /// Applique la configuration système. Renvoie `Ok(None)` si rien n'a
    /// été modifié (OS non pris en charge, ou pas de droits administrateur).
    #[allow(unused_variables)]
    pub fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<Self>> {
        #[cfg(target_os = "macos")]
        {
            return macos::engage(ca_cert, port);
        }
        #[cfg(target_os = "linux")]
        {
            return linux::engage(ca_cert, port);
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            tracing::warn!(
                "Configuration système automatique prise en charge \
                 uniquement sur macOS et Linux. Pointez votre client \
                 sur le proxy manuellement."
            );
            Ok(None)
        }
    }

    /// Restaure l'état système initial.
    pub fn revert(self) {
        match self.backend {
            Backend::Inactive => {}
            #[cfg(target_os = "macos")]
            Backend::MacOs {
                service,
                saved_web,
                saved_secure,
                ca_added,
                firefox_policies,
                mobileconfig,
            } => macos::revert(
                &service,
                &saved_web,
                &saved_secure,
                ca_added,
                &firefox_policies,
                mobileconfig.as_deref(),
            ),
            #[cfg(target_os = "linux")]
            Backend::Linux {
                distro,
                trust_file,
                ca_added,
                firefox_policy,
                env_proxy,
            } => linux::revert(
                &distro,
                &trust_file,
                ca_added,
                firefox_policy.as_deref(),
                env_proxy.as_deref(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_disabled_proxy() {
        let s = parse_getproxy("Enabled: No\nServer:\nPort: 0\nAuthenticated Proxy Enabled: 0\n");
        assert!(!s.enabled);
        assert_eq!(s.server, "");
        assert_eq!(s.port, "0");
    }

    #[test]
    fn parse_enabled_proxy() {
        let s = parse_getproxy("Enabled: Yes\nServer: 10.0.0.2\nPort: 3128\n");
        assert!(s.enabled);
        assert_eq!(s.server, "10.0.0.2");
        assert_eq!(s.port, "3128");
    }

    #[test]
    fn firefox_policy_is_well_formed() {
        let p = firefox_policy_json();
        assert!(p.trim_start().starts_with('{'));
        assert!(p.trim_end().ends_with('}'));
        assert!(p.contains("\"policies\""));
        assert!(p.contains("\"Certificates\""));
        assert!(p.contains("\"ImportEnterpriseRoots\""));
        assert!(p.contains("true"));
        // Accolades équilibrées.
        let open = p.matches('{').count();
        let close = p.matches('}').count();
        assert_eq!(open, close);
        assert!(open >= 3);
    }

    #[test]
    fn mobileconfig_is_well_formed() {
        let m = firefox_mobileconfig();
        assert!(m.starts_with("<?xml"));
        assert!(m.contains("<!DOCTYPE plist"));
        assert!(m.contains("com.apple.ManagedClient.preferences"));
        assert!(m.contains("org.mozilla.firefox"));
        assert!(m.contains("ImportEnterpriseRoots"));
        assert!(m.contains("EnterprisePoliciesEnabled"));
        assert!(m.contains("<key>PayloadType</key>"));
        assert!(m.trim_end().ends_with("</plist>"));
        // Balises plist/dict équilibrées.
        assert_eq!(m.matches("<plist").count(), m.matches("</plist>").count());
        assert_eq!(m.matches("<dict>").count(), m.matches("</dict>").count());
        assert_eq!(m.matches("<array>").count(), m.matches("</array>").count());
    }

    #[test]
    fn parse_is_case_insensitive_for_enabled() {
        assert!(parse_getproxy("Enabled: yes\n").enabled);
        assert!(parse_getproxy("Enabled: YES\n").enabled);
        assert!(!parse_getproxy("Enabled: no\n").enabled);
    }

    #[test]
    fn parse_ignores_garbage_lines() {
        let s = parse_getproxy("\n  \nEnabled:Yes\nrandom line\nPort: 8080\n");
        assert!(s.enabled);
        assert_eq!(s.port, "8080");
    }

    #[test]
    fn parse_empty_input_is_disabled() {
        let s = parse_getproxy("");
        assert!(!s.enabled);
        assert_eq!(s.server, "");
        assert_eq!(s.port, "");
    }
}
