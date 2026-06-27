//! Client du serveur de rendez-vous — appairage automatique X25519.
//!
//! Deux appareils partageant un même CODE court échangent leurs CLÉS PUBLIQUES
//! via le serveur, puis dérivent chacun la MÊME clé symétrique (cf.
//! [`crate::agreement`]). Le serveur ne voit jamais la clé secrète ni les
//! messages — seulement des clés publiques éphémères et des blobs chiffrés.
//!
//! Protocole identique à l'app iOS (`PairingClient.swift`) :
//!
//! ```text
//! POST /pair {code, peerId, pub}                 -> { peers: [{peerId, pub}] }
//! GET  /pair?code=..&peerId=..                   -> { peers: [...] }
//! POST /wrap {code, fromPeerId, fromPub, items}  -> { ok, count }
//! GET  /wrap?code=..&peerId=..                   -> { wrap: {from, fromPub, blob} | null }
//! ```

use std::time::Duration;

use rand::RngCore;
use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, Item, Value};

/// URL par défaut du serveur de rendez-vous (instance du projet, identique à
/// `BrocolisKeyStore.serverURL` côté iOS).
pub const DEFAULT_SERVER: &str = "https://brocolis-serveur-839b43057467.herokuapp.com";

#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("le serveur a répondu {0}")]
    Server(u16),
    #[error("erreur réseau : {0}")]
    Network(String),
}

/// Clé publique d'un participant (réponse du serveur).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingPeer {
    #[serde(rename = "peerId")]
    pub peer_id: String,
    #[serde(rename = "pub")]
    pub public_key: String,
}

#[derive(Debug, Deserialize)]
struct PairingResponse {
    #[serde(default)]
    peers: Vec<PairingPeer>,
}

/// Clé de groupe emballée reçue (key-wrapping X25519).
#[derive(Debug, Clone, Deserialize)]
pub struct WrapInbox {
    pub from: String,
    #[serde(rename = "fromPub")]
    pub from_pub: String,
    pub blob: String,
}

#[derive(Debug, Deserialize)]
struct WrapResponse {
    wrap: Option<WrapInbox>,
}

/// Une clé de groupe emballée à destination d'un membre.
#[derive(Debug, Clone, Serialize)]
pub struct WrapItem {
    #[serde(rename = "toPeerId")]
    pub to_peer_id: String,
    pub blob: String,
}

/// Client HTTP du serveur de rendez-vous.
pub struct RendezvousClient {
    base: String,
    http: reqwest::Client,
}

impl RendezvousClient {
    /// Construit un client pour l'URL de serveur donnée.
    pub fn new(server: &str) -> Result<Self, PairingError> {
        let base = server.trim().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| PairingError::Network(e.to_string()))?;
        Ok(Self { base, http })
    }

    /// Publie sa clé publique ; renvoie les clés publiques des AUTRES
    /// participants du code (POST /pair).
    pub async fn publish(
        &self,
        code: &str,
        peer_id: &str,
        public_key: &str,
    ) -> Result<Vec<PairingPeer>, PairingError> {
        let body = serde_json::json!({ "code": code, "peerId": peer_id, "pub": public_key });
        let resp: PairingResponse = self.post_json("/pair", &body).await?;
        Ok(resp.peers)
    }

    /// Récupère les clés publiques des autres participants (GET /pair).
    pub async fn fetch(&self, code: &str, peer_id: &str) -> Result<Vec<PairingPeer>, PairingError> {
        let path = format!("/pair?code={}&peerId={}", enc(code), enc(peer_id));
        let resp: PairingResponse = self.get_json(&path).await?;
        Ok(resp.peers)
    }

    /// (Fondateur) Dépose les clés de groupe emballées pour chaque membre
    /// (POST /wrap).
    pub async fn post_wraps(
        &self,
        code: &str,
        from_peer_id: &str,
        from_pub: &str,
        items: &[WrapItem],
    ) -> Result<(), PairingError> {
        let body = serde_json::json!({
            "code": code,
            "fromPeerId": from_peer_id,
            "fromPub": from_pub,
            "items": items,
        });
        let _: serde_json::Value = self.post_json("/wrap", &body).await?;
        Ok(())
    }

    /// (Membre) Récupère la clé de groupe emballée qui lui est destinée, si
    /// disponible (GET /wrap).
    pub async fn fetch_wrap(
        &self,
        code: &str,
        peer_id: &str,
    ) -> Result<Option<WrapInbox>, PairingError> {
        let path = format!("/wrap?code={}&peerId={}", enc(code), enc(peer_id));
        let resp: WrapResponse = self.get_json(&path).await?;
        Ok(resp.wrap)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, PairingError> {
        let resp = self
            .http
            .get(format!("{}{path}", self.base))
            .send()
            .await
            .map_err(|e| PairingError::Network(e.to_string()))?;
        parse(resp).await
    }

    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, PairingError> {
        let resp = self
            .http
            .post(format!("{}{path}", self.base))
            .json(body)
            .send()
            .await
            .map_err(|e| PairingError::Network(e.to_string()))?;
        parse(resp).await
    }
}

async fn parse<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T, PairingError> {
    let status = resp.status();
    if !status.is_success() {
        return Err(PairingError::Server(status.as_u16()));
    }
    resp.json::<T>()
        .await
        .map_err(|e| PairingError::Network(e.to_string()))
}

/// Identifiant de participant aléatoire (16 octets en hex).
pub fn random_peer_id() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Encodage minimal d'un paramètre de requête (les codes sont courts et
/// alphanumériques ; on échappe tout le reste en `%XX`).
fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Ajoute une clé base64 au tableau `keys` d'un document TOML, en créant le
/// tableau s'il n'existe pas et en préservant commentaires et mise en forme.
/// Les doublons exacts sont ignorés. Renvoie le TOML mis à jour.
pub fn append_key_to_toml(existing: &str, key_b64: &str) -> Result<String, String> {
    let mut doc = existing
        .parse::<DocumentMut>()
        .map_err(|e| format!("TOML invalide : {e}"))?;
    let arr = doc
        .entry("keys")
        .or_insert(Item::Value(Value::Array(Array::new())))
        .as_array_mut()
        .ok_or_else(|| "le champ `keys` n'est pas un tableau".to_string())?;
    if !arr.iter().any(|v| v.as_str() == Some(key_b64)) {
        arr.push(key_b64);
    }
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_deserializes_server_shape() {
        let json = r#"{"peers":[{"peerId":"abc","pub":"UFVCS0VZ"}]}"#;
        let resp: PairingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.peers.len(), 1);
        assert_eq!(resp.peers[0].peer_id, "abc");
        assert_eq!(resp.peers[0].public_key, "UFVCS0VZ");
    }

    #[test]
    fn empty_peers_when_field_absent() {
        let resp: PairingResponse = serde_json::from_str("{}").unwrap();
        assert!(resp.peers.is_empty());
    }

    #[test]
    fn wrap_response_decodes_null_and_value() {
        let none: WrapResponse = serde_json::from_str(r#"{"wrap":null}"#).unwrap();
        assert!(none.wrap.is_none());
        let some: WrapResponse =
            serde_json::from_str(r#"{"wrap":{"from":"x","fromPub":"P","blob":"B"}}"#).unwrap();
        let w = some.wrap.unwrap();
        assert_eq!(w.from, "x");
        assert_eq!(w.from_pub, "P");
        assert_eq!(w.blob, "B");
    }

    #[test]
    fn wrap_item_serializes_with_rename() {
        let item = WrapItem {
            to_peer_id: "id".into(),
            blob: "B".into(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"toPeerId\":\"id\""));
        assert!(json.contains("\"blob\":\"B\""));
    }

    #[test]
    fn new_trims_trailing_slash() {
        let c = RendezvousClient::new("https://example.test/").unwrap();
        assert_eq!(c.base, "https://example.test");
    }

    #[test]
    fn peer_id_is_32_hex_chars() {
        let id = random_peer_id();
        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn append_key_creates_array_when_absent() {
        let toml = "key = \"AAA\"\n";
        let out = append_key_to_toml(toml, "BBB").unwrap();
        assert!(out.contains("key = \"AAA\""), "champ existant préservé");
        let parsed: toml::Value = toml::from_str(&out).unwrap();
        let keys = parsed.get("keys").unwrap().as_array().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].as_str(), Some("BBB"));
    }

    #[test]
    fn append_key_extends_existing_array_and_dedupes() {
        let toml = "keys = [\"AAA\"]\n";
        let out = append_key_to_toml(toml, "BBB").unwrap();
        let out = append_key_to_toml(&out, "BBB").unwrap(); // doublon ignoré
        let parsed: toml::Value = toml::from_str(&out).unwrap();
        let keys = parsed.get("keys").unwrap().as_array().unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn append_key_preserves_comments() {
        let toml = "# mon commentaire\nlisten = \"127.0.0.1:8080\"\n";
        let out = append_key_to_toml(toml, "ZZZ").unwrap();
        assert!(out.contains("# mon commentaire"));
    }
}
