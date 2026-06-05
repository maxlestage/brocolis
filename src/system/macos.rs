//! Implémentation macOS de `brocolis auto` :
//! - approbation du CA dans le trousseau Système (`security add-trusted-cert`) ;
//! - configuration des proxies web/HTTPS système (`networksetup`) ;
//! - politique entreprise Firefox + profil `.mobileconfig` alternatif ;
//! - restauration de tout à l'arrêt.

use super::{parse_getproxy, Backend, ProxyState, SystemProxy};
use std::path::Path;
use std::process::Command;

extern "C" {
    #[link_name = "getuid"]
    fn c_getuid() -> u32;
}

fn is_root() -> bool {
    // SAFETY: getuid() est sans effet de bord et toujours sûr.
    unsafe { c_getuid() == 0 }
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
    parse_primary_service(&ns(&["-listallnetworkservices"])?)
        .ok_or_else(|| anyhow::anyhow!("aucun service réseau actif"))
}

/// Analyse la sortie de `networksetup -listallnetworkservices` et renvoie
/// le premier service réseau actif (non préfixé par `*` qui marque les
/// services désactivés). La première ligne, qui est un en-tête d'avis, est
/// ignorée.
pub(super) fn parse_primary_service(out: &str) -> Option<String> {
    for line in out.lines().skip(1) {
        let l = line.trim();
        if !l.is_empty() && !l.starts_with('*') {
            return Some(l.to_string());
        }
    }
    None
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_first_active_service() {
        let out = "An asterisk (*) denotes that a network service is disabled.\n\
                   Wi-Fi\n\
                   Bluetooth PAN\n";
        assert_eq!(parse_primary_service(out).as_deref(), Some("Wi-Fi"));
    }

    #[test]
    fn skips_disabled_services() {
        let out = "An asterisk (*) denotes that a network service is disabled.\n\
                   *Ethernet\n\
                   *Wi-Fi\n\
                   Thunderbolt Bridge\n";
        assert_eq!(parse_primary_service(out).as_deref(), Some("Thunderbolt Bridge"));
    }

    #[test]
    fn skips_blank_lines() {
        let out = "Header\n\n   \nMy Service\n";
        assert_eq!(parse_primary_service(out).as_deref(), Some("My Service"));
    }

    #[test]
    fn returns_none_when_all_disabled() {
        let out = "Header\n*Wi-Fi\n*Ethernet\n";
        assert_eq!(parse_primary_service(out), None);
    }

    #[test]
    fn returns_none_on_header_only() {
        assert_eq!(parse_primary_service("Header only\n"), None);
        assert_eq!(parse_primary_service(""), None);
    }

    #[test]
    fn firefox_apps_lists_known_variants() {
        let names: Vec<&str> = FIREFOX_APPS
            .iter()
            .map(|p| p.rsplit(/).next().unwrap_or(""))
            .collect();
        assert!(names.contains(&"Firefox.app"));
        assert!(names.contains(&"Firefox Developer Edition.app"));
        assert!(names.contains(&"Firefox Nightly.app"));
        assert!(names.contains(&"Firefox ESR.app"));
    }

    #[test]
    fn ca_cn_matches_root_params() {
        // Sanity : si on renomme le CA dans ca.rs, ce test bougera ici aussi.
        assert_eq!(CA_CN, "Brocolis Root CA");
    }
}
