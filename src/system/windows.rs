//! Implémentation Windows de `brocolis auto`.
//!
//! Étapes pilotées par `engage()` (et défaites par `revert()`) :
//!
//! 1. Ajoute le CA au magasin **Local Machine\Trusted Root Certification
//!    Authorities** via `certutil -addstore -f Root <ca-pem>`.
//! 2. Dépose la politique entreprise Firefox
//!    (`Certificates.ImportEnterpriseRoots = true`) dans chaque variante
//!    détectée — `<install>\distribution\policies.json`. Le chemin
//!    d'installation est résolu via `%ProgramFiles%` /
//!    `%ProgramFiles(x86)%` plus une liste de variantes connues
//!    (Developer Edition, Nightly).
//! 3. Règle le proxy **WinHTTP** système via
//!    `netsh winhttp set proxy proxy-server="127.0.0.1:<port>"
//!    bypass-list="<local>"`, après avoir sauvegardé l'état initial pour
//!    le restaurer à l'arrêt.
//!
//! WinHTTP ne couvre pas tous les flux utilisateur — les navigateurs
//! basés sur WinINet (Edge, Chrome) lisent
//! `HKCU\…\Internet Settings`, qui est par utilisateur. Pour ces
//! navigateurs, l'utilisateur règle le proxy à la main (ou via la
//! politique entreprise Firefox déposée, qui couvre déjà Firefox).

use super::{firefox_policy_json, Backend, SystemProxy};
use std::path::{Path, PathBuf};
use std::process::Command;

#[link(name = "shell32")]
extern "system" {
    fn IsUserAnAdmin() -> i32;
}

fn is_admin() -> bool {
    // SAFETY: IsUserAnAdmin lit le token courant, sans effet de bord.
    unsafe { IsUserAnAdmin() != 0 }
}

/// Nom commun du certificat racine (défini dans `ca.rs`, `root_params`).
const CA_CN: &str = "Brocolis Root CA";

/// État WinHTTP sauvegardé pour restauration à l'arrêt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WinHttpState {
    /// `true` si `netsh winhttp show proxy` annonçait « Direct access (no proxy server) ».
    pub was_direct: bool,
    /// Proxy initialement configuré (host:port, ou vide si direct).
    pub server: String,
    /// Bypass-list (souvent `<local>`).
    pub bypass: String,
}

/// Analyse la sortie de `netsh winhttp show proxy`.
///
/// Exemple « direct » :
/// ```text
/// Current WinHTTP proxy settings:
///     Direct access (no proxy server).
/// ```
///
/// Exemple « configuré » :
/// ```text
/// Current WinHTTP proxy settings:
///     Proxy Server(s) :  192.168.1.1:3128
///     Bypass List     :  <local>
/// ```
pub(super) fn parse_winhttp_show(out: &str) -> WinHttpState {
    let mut server = String::new();
    let mut bypass = String::new();
    let mut direct = false;
    for line in out.lines() {
        let l = line.trim();
        if l.to_lowercase().contains("direct access") {
            direct = true;
        } else if let Some(rest) = l.strip_prefix("Proxy Server(s) :") {
            let v = rest.trim();
            if !v.is_empty() && v.to_lowercase() != "(none)" {
                server = v.to_string();
            }
        } else if let Some(rest) = l.strip_prefix("Bypass List     :") {
            let v = rest.trim();
            if !v.is_empty() && v.to_lowercase() != "(none)" {
                bypass = v.to_string();
            }
        }
    }
    // Si aucun proxy lu et la mention « Direct » est absente, on traite
    // comme un état par défaut « direct » (reset à l'arrêt).
    if server.is_empty() {
        direct = true;
    }
    WinHttpState {
        was_direct: direct,
        server,
        bypass,
    }
}

fn winhttp_show() -> anyhow::Result<WinHttpState> {
    let out = Command::new("netsh").args(["winhttp", "show", "proxy"]).output()?;
    if !out.status.success() {
        anyhow::bail!(
            "netsh winhttp show proxy: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(parse_winhttp_show(&String::from_utf8_lossy(&out.stdout)))
}

fn winhttp_set_proxy(host: &str, port: u16) -> anyhow::Result<()> {
    let proxy_arg = format!("proxy-server={host}:{port}");
    let out = Command::new("netsh")
        .args(["winhttp", "set", "proxy"])
        .arg(&proxy_arg)
        .arg("bypass-list=<local>")
        .output()?;
    if !out.status.success() {
        anyhow::bail!(
            "netsh winhttp set proxy: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn winhttp_restore(saved: &WinHttpState) {
    if saved.was_direct {
        let _ = Command::new("netsh")
            .args(["winhttp", "reset", "proxy"])
            .output();
        return;
    }
    let proxy_arg = format!("proxy-server={}", saved.server);
    let bypass_arg = if saved.bypass.is_empty() {
        String::from("bypass-list=")
    } else {
        format!("bypass-list={}", saved.bypass)
    };
    let _ = Command::new("netsh")
        .args(["winhttp", "set", "proxy"])
        .arg(&proxy_arg)
        .arg(&bypass_arg)
        .output();
}

/// Variantes Firefox connues à scanner sous Windows.
const FIREFOX_VARIANTS: &[&str] = &[
    "Mozilla Firefox",
    "Firefox Developer Edition",
    "Firefox Nightly",
];

fn firefox_install_dirs() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(pf) = std::env::var("ProgramFiles") {
        roots.push(PathBuf::from(pf));
    } else {
        roots.push(PathBuf::from(r"C:\Program Files"));
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        roots.push(PathBuf::from(pf86));
    } else {
        roots.push(PathBuf::from(r"C:\Program Files (x86)"));
    }

    let mut dirs = Vec::new();
    for root in &roots {
        for variant in FIREFOX_VARIANTS {
            dirs.push(root.join(variant));
        }
    }
    // Déduplication en préservant l'ordre.
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}

fn enable_firefox() -> Vec<PathBuf> {
    let mut created = Vec::new();
    let mut found_any = false;
    for app in firefox_install_dirs() {
        if !app.exists() {
            continue;
        }
        found_any = true;
        let dist = app.join("distribution");
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
        if let Err(e) = std::fs::write(&policy, firefox_policy_json()) {
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

fn disable_firefox(policies: &[PathBuf]) {
    for policy in policies {
        let _ = std::fs::remove_file(policy);
        if let Some(dir) = policy.parent() {
            // Retire le dossier `distribution` s'il est devenu vide.
            let _ = std::fs::remove_dir(dir);
        }
    }
    if !policies.is_empty() {
        tracing::info!("Firefox: politique(s) retirée(s)");
    }
}

fn ca_present() -> bool {
    let out = Command::new("certutil")
        .args(["-store", "Root", CA_CN])
        .output();
    match out {
        Ok(o) => {
            // certutil renvoie 0 et liste les certs correspondants quand trouvé,
            // exit code non-nul (souvent 0x80092004) sinon.
            o.status.success()
                && String::from_utf8_lossy(&o.stdout).contains(CA_CN)
        }
        Err(_) => false,
    }
}

fn trust_ca(cert: &Path) -> anyhow::Result<()> {
    let out = Command::new("certutil")
        .args(["-addstore", "-f", "Root"])
        .arg(cert)
        .output()?;
    if !out.status.success() {
        anyhow::bail!(
            "certutil -addstore Root: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    tracing::info!(
        "CA approuvé dans le magasin Local Machine\\Trusted Root Certification Authorities"
    );
    Ok(())
}

fn untrust_ca() {
    let out = Command::new("certutil")
        .args(["-delstore", "Root", CA_CN])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            tracing::info!("CA retiré du magasin Trusted Root");
        }
        Ok(o) => tracing::warn!(
            "retrait du CA: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ),
        Err(e) => tracing::warn!("retrait du CA impossible: {e}"),
    }
}

pub(super) fn engage(ca_cert: &Path, port: u16) -> anyhow::Result<Option<SystemProxy>> {
    if !is_admin() {
        tracing::warn!(
            "Droits administrateur requis : relancez depuis un terminal \
             « Exécuter en tant qu'administrateur »."
        );
        return Ok(None);
    }
    // Ne retirer le CA à l'arrêt que si nous l'avons réellement ajouté.
    let ca_added = !ca_present();
    trust_ca(ca_cert)?;
    if !ca_added {
        tracing::info!("CA déjà présent dans le magasin : conservé à l'arrêt");
    }
    let saved_winhttp = winhttp_show()?;
    winhttp_set_proxy("127.0.0.1", port)?;
    tracing::info!("Proxy WinHTTP réglé sur 127.0.0.1:{port}");
    let firefox_policies = enable_firefox();
    Ok(Some(SystemProxy {
        backend: Backend::Windows {
            ca_added,
            saved_winhttp,
            firefox_policies,
        },
    }))
}

pub(super) fn revert(
    saved_winhttp: &WinHttpState,
    ca_added: bool,
    firefox_policies: &[PathBuf],
) {
    winhttp_restore(saved_winhttp);
    if ca_added {
        untrust_ca();
    }
    disable_firefox(firefox_policies);
    tracing::info!("État système Windows restauré");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_direct_state() {
        let out = "Current WinHTTP proxy settings:\n    Direct access (no proxy server).\n";
        let s = parse_winhttp_show(out);
        assert!(s.was_direct);
        assert_eq!(s.server, "");
        assert_eq!(s.bypass, "");
    }

    #[test]
    fn parses_configured_state() {
        let out = "Current WinHTTP proxy settings:\n\
                   \n\
                   \tProxy Server(s) :  192.168.1.1:3128\n\
                   \tBypass List     :  <local>\n";
        let s = parse_winhttp_show(out);
        assert!(!s.was_direct);
        assert_eq!(s.server, "192.168.1.1:3128");
        assert_eq!(s.bypass, "<local>");
    }

    #[test]
    fn empty_output_treated_as_direct() {
        let s = parse_winhttp_show("");
        assert!(s.was_direct);
        assert_eq!(s.server, "");
    }

    #[test]
    fn firefox_install_dirs_includes_known_variants() {
        let dirs = firefox_install_dirs();
        // Au moins 3 variantes × 2 ProgramFiles (avec valeurs par défaut).
        assert!(dirs.len() >= 3);
        let names: Vec<String> = dirs
            .iter()
            .filter_map(|d| d.file_name().and_then(|s| s.to_str()).map(String::from))
            .collect();
        assert!(names.iter().any(|n| n == "Mozilla Firefox"));
        assert!(names.iter().any(|n| n == "Firefox Developer Edition"));
        assert!(names.iter().any(|n| n == "Firefox Nightly"));
    }
}
