//! Où l'application range ses données, et leur création au premier lancement.
//!
//! Chaque utilisateur a son propre dossier : configuration, autorité de
//! certification et identité d'appairage. Rien n'exige les droits
//! administrateur ici — seule l'activation de la protection les demande.
//!
//! | OS      | dossier                                         |
//! |---------|-------------------------------------------------|
//! | macOS   | `~/Library/Application Support/Brocolis`        |
//! | Linux   | `$XDG_CONFIG_HOME/brocolis` (`~/.config/brocolis`) |
//! | Windows | `%APPDATA%\Brocolis`                            |
//!
//! `BROCOLIS_APP_DIR` remplace ce choix : les tests s'en servent pour
//! travailler dans un dossier jetable, et on peut s'en servir pour une
//! installation portable.

use std::path::{Path, PathBuf};

use brocolis::SymKey;

/// Les fichiers de l'application.
#[derive(Clone, Debug)]
pub struct Files {
    pub dir: PathBuf,
    pub config: PathBuf,
    pub ca_cert: PathBuf,
    pub ca_key: PathBuf,
    pub identity: PathBuf,
}

impl Files {
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            config: dir.join("brocolis.toml"),
            ca_cert: dir.join("brocolis-ca.crt"),
            ca_key: dir.join("brocolis-ca.key"),
            identity: dir.join("brocolis-x25519.key"),
            dir,
        }
    }

    /// Le dossier de l'utilisateur courant (voir l'en-tête du module).
    pub fn for_user() -> Self {
        Self::at(user_dir())
    }

    /// Crée le dossier et, s'il n'existe pas, un `brocolis.toml` avec une clé
    /// neuve : l'application est utilisable dès le premier lancement. Une
    /// configuration existante n'est jamais réécrite.
    pub fn ensure(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        if !self.config.exists() {
            std::fs::write(&self.config, default_config(self))?;
        }
        Ok(())
    }
}

fn user_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("BROCOLIS_APP_DIR") {
        return PathBuf::from(dir);
    }
    let home = || {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    #[cfg(target_os = "macos")]
    {
        home().join("Library/Application Support/Brocolis")
    }
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join("AppData").join("Roaming"))
            .join("Brocolis")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"))
            .join("brocolis")
    }
}

/// Configuration initiale. Les chemins du CA sont ABSOLUS : `brocolis on`,
/// lancé avec les droits administrateur depuis un autre dossier courant, doit
/// retrouver exactement le CA que le proxy de l'application utilise.
fn default_config(files: &Files) -> String {
    let path = |p: &Path| p.display().to_string().replace('\\', "\\\\");
    format!(
        "# Configuration de l'application Brocolis.\n\
         # Même format que pour la ligne de commande (voir brocolis.example.toml).\n\
         listen = \"127.0.0.1:8080\"\n\
         ca_cert = \"{}\"\n\
         ca_key = \"{}\"\n\
         # Clé principale : c'est elle qui chiffre. Partagez-la par un canal sûr.\n\
         key = \"{}\"\n\
         keys = []\n\
         encrypt_hosts = []\n",
        path(&files.ca_cert),
        path(&files.ca_key),
        SymKey::generate().to_base64(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_configuration_initiale_est_valide_et_complete() {
        let dir = std::env::temp_dir().join(format!("brocolis-app-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let files = Files::at(&dir);
        files.ensure().unwrap();

        let cfg = brocolis::Config::load(&files.config).unwrap();
        assert_eq!(cfg.ca_cert, files.ca_cert, "chemin du CA absolu, identique");
        assert_eq!(cfg.all_key_b64().len(), 1, "une clé prête à l'emploi");

        // Un second appel ne touche pas à ce qui existe.
        let avant = std::fs::read_to_string(&files.config).unwrap();
        files.ensure().unwrap();
        assert_eq!(avant, std::fs::read_to_string(&files.config).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
