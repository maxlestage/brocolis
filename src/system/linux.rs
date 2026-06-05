//! Implémentation Linux de `brocolis auto`.
//!
//! Détecte la famille de distribution via la présence de son répertoire
//! d'« anchors » et de son outil de rebuild :
//!
//! | Famille          | Répertoire                                   | Outil                |
//! | ---------------- | -------------------------------------------- | -------------------- |
//! | Debian / Ubuntu  | `/usr/local/share/ca-certificates/`          | `update-ca-certificates` |
//! | Fedora / RHEL    | `/etc/pki/ca-trust/source/anchors/`          | `update-ca-trust`       |
//! | Arch / dérivées  | `/etc/ca-certificates/trust-source/anchors/` | `trust extract-compat`  |
//!
//! Une fois la famille connue, `engage()` dépose le CA, écrit la politique
//! entreprise Firefox dans `/etc/firefox/policies/policies.json`, et un
//! fichier `/etc/profile.d/brocolis-proxy.sh` qui rend `http_proxy` et
//! `https_proxy` actifs sur les nouvelles sessions shell. `revert()`
//! annule chaque action que nous avons effectivement réalisée (les
//! ressources préexistantes sont conservées).

use super::{firefox_policy_json, Backend, SystemProxy};
use std::path::{Path, PathBuf};
use std::process::Command;

const FF_POLICY_DIR: &str = "/etc/firefox/policies";
const FF_POLICY: &str = "/etc/firefox/policies/policies.json";
const ENV_PROXY: &str = "/etc/profile.d/brocolis-proxy.sh";
const TRUST_FILE_NAME: &str = "brocolis-ca.crt";

extern "C" {
    #[link_name = "getuid"]
    fn c_getuid() -> u32;
}

fn is_root() -> bool {
    // SAFETY: getuid() est sans effet de bord et toujours sûr.
    unsafe { c_getuid() == 0 }
}

/// Famille de distribution Linux gérée. Utilisée par le `Backend::Linux`
/// pour rejouer la mise à jour du magasin de confiance à l'arrêt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Distro {
    Debian,
    Rhel,
    Arch,
}

impl Distro {
    fn trust_dir(&self) -> &'static Path {
        Path::new(match self {
            Self::Debian => "/usr/local/share/ca-certificates",
            Self::Rhel => "/etc/pki/ca-trust/source/anchors",
            Self::Arch => "/etc/ca-certificates/trust-source/anchors",
        })
    }

    fn detect() -> anyhow::Result<Self> {
        if Path::new("/usr/local/share/ca-certificates").is_dir()
            && have_cmd("update-ca-certificates")
        {
            Ok(Self::Debian)
        } else if Path::new("/etc/pki/ca-trust/source/anchors").is_dir()
            && have_cmd("update-ca-trust")
        {
            Ok(Self::Rhel)
        } else if Path::new("/etc/ca-certificates/trust-source/anchors").is_dir()
            && have_cmd("trust")
        {
            Ok(Self::Arch)
        } else {
            anyhow::bail!(
                "magasin de confiance non détecté (essayé Debian/Ubuntu, \
                 Fedora/RHEL, Arch). Installez `ca-certificates` ou équivalent."
            )
        }
    }

    fn run_trust_update(&self) -> anyhow::Result<()> {
        let (cmd, args): (&str, &[&str]) = match self {
            Self::Debian => ("update-ca-certificates", &[]),
            Self::Rhel => ("update-ca-trust", &[]),
            Self::Arch => ("trust", &["extract-compat"]),
        };
        let out = Command::new(cmd).args(args).output()?;
        if !out.status.success() {
            anyhow::bail!(
                "{cmd}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }
}

fn have_cmd(name: &str) -> bool {
    // `command -v` (POSIX) — préférable à `which`, dispo partout. On passe
    // par sh pour ne pas hardcoder le chemin.
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", name))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn trust_ca(distro: &Distro, ca_cert: &Path, trust_file: &Path) -> anyhow::Result<()> {
    std::fs::copy(ca_cert, trust_file)?;
    // Cert public : lisible par tout le monde.
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(trust_file, std::fs::Permissions::from_mode(0o644));
    if let Err(e) = distro.run_trust_update() {
        let _ = std::fs::remove_file(trust_file);
        return Err(e);
    }
    tracing::info!(
        "CA approuvé dans le magasin de confiance ({})",
        trust_file.display()
    );
    Ok(())
}

fn untrust_ca(distro: &Distro, trust_file: &Path) {
    if let Err(e) = std::fs::remove_file(trust_file) {
        tracing::warn!("retrait du CA: {e}");
        return;
    }
    if let Err(e) = distro.run_trust_update() {
        tracing::warn!("mise à jour du magasin après retrait: {e}");
    }
    tracing::info!("CA retiré du magasin de confiance");
}

fn enable_firefox() -> Option<PathBuf> {
    let policy = Path::new(FF_POLICY);
    if policy.exists() {
        tracing::warn!(
            "Politique Firefox déjà présente ({}) : laissée intacte. \
             Activez-y `Certificates.ImportEnterpriseRoots` si besoin.",
            policy.display()
        );
        return None;
    }
    if let Err(e) = std::fs::create_dir_all(FF_POLICY_DIR) {
        tracing::warn!("Firefox: création {FF_POLICY_DIR} impossible: {e}");
        return None;
    }
    if let Err(e) = std::fs::write(policy, firefox_policy_json()) {
        tracing::warn!("Firefox: écriture {} impossible: {e}", policy.display());
        return None;
    }
    tracing::info!("Firefox: confiance activée ({})", policy.display());
    Some(policy.to_path_buf())
}

fn disable_firefox(policy: Option<&Path>) {
    if let Some(p) = policy {
        let _ = std::fs::remove_file(p);
        if let Some(dir) = p.parent() {
            // Retire `/etc/firefox/policies` s'il est devenu vide.
            let _ = std::fs::remove_dir(dir);
        }
        tracing::info!("Firefox: politique retirée");
    }
}

/// Contenu écrit dans `/etc/profile.d/brocolis-proxy.sh` pour un port donné.
pub(super) fn env_proxy_content(port: u16) -> String {
    format!(
        "# Brocolis : proxy HTTP/HTTPS via 127.0.0.1:{port}.\n\
         # Posé par `brocolis auto`, retiré à l'arrêt (Ctrl-C).\n\
         # Effet sur les NOUVELLES sessions shell. Pour la session courante :\n\
         #   source {ENV_PROXY}\n\
         export http_proxy=\"http://127.0.0.1:{port}\"\n\
         export https_proxy=\"http://127.0.0.1:{port}\"\n\
         export HTTP_PROXY=\"http://127.0.0.1:{port}\"\n\
         export HTTPS_PROXY=\"http://127.0.0.1:{port}\"\n\
         export no_proxy=\"localhost,127.0.0.1,::1\"\n\
         export NO_PROXY=\"localhost,127.0.0.1,::1\"\n"
    )
}

fn write_env_proxy(port: u16) -> Option<PathBuf> {
    let path = Path::new(ENV_PROXY);
    if path.exists() {
        tracing::warn!("{} déjà présent : laissé intact", path.display());
        return None;
    }
    match std::fs::write(path, env_proxy_content(port)) {
        Ok(()) => {
            tracing::info!(
                "Variables http_proxy/https_proxy écrites ({}) — \
                 effet sur les nouvelles sessions shell",
                path.display()
            );
            Some(path.to_path_buf())
        }
        Err(e) => {
            tracing::warn!("écriture {} impossible: {e}", path.display());
            None
        }
    }
}

fn remove_env_proxy(path: Option<&Path>) {
    if let Some(p) = path {
        let _ = std::fs::remove_file(p);
        tracing::info!("Variables proxy retirées ({})", p.display());
    }
}

pub(super) fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<SystemProxy>> {
    if !is_root() {
        tracing::warn!(
            "Droits administrateur requis : relancez `sudo brocolis auto`."
        );
        return Ok(None);
    }
    let distro = Distro::detect()?;
    let trust_file = distro.trust_dir().join(TRUST_FILE_NAME);

    // Ne retirer le CA à l'arrêt que si nous l'avons réellement ajouté.
    let ca_added = !trust_file.exists();
    if ca_added {
        trust_ca(&distro, ca_cert, &trust_file)?;
    } else {
        tracing::info!(
            "CA déjà présent dans le magasin ({}) : conservé à l'arrêt",
            trust_file.display()
        );
    }

    let firefox_policy = enable_firefox();
    let env_proxy = write_env_proxy(port);

    Ok(Some(SystemProxy {
        backend: Backend::Linux {
            distro,
            trust_file,
            ca_added,
            firefox_policy,
            env_proxy,
        },
    }))
}

pub(super) fn revert(
    distro: &Distro,
    trust_file: &Path,
    ca_added: bool,
    firefox_policy: Option<&Path>,
    env_proxy: Option<&Path>,
) {
    if ca_added {
        untrust_ca(distro, trust_file);
    }
    disable_firefox(firefox_policy);
    remove_env_proxy(env_proxy);
    tracing::info!("État système restauré");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_proxy_content_contains_port() {
        let s = env_proxy_content(8080);
        assert!(s.contains("127.0.0.1:8080"));
        assert!(s.contains("export http_proxy="));
        assert!(s.contains("export https_proxy="));
        assert!(s.contains("export HTTP_PROXY="));
        assert!(s.contains("export HTTPS_PROXY="));
        assert!(s.contains("export no_proxy="));
        assert!(s.contains("localhost,127.0.0.1,::1"));
    }

    #[test]
    fn env_proxy_content_varies_with_port() {
        let a = env_proxy_content(8080);
        let b = env_proxy_content(9000);
        assert!(a.contains("8080"));
        assert!(!a.contains("9000"));
        assert!(b.contains("9000"));
        assert!(!b.contains("8080"));
    }

    #[test]
    fn distro_trust_dirs_are_distinct() {
        let d = Distro::Debian.trust_dir();
        let r = Distro::Rhel.trust_dir();
        let a = Distro::Arch.trust_dir();
        assert_ne!(d, r);
        assert_ne!(r, a);
        assert_ne!(d, a);
        assert!(d.is_absolute());
        assert!(r.is_absolute());
        assert!(a.is_absolute());
    }
}
