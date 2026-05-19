//! Intégration système macOS : configuration automatique du proxy et de la
//! confiance du certificat racine, restaurée à l'arrêt.
//!
//! `brocolis auto` (macOS) :
//! 1. ajoute le CA au **trousseau Système** (`security add-trusted-cert`),
//! 2. règle le **proxy web et HTTPS système** sur `127.0.0.1:<port>`
//!    (`networksetup`),
//! 3. restaure l'état initial à l'arrêt (Ctrl-C / SIGTERM).
//!
//! Les étapes 1 et 2 modifient l'état du système et exigent donc les droits
//! administrateur : il faut lancer `sudo brocolis auto`. macOS impose une
//! seule invite de mot de passe — c'est une frontière de sécurité de l'OS,
//! non contournable.
//!
//! Sur les autres systèmes d'exploitation, la configuration système
//! automatique n'est pas prise en charge : le proxy démarre quand même, mais
//! c'est à l'utilisateur de pointer son client dessus.

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
/// trousseau Système, elle fait que Firefox lui fait confiance sans aucune
/// action de l'utilisateur.
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
/// [`SystemProxy::engage`] renvoie `Ok(None)` si rien n'a été modifié (OS non
/// macOS, ou droits administrateur absents).
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
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{parse_getproxy, Backend, ProxyState, SystemProxy};
    use std::path::Path;
    use std::process::Command;

    fn is_root() -> bool {
        // SAFETY: getuid() est sans effet de bord et toujours sûr.
        unsafe { c_getuid() == 0 }
    }

    extern "C" {
        #[link_name = "getuid"]
        fn c_getuid() -> u32;
    }

    fn ns(args: &[&str]) -> anyhow::Result<String> {
        let out = Command::new("networksetup").args(args).output()?;
        if !out.status.success() {
            anyhow::bail!(
                "networksetup {:?}: {}",
                args,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn primary_service() -> anyhow::Result<String> {
        for line in ns(&["-listallnetworkservices"])?.lines().skip(1) {
            let l = line.trim();
            if !l.is_empty() && !l.starts_with('*') {
                return Ok(l.to_string());
            }
        }
        anyhow::bail!("aucun service réseau actif")
    }

    fn get(service: &str, secure: bool) -> anyhow::Result<ProxyState> {
        let cmd = if secure {
            "-getsecurewebproxy"
        } else {
            "-getwebproxy"
        };
        Ok(parse_getproxy(&ns(&[cmd, service])?))
    }

    fn set(service: &str, secure: bool, host: &str, port: u16) -> anyhow::Result<()> {
        let (a, b) = if secure {
            ("-setsecurewebproxy", "-setsecurewebproxystate")
        } else {
            ("-setwebproxy", "-setwebproxystate")
        };
        ns(&[a, service, host, &port.to_string()])?;
        ns(&[b, service, "on"])?;
        Ok(())
    }

    fn restore(service: &str, secure: bool, s: &ProxyState) -> anyhow::Result<()> {
        let (a, b) = if secure {
            ("-setsecurewebproxy", "-setsecurewebproxystate")
        } else {
            ("-setwebproxy", "-setwebproxystate")
        };
        if s.enabled && !s.server.is_empty() {
            ns(&[a, service, &s.server, &s.port])?;
            ns(&[b, service, "on"])?;
        } else {
            ns(&[b, service, "off"])?;
        }
        Ok(())
    }

    /// Nom commun du certificat racine (défini dans `ca.rs`, `root_params`).
    const CA_CN: &str = "Brocolis Root CA";
    const SYSTEM_KEYCHAIN: &str = "/Library/Keychains/System.keychain";

    /// Le CA est-il déjà présent dans le trousseau Système ?
    fn ca_present() -> bool {
        Command::new("security")
            .args(["find-certificate", "-c", CA_CN, SYSTEM_KEYCHAIN])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn trust_ca(cert: &Path) -> anyhow::Result<()> {
        let out = Command::new("security")
            .args([
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                SYSTEM_KEYCHAIN,
            ])
            .arg(cert)
            .output()?;
        if out.status.success() {
            tracing::info!("CA approuvé dans le trousseau Système");
        } else {
            tracing::warn!(
                "ajout du CA: {} (peut déjà être approuvé)",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }

    /// Retire le CA du trousseau Système (uniquement s'il a été ajouté par
    /// nous). `security delete-certificate` exige les droits root.
    fn untrust_ca() {
        let out = Command::new("security")
            .args(["delete-certificate", "-c", CA_CN, SYSTEM_KEYCHAIN])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                tracing::info!("CA retiré du trousseau Système");
            }
            Ok(o) => tracing::warn!(
                "retrait du CA: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ),
            Err(e) => tracing::warn!("retrait du CA impossible: {e}"),
        }
    }

    /// Variantes de Firefox potentiellement installées.
    const FIREFOX_APPS: &[&str] = &[
        "/Applications/Firefox.app",
        "/Applications/Firefox Developer Edition.app",
        "/Applications/Firefox Nightly.app",
        "/Applications/Firefox ESR.app",
    ];

    /// Active la confiance du CA dans toutes les variantes de Firefox
    /// détectées via la politique entreprise (Firefox n'utilise pas le
    /// trousseau macOS). Renvoie la liste des fichiers de politique *créés
    /// par nous* (à retirer à l'arrêt). Une politique préexistante n'est
    /// jamais écrasée.
    fn enable_firefox() -> Vec<std::path::PathBuf> {
        let mut created = Vec::new();
        let mut found_any = false;
        for app in FIREFOX_APPS {
            let app = Path::new(app);
            if !app.exists() {
                continue;
            }
            found_any = true;
            let dist = app.join("Contents/Resources/distribution");
            let policy = dist.join("policies.json");
            if policy.exists() {
                tracing::warn!(
                    "Politique Firefox déjà présente ({}) : laissée intacte. \
                     Activez-y `Certificates.ImportEnterpriseRoots` si besoin.",
                    policy.display()
                );
                continue;
            }
            if let Err(e) = std::fs::create_dir_all(&dist) {
                tracing::warn!("Firefox: création {} impossible: {e}", dist.display());
                continue;
            }
            if let Err(e) = std::fs::write(&policy, super::firefox_policy_json()) {
                tracing::warn!("Firefox: écriture {} impossible: {e}", policy.display());
                continue;
            }
            tracing::info!("Firefox: confiance activée ({})", policy.display());
            created.push(policy);
        }
        if !found_any {
            tracing::info!("Aucun Firefox détecté : étape ignorée");
        }
        created
    }

    /// Écrit le profil de configuration `.mobileconfig` à côté du CA, comme
    /// alternative ne modifiant pas le bundle Firefox. Renvoie son chemin.
    fn write_mobileconfig(ca_cert: &Path) -> Option<std::path::PathBuf> {
        let dir = ca_cert.parent().unwrap_or_else(|| Path::new("."));
        let path = dir.join("brocolis-firefox.mobileconfig");
        match std::fs::write(&path, super::firefox_mobileconfig()) {
            Ok(()) => {
                tracing::info!(
                    "Profil de configuration écrit : {} (alternative ne \
                     modifiant pas le bundle ; nécessite une approbation \
                     manuelle dans Réglages Système)",
                    path.display()
                );
                Some(path)
            }
            Err(e) => {
                tracing::warn!("Écriture du .mobileconfig impossible: {e}");
                None
            }
        }
    }

    fn disable_firefox(policies: &[std::path::PathBuf], mobileconfig: Option<&Path>) {
        for policy in policies {
            let _ = std::fs::remove_file(policy);
            if let Some(dir) = policy.parent() {
                // Retire le dossier `distribution` s'il est devenu vide.
                let _ = std::fs::remove_dir(dir);
            }
        }
        if let Some(mc) = mobileconfig {
            let _ = std::fs::remove_file(mc);
        }
        if !policies.is_empty() || mobileconfig.is_some() {
            tracing::info!("Firefox: politique(s) / profil retirés");
        }
    }

    pub(super) fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<SystemProxy>> {
        if !is_root() {
            tracing::warn!(
                "Droits administrateur requis : relancez `sudo brocolis auto`."
            );
            return Ok(None);
        }
        // Ne retirer le CA à l'arrêt que si nous l'avons réellement ajouté.
        let ca_added = !ca_present();
        trust_ca(ca_cert)?;
        if !ca_added {
            tracing::info!("CA déjà présent dans le trousseau : conservé à l'arrêt");
        }
        let service = primary_service()?;
        let saved_web = get(&service, false)?;
        let saved_secure = get(&service, true)?;
        set(&service, false, "127.0.0.1", port)?;
        set(&service, true, "127.0.0.1", port)?;
        tracing::info!("Proxy système réglé sur 127.0.0.1:{port} (« {service} »)");
        let firefox_policies = enable_firefox();
        let mobileconfig = write_mobileconfig(ca_cert);
        Ok(Some(SystemProxy {
            backend: Backend::MacOs {
                service,
                saved_web,
                saved_secure,
                ca_added,
                firefox_policies,
                mobileconfig,
            },
        }))
    }

    pub(super) fn revert(
        service: &str,
        web: &ProxyState,
        secure: &ProxyState,
        ca_added: bool,
        firefox_policies: &[std::path::PathBuf],
        mobileconfig: Option<&Path>,
    ) {
        if let Err(e) = restore(service, false, web) {
            tracing::error!("restauration proxy web: {e}");
        }
        if let Err(e) = restore(service, true, secure) {
            tracing::error!("restauration proxy HTTPS: {e}");
        }
        if ca_added {
            untrust_ca();
        }
        disable_firefox(firefox_policies, mobileconfig);
        tracing::info!("Proxy système restauré");
    }
}

impl SystemProxy {
    /// Applique la configuration système macOS. Renvoie `Ok(None)` si rien
    /// n'a été modifié (OS non macOS, ou pas de droits administrateur).
    #[allow(unused_variables)]
    pub fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<Self>> {
        #[cfg(target_os = "macos")]
        {
            macos::engage(ca_cert, port)
        }
        #[cfg(not(target_os = "macos"))]
        {
            tracing::warn!(
                "Configuration système automatique prise en charge uniquement \
                 sur macOS. Pointez votre client sur le proxy manuellement."
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
