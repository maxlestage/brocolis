//! Intégration système : configuration automatique du proxy et de la
//! confiance du certificat racine, restaurée à l'arrêt.
//!
//! - **macOS** : trousseau Système (`security`) + proxy web/HTTPS
//!   (`networksetup`). Nécessite les droits administrateur (`sudo`).
//! - **Linux** : magasin de certificats système
//!   (`update-ca-certificates` / `update-ca-trust`, nécessite root) +
//!   proxy GNOME (`gsettings`, par utilisateur, sans root).
//! - **Autres OS** : non pris en charge ; le proxy tourne, à l'utilisateur
//!   de pointer son client dessus.
//!
//! Ces opérations modifient l'état système : c'est une frontière de sécurité
//! imposée par l'OS, non contournable.

#[allow(unused_imports)]
use std::path::Path;

/// État d'un proxy `networksetup` (macOS).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyState {
    pub enabled: bool,
    pub server: String,
    pub port: String,
}

/// Analyse la sortie de `networksetup -getwebproxy "<service>"`.
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

/// Retire d'éventuelles apostrophes englobantes (`gsettings get` renvoie
/// `'manual'`, `'127.0.0.1'`, ...). Les entiers sont laissés tels quels.
pub fn gsettings_unquote(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

#[cfg(unix)]
fn is_root() -> bool {
    // SAFETY: getuid() est sans effet de bord et toujours sûr.
    unsafe { c_getuid() == 0 }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "getuid"]
    fn c_getuid() -> u32;
}

#[cfg(unix)]
fn in_path(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let p = dir.join(cmd);
                p.is_file()
            })
        })
        .unwrap_or(false)
}

/// Configuration système active, restaurée à la destruction logique
/// (`revert`). [`SystemProxy::engage`] renvoie `Ok(None)` si rien n'a été
/// modifié (OS non pris en charge, outils absents, droits insuffisants).
pub struct SystemProxy {
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
    #[cfg(target_os = "linux")]
    Linux {
        gnome: Option<linux::GnomeBackup>,
        ca_path: Option<std::path::PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// macOS
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
mod macos {
    use super::{is_root, parse_getproxy, Backend, ProxyState, SystemProxy};
    use std::path::Path;
    use std::process::Command;

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
        let cmd = if secure { "-getsecurewebproxy" } else { "-getwebproxy" };
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

// ---------------------------------------------------------------------------
// Linux (proxy GNOME + magasin de certificats système)
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
mod linux {
    use super::{gsettings_unquote, in_path, is_root, Backend, SystemProxy};
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const SCHEMA: &str = "org.gnome.system.proxy";

    #[derive(Debug, Clone)]
    pub struct GnomeBackup {
        mode: String,
        http_host: String,
        http_port: String,
        https_host: String,
        https_port: String,
    }

    fn gs_get(schema: &str, key: &str) -> Option<String> {
        let out = Command::new("gsettings").args(["get", schema, key]).output().ok()?;
        if !out.status.success() {
            return None;
        }
        Some(gsettings_unquote(&String::from_utf8_lossy(&out.stdout)))
    }

    fn gs_set(schema: &str, key: &str, value: &str) -> anyhow::Result<()> {
        let out = Command::new("gsettings")
            .args(["set", schema, key, value])
            .output()?;
        if !out.status.success() {
            anyhow::bail!(
                "gsettings set {schema} {key}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }

    fn gnome_available() -> bool {
        in_path("gsettings") && gs_get(SCHEMA, "mode").is_some()
    }

    fn save() -> Option<GnomeBackup> {
        Some(GnomeBackup {
            mode: gs_get(SCHEMA, "mode")?,
            http_host: gs_get(&format!("{SCHEMA}.http"), "host").unwrap_or_default(),
            http_port: gs_get(&format!("{SCHEMA}.http"), "port").unwrap_or_else(|| "0".into()),
            https_host: gs_get(&format!("{SCHEMA}.https"), "host").unwrap_or_default(),
            https_port: gs_get(&format!("{SCHEMA}.https"), "port").unwrap_or_else(|| "0".into()),
        })
    }

    fn apply(port: u16) -> anyhow::Result<()> {
        let p = port.to_string();
        gs_set(&format!("{SCHEMA}.http"), "host", "127.0.0.1")?;
        gs_set(&format!("{SCHEMA}.http"), "port", &p)?;
        gs_set(&format!("{SCHEMA}.https"), "host", "127.0.0.1")?;
        gs_set(&format!("{SCHEMA}.https"), "port", &p)?;
        gs_set(SCHEMA, "mode", "manual")?;
        Ok(())
    }

    pub(super) fn restore(b: &GnomeBackup) {
        let _ = gs_set(&format!("{SCHEMA}.http"), "host", &b.http_host);
        let _ = gs_set(&format!("{SCHEMA}.http"), "port", &b.http_port);
        let _ = gs_set(&format!("{SCHEMA}.https"), "host", &b.https_host);
        let _ = gs_set(&format!("{SCHEMA}.https"), "port", &b.https_port);
        let _ = gs_set(SCHEMA, "mode", &b.mode);
        tracing::info!("Proxy GNOME restauré");
    }

    /// Renvoie (chemin du fichier installé, commande de mise à jour).
    fn trust_layout() -> Option<(PathBuf, &'static str)> {
        if in_path("update-ca-certificates") {
            Some((
                PathBuf::from("/usr/local/share/ca-certificates/brocolis.crt"),
                "update-ca-certificates",
            ))
        } else if in_path("update-ca-trust") {
            Some((
                PathBuf::from("/etc/pki/ca-trust/source/anchors/brocolis.crt"),
                "update-ca-trust",
            ))
        } else {
            None
        }
    }

    fn trust_ca(cert: &Path) -> anyhow::Result<Option<PathBuf>> {
        if !is_root() {
            tracing::warn!(
                "CA non installé dans le magasin système (root requis : \
                 `sudo brocolis auto`)."
            );
            return Ok(None);
        }
        let (dest, tool) = match trust_layout() {
            Some(v) => v,
            None => {
                tracing::warn!("aucun outil de magasin de certificats détecté");
                return Ok(None);
            }
        };
        std::fs::copy(cert, &dest)?;
        let st = Command::new(tool).status()?;
        if !st.success() {
            tracing::warn!("{tool} a renvoyé une erreur");
        } else {
            tracing::info!("CA approuvé dans le magasin système ({tool})");
        }
        Ok(Some(dest))
    }

    pub(super) fn untrust_ca(path: &Path) {
        let _ = std::fs::remove_file(path);
        let tool = if in_path("update-ca-certificates") {
            "update-ca-certificates"
        } else {
            "update-ca-trust"
        };
        let _ = Command::new(tool).status();
        tracing::info!("CA retiré du magasin système");
    }

    pub(super) fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<SystemProxy>> {
        let gnome = if gnome_available() {
            let backup = save();
            apply(port)?;
            tracing::info!("Proxy GNOME réglé sur 127.0.0.1:{port}");
            backup
        } else {
            tracing::warn!(
                "GNOME (gsettings) indisponible : configurez le proxy \
                 manuellement (http(s)_proxy=127.0.0.1:{port})."
            );
            None
        };
        let ca_path = trust_ca(ca_cert)?;
        if gnome.is_none() && ca_path.is_none() {
            return Ok(None);
        }
        Ok(Some(SystemProxy {
            backend: Backend::Linux { gnome, ca_path },
        }))
    }
}

impl SystemProxy {
    /// Applique la configuration système. `Ok(None)` si rien n'a été modifié.
    #[allow(unused_variables)]
    pub fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<Self>> {
        #[cfg(target_os = "macos")]
        {
            macos::engage(ca_cert, port)
        }
        #[cfg(target_os = "linux")]
        {
            linux::engage(ca_cert, port)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            tracing::warn!(
                "Configuration système automatique non prise en charge sur cet \
                 OS. Pointez votre client sur le proxy manuellement."
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
            #[cfg(target_os = "linux")]
            Backend::Linux { gnome, ca_path } => {
                if let Some(b) = gnome {
                    linux::restore(&b);
                }
                if let Some(p) = ca_path {
                    linux::untrust_ca(&p);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_disabled_proxy() {
        let s = parse_getproxy("Enabled: No\nServer:\nPort: 0\n");
        assert!(!s.enabled);
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
        let s = parse_getproxy("\n  \nEnabled:Yes\nrandom\nPort: 8080\n");
        assert!(s.enabled);
        assert_eq!(s.port, "8080");
    }

    #[test]
    fn gsettings_unquote_strips_quotes() {
        assert_eq!(gsettings_unquote("'manual'"), "manual");
        assert_eq!(gsettings_unquote("  '127.0.0.1' \n"), "127.0.0.1");
    }

    #[test]
    fn gsettings_unquote_leaves_integers_and_bare() {
        assert_eq!(gsettings_unquote("8080"), "8080");
        assert_eq!(gsettings_unquote("none"), "none");
        assert_eq!(gsettings_unquote("''"), "");
    }
}
