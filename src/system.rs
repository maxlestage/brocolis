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

    fn trust_ca(cert: &Path) -> anyhow::Result<()> {
        let out = Command::new("security")
            .args([
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                "/Library/Keychains/System.keychain",
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

    pub(super) fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<SystemProxy>> {
        if !is_root() {
            tracing::warn!(
                "Droits administrateur requis : relancez `sudo brocolis auto`."
            );
            return Ok(None);
        }
        trust_ca(ca_cert)?;
        let service = primary_service()?;
        let saved_web = get(&service, false)?;
        let saved_secure = get(&service, true)?;
        set(&service, false, "127.0.0.1", port)?;
        set(&service, true, "127.0.0.1", port)?;
        tracing::info!("Proxy système réglé sur 127.0.0.1:{port} (« {service} »)");
        Ok(Some(SystemProxy {
            backend: Backend::MacOs {
                service,
                saved_web,
                saved_secure,
            },
        }))
    }

    pub(super) fn revert(service: &str, web: &ProxyState, secure: &ProxyState) {
        if let Err(e) = restore(service, false, web) {
            tracing::error!("restauration proxy web: {e}");
        }
        if let Err(e) = restore(service, true, secure) {
            tracing::error!("restauration proxy HTTPS: {e}");
        }
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
            } => macos::revert(&service, &saved_web, &saved_secure),
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
