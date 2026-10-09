//! Lancer `brocolis on` / `off` avec les droits administrateur.
//!
//! Brancher le système sur le proxy (magasin de confiance, proxy système)
//! exige ces droits : aucun logiciel ne peut s'en passer. L'application ne
//! les garde jamais — elle demande, au moment du clic, via la boîte de
//! dialogue native de chaque système :
//!
//! - macOS : `osascript … with administrator privileges` (mot de passe ou
//!   Touch ID) ;
//! - Linux : `pkexec` (l'agent polkit du bureau) ;
//! - Windows : `Start-Process -Verb RunAs` (contrôle de compte d'utilisateur).
//!
//! C'est la ligne de commande `brocolis`, livrée à côté de l'application, qui
//! fait le travail : une seule implémentation de `on`/`off`, déjà testée.
//!
//! La construction des commandes est séparée de leur exécution, et paramétrée
//! par l'OS : les règles de citation des trois systèmes se testent ainsi
//! toutes, sur n'importe quelle machine.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Mac,
    Linux,
    Windows,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Os::Mac
        } else if cfg!(windows) {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

/// Programme à lancer et ses arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub program: String,
    pub args: Vec<String>,
}

/// Citation shell POSIX : entre apostrophes, une apostrophe devenant `'\''`.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Littéral de chaîne AppleScript : entre guillemets, `\` et `"` échappés.
fn applescript_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', r"\\").replace('"', "\\\""))
}

/// Un argument de ligne de commande Windows, selon les règles de
/// `CommandLineToArgvW` : entre guillemets s'il contient une espace, les
/// guillemets internes et les barres obliques inverses qui les précèdent
/// échappés.
fn win_arg(s: &str) -> String {
    if !s.is_empty() && !s.contains([' ', '\t', '"']) {
        return s.to_owned();
    }
    let mut out = String::from("\"");
    let mut slashes = 0;
    for c in s.chars() {
        match c {
            '\\' => slashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(slashes * 2 + 1));
                out.push('"');
                slashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(slashes));
                out.push(c);
                slashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}

/// Chaîne PowerShell entre apostrophes (une apostrophe se double).
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// La commande qui lance `cli args…` avec élévation, pour `os`.
pub fn plan(os: Os, cli: &Path, args: &[String]) -> Plan {
    let cli = cli.display().to_string();
    match os {
        Os::Mac => {
            let line = std::iter::once(cli.as_str())
                .chain(args.iter().map(String::as_str))
                .map(sh_quote)
                .collect::<Vec<_>>()
                .join(" ");
            Plan {
                program: "osascript".into(),
                args: vec![
                    "-e".into(),
                    format!(
                        "do shell script {} with administrator privileges",
                        applescript_string(&line)
                    ),
                ],
            }
        }
        Os::Linux => Plan {
            program: "pkexec".into(),
            args: std::iter::once(cli).chain(args.iter().cloned()).collect(),
        },
        Os::Windows => {
            // `-ArgumentList` reçoit UNE chaîne déjà citée : passé en tableau,
            // PowerShell joindrait les éléments par des espaces sans les citer,
            // et un chemin comme `C:\Users\Jean Dupont\…` serait coupé en deux.
            let arglist = args
                .iter()
                .map(|a| win_arg(a))
                .collect::<Vec<_>>()
                .join(" ");
            let script = format!(
                "$p = Start-Process -FilePath {} -ArgumentList {} -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode",
                ps_quote(&cli),
                ps_quote(&arglist),
            );
            Plan {
                program: "powershell.exe".into(),
                args: vec![
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    script,
                ],
            }
        }
    }
}

/// Lance le plan et attend la fin. `Err` porte un message lisible, y compris
/// quand l'utilisateur a refusé l'autorisation.
pub fn run(plan: &Plan) -> Result<(), String> {
    let mut cmd = Command::new(&plan.program);
    cmd.args(&plan.args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Pas de fenêtre de console qui clignote pendant la demande.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("{} : {e}", plan.program))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let msg = err.trim();
    Err(if msg.is_empty() {
        format!("code de sortie {}", out.status.code().unwrap_or(-1))
    } else {
        msg.lines().last().unwrap_or(msg).to_owned()
    })
}

/// Vrai si le message d'échec signifie « l'utilisateur a refusé » : ce n'est
/// pas une erreur à afficher comme telle.
pub fn is_cancel(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("-128")              // macOS : User canceled.
        || m.contains("canceled")
        || m.contains("cancelled")
        || m.contains("annul")       // libellés localisés
        || m.contains("code de sortie 126") // pkexec : refus ou fenêtre fermée
        || m.contains("dismissed")         // pkexec : « Request dismissed »
        || m.contains("not authorized")
}

/// Où trouver la ligne de commande `brocolis`.
///
/// D'abord à côté de l'application (archive Windows, paquets Linux, et
/// `Brocolis.app/Contents/MacOS` où elle est embarquée), puis dans le `PATH`,
/// puis à l'emplacement d'installation habituel.
pub fn find_cli() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "brocolis.exe"
    } else {
        "brocolis"
    };
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join(name)));
    let in_path = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(name))
            .find(|p| p.is_file())
    });
    beside
        .filter(|p| p.is_file())
        .or(in_path)
        .or_else(|| Some(PathBuf::from("/usr/local/bin/brocolis")).filter(|p| p.is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn macos_cite_pour_le_shell_puis_pour_applescript() {
        let p = plan(
            Os::Mac,
            Path::new("/Applications/Brocolis.app/Contents/MacOS/brocolis"),
            &a(&[
                "on",
                "--config",
                "/Users/jean/Library/Application Support/Brocolis/brocolis.toml",
            ]),
        );
        assert_eq!(p.program, "osascript");
        assert_eq!(
            p.args[1],
            "do shell script \"'/Applications/Brocolis.app/Contents/MacOS/brocolis' 'on' '--config' \
             '/Users/jean/Library/Application Support/Brocolis/brocolis.toml'\" with administrator privileges"
        );
    }

    #[test]
    fn macos_neutralise_apostrophes_et_guillemets() {
        let p = plan(Os::Mac, Path::new("/x/brocolis"), &a(&["l'été \"chaud\""]));
        // Shell : 'l'\''été "chaud"' ; puis AppleScript échappe \ et ".
        assert!(
            p.args[1].contains(r#"'l'\\''été \"chaud\"'"#),
            "{}",
            p.args[1]
        );
    }

    #[test]
    fn linux_passe_par_pkexec_sans_shell() {
        let p = plan(
            Os::Linux,
            Path::new("/usr/local/bin/brocolis"),
            &a(&["off"]),
        );
        assert_eq!(p.program, "pkexec");
        assert_eq!(p.args, a(&["/usr/local/bin/brocolis", "off"]));
    }

    #[test]
    fn windows_cite_les_chemins_avec_espaces() {
        let p = plan(
            Os::Windows,
            Path::new(r"C:\Program Files\Brocolis\brocolis.exe"),
            &a(&[
                "on",
                "--config",
                r"C:\Users\Jean Dupont\AppData\Roaming\Brocolis\brocolis.toml",
            ]),
        );
        assert_eq!(p.program, "powershell.exe");
        assert_eq!(
            p.args[3],
            "$p = Start-Process -FilePath 'C:\\Program Files\\Brocolis\\brocolis.exe' \
             -ArgumentList 'on --config \"C:\\Users\\Jean Dupont\\AppData\\Roaming\\Brocolis\\brocolis.toml\"' \
             -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode"
        );
    }

    #[test]
    fn windows_apostrophe_doublee_pour_powershell() {
        let p = plan(
            Os::Windows,
            Path::new(r"C:\O'Brien\brocolis.exe"),
            &a(&["off"]),
        );
        assert!(
            p.args[3].contains(r"-FilePath 'C:\O''Brien\brocolis.exe'"),
            "{}",
            p.args[3]
        );
    }

    #[test]
    fn regles_de_citation_windows() {
        assert_eq!(win_arg("on"), "on");
        assert_eq!(win_arg(""), "\"\"");
        assert_eq!(win_arg(r"a b"), "\"a b\"");
        assert_eq!(win_arg(r#"a"b"#), r#""a\"b""#);
        // Une barre finale doit être doublée, sinon elle échapperait le
        // guillemet fermant.
        assert_eq!(win_arg(r"C:\a b\"), r#""C:\a b\\""#);
    }

    #[test]
    fn refus_reconnus() {
        assert!(is_cancel("execution error: User canceled. (-128)"));
        assert!(is_cancel("code de sortie 126"));
        assert!(is_cancel(
            "Error executing command as another user: Request dismissed"
        ));
        assert!(is_cancel(
            "Start-Process : This command cannot be run due to the error: The operation was canceled by the user."
        ));
        assert!(!is_cancel("certutil -addstore Root a échoué"));
    }
}
