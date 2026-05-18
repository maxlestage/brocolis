//! Intégration système : configuration automatique du proxis et de la
//! confiance du certificat racine.
//!
//! Sur macOS, `brocolis auto` :
//! 1. fait confiance au CA dans le trousseau Système (`security`),
//! 2. règle le proxy web/HTTPS système sur `127.0.0.1:<port>`
//!    (`networksetup`),
//! 3. restaure l'état initial à l'arrêt.
//!
//! Ces opérations modifient l'état système : elles exigent les droits
//! administrateur (lancer `sudo brocolis auto`). C'est une frontière de
//! sécurité imposée par macOS, pas contournable.
//!
//! Sur les autres OS, la configuration système automatique n'est pas (encore)
//! prise en charge : le proxy tourne, mais c'est à l'utilisateur de pointer
//! son navigateur dessus.

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
/// Exemple de sortie :
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

/// Garde RAII : applique la configuration système puis la restaure.
///
/// Sur les plateformes non-macOS, [`SystemProxy::engage`] renvoie `Ok(None)`
/// (aucune modification système).
pub struct SystemProxy {
    #[cfg(target_os = "macos")]
    service: String,
    #[cfg(target_os = "macos")]
    saved_web: ProxyState,
    #[cfg(target_os = "macos")]
    saved_secure: ProxyState,
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{parse_getproxy, ProxyState, SystemProxy};
    use std::path::Path;
    use std::process::Command;

    fn run(args: &[&str]) -> anyhow::Result<String> {
        let out = Command::new("networksetup").args(args).output()?;
        if !out.status.success() {
            anyhow::bail!(
                "networksetup {:?} a échoué: {}",
                args,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    pub(super) fn is_root() -> bool {
        // SAFETY: getuid() n'a pas d'effet de bord et est toujours sûr.
        unsafe { libc_getuid() == 0 }
    }

    extern "C" {
        #[link_name = "getuid"]
        fn libc_getuid() -> u32;
    }

    /// Premier service réseau actif (non désactivé, préfixe `*`).
    fn primary_service() -> anyhow::Result<String> {
        let list = run(&["-listallnetworkservices"])?;
        for line in list.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('*') {
                continue;
            }
            return Ok(line.to_string());
        }
        anyhow::bail!("aucun service réseau actif détecté")
    }

    fn get_proxy(service: &str, secure: bool) -> anyhow::Result<ProxyState> {
        let cmd = if secure {
            "-getsecurewebproxy"
        } else {
            "-getwebproxy"
        };
        Ok(parse_getproxy(&run(&[cmd, service])?))
    }

    fn set_proxy(service: &str, secure: bool, host: &str, port: u16) -> anyhow::Result<()> {
        let (set, state) = if secure {
            ("-setsecurewebproxy", "-setsecurewebproxystate")
        } else {
            ("-setwebproxy", "-setwebproxystate")
        };
        let port = port.to_string();
        run(&[set, service, host, &port])?;
        run(&[state, service, "on"])?;
        Ok(())
    }

    fn restore_proxy(service: &str, secure: bool, saved: &ProxyState) -> anyhow::Result<()> {
        let (set, state) = if secure {
            ("-setsecurewebproxy", "-setsecurewebproxystate")
        } else {
            ("-setwebproxy", "-setwebproxystate")
        };
        if saved.enabled && !saved.server.is_empty() {
            run(&[set, service, &saved.server, &saved.port])?;
            run(&[state, service, "on"])?;
        } else {
            run(&[state, service, "off"])?;
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
        if !out.status.success() {
            // Souvent déjà approuvé : on avertit sans échouer.
            tracing::warn!(
                "ajout du CA au trousseau: {} (peut déjà être approuvé)",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        } else {
            tracing::info!("CA approuvé dans le trousseau Système");
        }
        Ok(())
    }

    pub(super) fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<SystemProxy>> {
        if !is_root() {
            tracing::warn!(
                "Configuration système ignorée : droits administrateur requis. \
                 Relancez avec `sudo brocolis auto`."
            );
            return Ok(None);
        }
        trust_ca(ca_cert)?;
        let service = primary_service()?;
        let saved_web = get_proxy(&service, false)?;
        let saved_secure = get_proxy(&service, true)?;
        set_proxy(&service, false, "127.0.0.1", port)?;
        set_proxy(&service, true, "127.0.0.1", port)?;
        tracing::info!(
            "Proxy système réglé sur 127.0.0.1:{port} (service « {service} »)"
        );
        Ok(Some(SystemProxy {
            service,
            saved_web,
            saved_secure,
        }))
    }

    pub(super) fn revert(p: &SystemProxy) {
        if let Err(e) = restore_proxy(&p.service, false, &p.saved_web) {
            tracing::error!("restauration proxy web: {e}");
        }
        if let Err(e) = restore_proxy(&p.service, true, &p.saved_secure) {
            tracing::error!("restauration proxy HTTPS: {e}");
        }
        tracing::info!("Proxy système restauré dans son état initial");
    }
}

impl SystemProxy {
    /// Applique la configuration système (macOS uniquement). Renvoie
    /// `Ok(None)` si rien n'a été modifié (autre OS, ou pas de droits root).
    #[allow(unused_variables)]
    pub fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<Self>> {
        #[cfg(target_os = "macos")]
        {
            imp::engage(ca_cert, port)
        }
        #[cfg(not(target_os = "macos"))]
        {
            tracing::warn!(
                "Configuration système automatique non prise en charge sur cet OS. \
                 Pointez votre navigateur sur le proxy manuellement."
            );
            Ok(None)
        }
    }

    /// Restaure l'état système initial.
    pub fn revert(self) {
        #[cfg(target_os = "macos")]
        imp::revert(&self);
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
    fn parse_ignores_garbage_lines() {
        let s = parse_getproxy("\n  \nEnabled:Yes\nrandom line\nPort: 8080\n");
        assert!(s.enabled);
        assert_eq!(s.port, "8080");
    }
}
