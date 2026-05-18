//! Configuration au format TOML.
//!
//! Exemple de `brocolis.toml` :
//!
//! ```toml
//! listen = "127.0.0.1:8080"
//! ca_cert = "brocolis-ca.crt"
//! ca_key = "brocolis-ca.key"
//! key = "<clé base64 de 32 octets, optionnel>"
//! encrypt_hosts = ["twitter.com", "x.com"]
//! ```

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Adresse d'écoute du proxy.
    #[serde(default = "default_listen")]
    pub listen: SocketAddr,
    /// Chemin du certificat racine (CA).
    #[serde(default = "default_ca_cert")]
    pub ca_cert: PathBuf,
    /// Chemin de la clé privée du CA.
    #[serde(default = "default_ca_key")]
    pub ca_key: PathBuf,
    /// Clé symétrique partagée (base64, 32 octets). Si absente, une clé
    /// éphémère est générée au démarrage.
    #[serde(default)]
    pub key: Option<String>,
    /// Hôtes dont le trafic doit être traité (sous-chaîne du nom d'hôte).
    /// Vide = tous les hôtes.
    #[serde(default)]
    pub encrypt_hosts: Vec<String>,
}

fn default_listen() -> SocketAddr {
    "127.0.0.1:8080".parse().expect("adresse par défaut valide")
}
fn default_ca_cert() -> PathBuf {
    PathBuf::from("brocolis-ca.crt")
}
fn default_ca_key() -> PathBuf {
    PathBuf::from("brocolis-ca.key")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            ca_cert: default_ca_cert(),
            ca_key: default_ca_key(),
            key: None,
            encrypt_hosts: Vec::new(),
        }
    }
}

impl Config {
    /// Charge la configuration depuis un fichier TOML. Si le fichier n'existe
    /// pas, renvoie la configuration par défaut.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            tracing::warn!(
                "fichier de configuration {} introuvable, valeurs par défaut utilisées",
                path.display()
            );
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&raw)?;
        Ok(cfg)
    }

    /// Indique si le trafic vers `host` doit être traité par le chiffrement
    /// sélectif. Une liste vide signifie « tous les hôtes ».
    pub fn host_in_scope(&self, host: &str) -> bool {
        if self.encrypt_hosts.is_empty() {
            return true;
        }
        self.encrypt_hosts
            .iter()
            .any(|h| host == h || host.ends_with(&format!(".{h}")) || host.contains(h.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_config() {
        let toml = r#"
listen = "0.0.0.0:9090"
ca_cert = "ca.crt"
ca_key = "ca.key"
key = "abc"
encrypt_hosts = ["twitter.com", "x.com"]
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.listen, "0.0.0.0:9090".parse().unwrap());
        assert_eq!(cfg.key.as_deref(), Some("abc"));
        assert_eq!(cfg.encrypt_hosts.len(), 2);
    }

    #[test]
    fn defaults_when_fields_missing() {
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg.listen, default_listen());
        assert!(cfg.key.is_none());
        assert!(cfg.encrypt_hosts.is_empty());
    }

    #[test]
    fn host_scope_matching() {
        let cfg = Config {
            encrypt_hosts: vec!["twitter.com".into()],
            ..Config::default()
        };
        assert!(cfg.host_in_scope("twitter.com"));
        assert!(cfg.host_in_scope("api.twitter.com"));
        assert!(!cfg.host_in_scope("example.org"));

        let all = Config::default();
        assert!(all.host_in_scope("anything.test"));
    }
}
