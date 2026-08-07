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

/// Pose un fichier UNIQUEMENT s'il n'existe pas déjà.
///
/// Le `Option` renvoyé est le contrat de sécurité de `auto` : il vaut `Some`
/// seulement si NOUS avons créé le fichier. C'est lui qui est mémorisé, et
/// c'est lui seul qui sera retiré à l'arrêt. Un fichier préexistant (une
/// politique Firefox posée par l'administrateur, par exemple) renvoie `None`
/// et ne sera donc jamais ni écrasé, ni supprimé.
pub(super) fn write_if_absent(path: &Path, contents: &str, quoi: &str) -> Option<PathBuf> {
    if path.exists() {
        tracing::warn!("{quoi} déjà présent ({}) : laissé intact", path.display());
        return None;
    }
    if let Some(dir) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::warn!("{quoi}: création {} impossible: {e}", dir.display());
            return None;
        }
    }
    if let Err(e) = std::fs::write(path, contents) {
        tracing::warn!("{quoi}: écriture {} impossible: {e}", path.display());
        return None;
    }
    tracing::info!("{quoi} posé ({})", path.display());
    Some(path.to_path_buf())
}

fn enable_firefox() -> Option<PathBuf> {
    write_if_absent(
        Path::new(FF_POLICY),
        firefox_policy_json(),
        "Politique Firefox",
    )
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
    write_if_absent(
        Path::new(ENV_PROXY),
        &env_proxy_content(port),
        "Variables http_proxy/https_proxy",
    )
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

    /// Répertoire de travail isolé.
    fn tmp(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("brocolis-auto-{}-{nom}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// LA garantie de `auto` : un fichier qui existait déjà n'est ni écrasé,
    /// ni revendiqué.
    ///
    /// C'est ce qui empêche `auto` de détruire la configuration de
    /// l'administrateur — une politique Firefox posée par l'entreprise, par
    /// exemple. Le `None` renvoyé fait que rien ne sera retiré à l'arrêt.
    #[test]
    fn un_fichier_preexistant_nest_ni_ecrase_ni_revendique() {
        let d = tmp("preexistant");
        let f = d.join("policies.json");
        std::fs::write(&f, "{\"politique\": \"de l'administrateur\"}").unwrap();

        let revendique = write_if_absent(&f, firefox_policy_json(), "Politique Firefox");

        assert!(revendique.is_none(), "un fichier existant ne doit pas être revendiqué");
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            "{\"politique\": \"de l'administrateur\"}",
            "le contenu de l'administrateur doit être intact"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// À l'inverse, un fichier absent est bien posé et revendiqué.
    #[test]
    fn un_fichier_absent_est_pose_et_revendique() {
        let d = tmp("absent");
        // Sous-dossier inexistant : il doit être créé au passage.
        let f = d.join("policies").join("policies.json");

        let revendique = write_if_absent(&f, firefox_policy_json(), "Politique Firefox");

        assert_eq!(revendique.as_deref(), Some(f.as_path()));
        assert!(f.exists(), "le fichier doit avoir été créé");
        assert!(
            std::fs::read_to_string(&f).unwrap().contains("ImportEnterpriseRoots"),
            "le contenu attendu doit avoir été écrit"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// La restauration ne retire QUE ce qui a été revendiqué.
    #[test]
    fn la_restauration_ne_retire_rien_si_rien_na_ete_pose() {
        let d = tmp("revert-none");
        let temoin = d.join("policies.json");
        std::fs::write(&temoin, "intact").unwrap();

        // `None` = nous n'avons rien posé.
        disable_firefox(None);

        assert!(temoin.exists(), "aucun fichier ne doit être retiré");
        assert_eq!(std::fs::read_to_string(&temoin).unwrap(), "intact");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Et elle retire bien ce qui l'a été.
    #[test]
    fn la_restauration_retire_ce_qui_a_ete_pose() {
        let d = tmp("revert-some");
        let dossier = d.join("policies");
        let f = dossier.join("policies.json");
        let pose = write_if_absent(&f, firefox_policy_json(), "Politique Firefox").unwrap();

        disable_firefox(Some(&pose));

        assert!(!f.exists(), "le fichier posé doit être retiré");
        assert!(
            !dossier.exists(),
            "le dossier créé pour l'occasion, devenu vide, doit être retiré aussi"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Un dossier qui contient AUTRE CHOSE ne doit pas être emporté.
    ///
    /// `disable_firefox` retire le dossier parent après le fichier. S'il
    /// contenait d'autres politiques, les supprimer détruirait la
    /// configuration de l'utilisateur.
    #[test]
    fn la_restauration_nemporte_pas_un_dossier_encore_occupe() {
        let d = tmp("revert-occupe");
        let dossier = d.join("policies");
        std::fs::create_dir_all(&dossier).unwrap();
        let voisin = dossier.join("autre-politique.json");
        std::fs::write(&voisin, "politique d'un tiers").unwrap();

        let f = dossier.join("policies.json");
        let pose = write_if_absent(&f, firefox_policy_json(), "Politique Firefox").unwrap();

        disable_firefox(Some(&pose));

        assert!(!f.exists(), "notre fichier est retiré");
        assert!(dossier.exists(), "le dossier encore occupé doit rester");
        assert_eq!(
            std::fs::read_to_string(&voisin).unwrap(),
            "politique d'un tiers",
            "le fichier d'un tiers doit être intact"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Poser deux fois de suite ne revendique le fichier qu'une seule fois :
    /// un `auto` relancé ne doit pas s'attribuer ce qu'il a laissé derrière.
    #[test]
    fn poser_deux_fois_ne_revendique_quune_fois() {
        let d = tmp("idempotence");
        let f = d.join("brocolis-proxy.sh");

        let premier = write_if_absent(&f, &env_proxy_content(8080), "Variables proxy");
        let second = write_if_absent(&f, &env_proxy_content(9090), "Variables proxy");

        assert!(premier.is_some(), "la première pose revendique");
        assert!(second.is_none(), "la seconde ne revendique pas");
        assert!(
            std::fs::read_to_string(&f).unwrap().contains("8080"),
            "et n'écrase pas le contenu de la première"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Le dossier de politiques doit rester cohérent avec le chemin du fichier :
    /// une faute de frappe dans l'un des deux ferait écrire au mauvais endroit.
    #[test]
    fn le_dossier_de_politique_correspond_au_chemin_du_fichier() {
        assert_eq!(
            Path::new(FF_POLICY).parent().unwrap(),
            Path::new("/etc/firefox/policies")
        );
    }

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

    #[test]
    fn env_proxy_uses_lf_line_endings() {
        // Important pour /etc/profile.d/*.sh : pas de CRLF.
        let s = env_proxy_content(8080);
        assert!(!s.contains('\r'), "le contenu env-proxy contient des CR");
    }

    #[test]
    fn env_proxy_bypass_covers_loopback_only() {
        let s = env_proxy_content(8080);
        // Doit lister localhost, 127.0.0.1 et ::1, et ne PAS contenir d'IP
        // RFC1918 ou de domaine externe (ce serait suspect).
        assert!(s.contains("localhost,127.0.0.1,::1"));
        assert!(!s.contains("10.0.0"));
        assert!(!s.contains("192.168"));
        assert!(!s.contains(".com"));
    }

    #[test]
    fn ff_policy_dir_is_etc_firefox() {
        assert_eq!(
            Path::new(FF_POLICY).parent().unwrap(),
            Path::new("/etc/firefox/policies")
        );
        assert_eq!(FF_POLICY, "/etc/firefox/policies/policies.json");
        assert_eq!(ENV_PROXY, "/etc/profile.d/brocolis-proxy.sh");
        // Le fichier env-proxy doit terminer en .sh pour être source-able
        // par /etc/profile (qui ne charge que *.sh).
        assert!(ENV_PROXY.ends_with(".sh"));
    }

    #[test]
    fn distro_trust_dirs_have_expected_paths() {
        assert_eq!(
            Distro::Debian.trust_dir().to_str(),
            Some("/usr/local/share/ca-certificates"),
        );
        assert_eq!(
            Distro::Rhel.trust_dir().to_str(),
            Some("/etc/pki/ca-trust/source/anchors"),
        );
        assert_eq!(
            Distro::Arch.trust_dir().to_str(),
            Some("/etc/ca-certificates/trust-source/anchors"),
        );
    }
}
