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
//!
//! # Un contact, une clé : celle que l'on partage avec cette personne.
//! [[contacts]]
//! name = "Alice"
//! key = "<clé partagée avec Alice>"
//! ```

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Un correspondant et LA clé partagée avec lui : elle chiffre ce qu'on lui
/// envoie et ouvre ce qu'il nous envoie.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    pub name: String,
    pub key: String,
    /// Le paquet de clés v3 du correspondant (base64), reçu à l'appairage :
    /// avec lui, ce qu'on lui envoie part en double cliquet (protocole v3 de
    /// la messagerie). Public par nature, signé par son identité.
    #[serde(default)]
    pub bundle: Option<String>,
    /// Quand ce paquet est arrivé (secondes Unix) : il sert à ouvrir une
    /// session pendant 30 jours.
    #[serde(default)]
    pub bundle_at: Option<u64>,
    /// Sa capacité v4 signée (base64, §9.6), reçue avec le paquet : avec
    /// elle, ce qu'on lui envoie part en triple cliquet (protocole v4).
    #[serde(default)]
    pub caps: Option<String>,
    /// L'identité (IK X25519 ‖ IK Ed25519, base64) qu'avait ce contact avant
    /// un ré-appairage qui en a donné une autre (§9.8 de la sécurité de la
    /// messagerie).
    #[serde(default)]
    pub previous_identity: Option<String>,
    /// Son identité a changé et l'utilisateur ne l'a pas encore confirmée :
    /// l'envoi vers lui est bloqué (la lecture, non).
    #[serde(default)]
    pub identity_changed: bool,
    /// `false` : la clé vient d'un appairage de groupe (jamais de v3).
    #[serde(default)]
    pub duo: Option<bool>,
    /// « À deux, double cliquet » : le v3 à partir de la clé partagée (PSK),
    /// quand l'autre est à jour mais qu'aucun paquet n'a été échangé.
    #[serde(default)]
    pub psk: bool,
}

impl Contact {
    /// La clé du contact, sans les espaces autour ; `None` si elle est vide.
    pub fn key(&self) -> Option<&str> {
        Some(self.key.trim()).filter(|k| !k.is_empty())
    }
}

/// Un groupe à cliquet : chacun y écrit avec sa propre chaîne (Sender Keys),
/// remise aux autres par leur session à deux.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub name: String,
    /// L'identifiant du groupe (16 octets, base64), le même chez chaque
    /// membre.
    pub id: String,
    /// Les autres membres : des noms de contacts.
    pub members: Vec<String>,
}

impl Group {
    /// L'identifiant en octets ; `None` s'il est mal formé.
    pub fn id_bytes(&self) -> Option<[u8; 16]> {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(self.id.trim())
            .ok()?
            .try_into()
            .ok()
    }
}

/// Deux noms désignent-ils le même contact ? Sans tenir compte de la casse ni
/// des espaces autour : « alice » et « Alice » sont la même personne.
pub fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

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
    /// Clé symétrique partagée primaire (base64, 32 octets). Si absente, une clé
    /// éphémère est générée au démarrage. Sert au chiffrement par défaut.
    #[serde(default)]
    pub key: Option<String>,
    /// Clés supplémentaires (base64, 32 octets) — typiquement une par
    /// correspondant. Ajoutées au trousseau en plus de `key`. Au déchiffrement,
    /// brocolis essaie TOUTES les clés : on lit ce que chacun nous envoie sans
    /// jamais changer de clé active.
    #[serde(default)]
    pub keys: Vec<String>,
    /// Si vrai, chaque `((secret))` est chiffré pour TOUTES les clés du
    /// trousseau (jeton `v2` multi-destinataire, lisible par tous les
    /// correspondants). Défaut : faux (jeton `v1`, compatible proxy/iOS).
    #[serde(default)]
    pub encrypt_multi: bool,
    /// Correspondants nommés, une clé chacun (`[[contacts]]`).
    #[serde(default)]
    pub contacts: Vec<Contact>,
    /// Groupes à cliquet (`[[groups]]`, §9.9 de la sécurité de la
    /// messagerie) : des contacts appairés, et un identifiant commun.
    #[serde(default)]
    pub groups: Vec<Group>,
    /// Pour qui le proxy chiffre : des noms de contacts. Vide = la clé `key`
    /// (ou tout le trousseau si `encrypt_multi`). Plusieurs noms = un seul
    /// jeton `v2` que chacun d'eux ouvre avec sa clé.
    #[serde(default)]
    pub encrypt_to: Vec<String>,
    /// Hôtes dont le trafic doit être traité (sous-chaîne du nom d'hôte).
    /// Vide = tous les hôtes.
    #[serde(default)]
    pub encrypt_hosts: Vec<String>,
}

impl Config {
    /// Toutes les clés base64 du trousseau, dans l'ordre : `key` (primaire)
    /// d'abord, puis `keys`. Les chaînes vides sont ignorées.
    ///
    /// Les clés des contacts suivent : au déchiffrement, le proxy lit ainsi ce
    /// que chacun nous envoie. Une même clé présente deux fois n'est gardée
    /// qu'une fois.
    pub fn all_key_b64(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let contact_keys = self.contacts.iter().map(|c| &c.key);
        for k in self.key.iter().chain(self.keys.iter()).chain(contact_keys) {
            let k = k.trim();
            if !k.is_empty() && !out.iter().any(|o| o == k) {
                out.push(k.to_string());
            }
        }
        out
    }

    /// Le groupe de ce nom (casse indifférente).
    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups.iter().find(|g| same_name(&g.name, name))
    }

    /// Le contact de ce nom (casse indifférente).
    pub fn contact(&self, name: &str) -> Option<&Contact> {
        self.contacts.iter().find(|c| same_name(&c.name, name))
    }

    /// Clés avec lesquelles CHIFFRER quand `encrypt_to` désigne des contacts :
    /// la clé de chacun. `Ok(None)` si `encrypt_to` est vide (règle
    /// par défaut). Un nom inconnu, ou un contact sans clé, est une erreur :
    /// chiffrer pour quelqu'un d'autre que prévu, ou ne pas chiffrer du tout,
    /// serait pire que de refuser de démarrer.
    pub fn encryption_targets(&self) -> anyhow::Result<Option<Vec<String>>> {
        if self.encrypt_to.is_empty() {
            return Ok(None);
        }
        let mut keys = Vec::new();
        for name in &self.encrypt_to {
            let contact = self
                .contact(name)
                .ok_or_else(|| anyhow::anyhow!("encrypt_to : contact « {name} » inconnu"))?;
            let key = contact.key().ok_or_else(|| {
                anyhow::anyhow!("encrypt_to : le contact « {name} » n'a aucune clé")
            })?;
            if !keys.iter().any(|k: &String| k == key) {
                keys.push(key.to_string());
            }
        }
        Ok(Some(keys))
    }
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
            keys: Vec::new(),
            encrypt_multi: false,
            contacts: Vec::new(),
            groups: Vec::new(),
            encrypt_to: Vec::new(),
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
    fn contacts_et_destinataires() {
        let toml = r#"
key = "PRINCIPALE"
keys = ["ANONYME", "ALICE"]
encrypt_to = ["alice", "Bob"]

[[contacts]]
name = "Alice"
key = "ALICE"

[[contacts]]
name = "Bob"
key = "BOB"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            cfg.all_key_b64(),
            ["PRINCIPALE", "ANONYME", "ALICE", "BOB"],
            "toutes les clés, sans doublon, contacts compris"
        );
        assert_eq!(cfg.contact("ALICE").unwrap().key(), Some("ALICE"));
        assert_eq!(
            cfg.encryption_targets().unwrap().unwrap(),
            ["ALICE", "BOB"],
            "la clé de chaque destinataire"
        );
        // Un contact a UNE clé : une liste est refusée.
        assert!(toml::from_str::<Config>("[[contacts]]\nname = \"A\"\nkeys = [\"K\"]\n").is_err());
    }

    #[test]
    fn destinataire_inconnu_ou_sans_cle_refuse() {
        let inconnu: Config = toml::from_str("encrypt_to = [\"Zoé\"]").unwrap();
        assert!(inconnu.encryption_targets().is_err());
        let vide: Config =
            toml::from_str("encrypt_to = [\"Zoé\"]\n[[contacts]]\nname = \"Zoé\"\nkey = \" \"\n")
                .unwrap();
        assert!(vide.encryption_targets().is_err());
        assert!(Config::default().encryption_targets().unwrap().is_none());
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
