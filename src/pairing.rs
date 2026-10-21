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
//! POST /pair {code, peerId, pub, bundle?, caps?} -> { peers: [{peerId, pub, bundle?, caps?}] }
//! GET  /pair?code=..&peerId=..                   -> { peers: [...] }
//! POST /wrap {code, fromPeerId, fromPub, items}  -> { ok, count }
//! GET  /wrap?code=..&peerId=..                   -> { wrap: {from, fromPub, blob} | null }
//! ```
//!
//! `bundle` est le paquet de clés publiques v3 (§8.4 de la sécurité de la
//! messagerie) : publié à chaque publication, vérifié par le serveur, rendu
//! avec le participant. Un ancien client l'ignore ; s'il manque d'un côté,
//! l'appairage reste en ancien format (et l'empreinte aussi).
//!
//! `caps` est la capacité v4 signée (§9.6) : avec elle des deux côtés, le
//! contact passe au triple cliquet et l'empreinte prend le contexte v4
//! (§9.7) — un serveur qui la retirerait d'un côté se trahirait.

use std::time::Duration;

use rand::Rng;
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
    /// Son paquet de clés v3 (base64), s'il en a publié un.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<String>,
    /// Sa capacité v4 signée (base64), s'il en a publié une.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caps: Option<String>,
}

/// Ce que l'on publie au salon en plus de sa clé publique : le paquet v3
/// (§8.4) et la capacité v4 (§9.6), en base64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    pub bundle: String,
    pub caps: Option<String>,
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
        mine: Option<&Published>,
    ) -> Result<Vec<PairingPeer>, PairingError> {
        let mut body = serde_json::json!({ "code": code, "peerId": peer_id, "pub": public_key });
        if let Some(m) = mine {
            body["bundle"] = serde_json::Value::from(m.bundle.as_str());
            if let Some(c) = &m.caps {
                body["caps"] = serde_json::Value::from(c.as_str());
            }
        }
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
    rand::rng().fill_bytes(&mut b);
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

/// Le résultat d'un appairage à deux.
pub struct Paired {
    /// La clé partagée (base64).
    pub key_b64: String,
    /// L'empreinte à comparer de vive voix : celle du v3 (elle couvre la clé
    /// et les deux identités) si les deux côtés ont publié un paquet, celle
    /// du mode compatible sinon.
    pub fingerprint: String,
    /// Le paquet v3 du correspondant (base64), seulement si les deux côtés en
    /// ont publié un : c'est ce qui fait passer le contact en double cliquet.
    pub bundle: Option<String>,
    /// Sa capacité v4 (base64), seulement si les deux côtés en ont publié
    /// une valide : le contact passe alors au triple cliquet.
    pub caps: Option<String>,
}

/// Une capacité v4 (base64) valide pour ce paquet (§9.6).
pub fn caps_of(bundle: &brocolis_proto::KeyBundle, caps: Option<&str>) -> Option<[u8; 64]> {
    use base64::Engine as _;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(caps?.trim())
        .ok()?;
    let raw: [u8; 64] = raw.try_into().ok()?;
    crate::v3::caps_valid(bundle, Some(&raw)).then_some(raw)
}

/// L'empreinte d'un appairage à deux avec les deux paquets (§8.4) :
/// `SHA-256("Brocolis_v3_Appairage" ‖ K ‖ min(P_A, P_B) ‖ max(P_A, P_B))[0..8]`,
/// sous `"Brocolis_v4_Appairage"` quand les deux capacités v4 sont valides
/// (§9.7). Renvoie aussi si c'est le cas. `None` si une clé ou un paquet est
/// illisible.
pub fn duo_fingerprint(
    key_b64: &str,
    mine: &Published,
    theirs: &str,
    theirs_caps: Option<&str>,
) -> Option<(String, bool)> {
    let key = crate::crypto::SymKey::from_base64(key_b64).ok()?.bytes();
    let a = brocolis_proto::KeyBundle::from_base64(mine.bundle.trim()).ok()?;
    let b = brocolis_proto::KeyBundle::from_base64(theirs.trim()).ok()?;
    let v4 = caps_of(&a, mine.caps.as_deref()).is_some() && caps_of(&b, theirs_caps).is_some();
    let (ia, ib) = (crate::v3::identity_of(&a), crate::v3::identity_of(&b));
    Some(if v4 {
        (crate::v3::pairing_fingerprint4(&key, &ia, &ib), true)
    } else {
        (crate::v3::pairing_fingerprint(&key, &ia, &ib), false)
    })
}

/// Appairage à deux (le « mode compatible » de la messagerie) : publie la clé
/// publique de `identity` sous `code` — et le paquet v3 `bundle` s'il est
/// donné —, attend qu'un correspondant en fasse autant (au plus `timeout`),
/// puis dérive la clé partagée avec le premier arrivé.
///
/// Les deux côtés obtiennent la même clé et la même empreinte ; l'empreinte
/// doit être comparée de vive voix avant tout secret (elle trahirait un
/// serveur qui substituerait les clés publiques ou les paquets).
pub async fn pair_two(
    client: &RendezvousClient,
    code: &str,
    identity: &crate::agreement::PrivateKey,
    mine: Option<&Published>,
    timeout: Duration,
) -> anyhow::Result<Paired> {
    let peer_id = random_peer_id();
    let my_pub = identity.public_key_base64();
    tracing::info!("Publication de la clé publique (code « {code} »)…");
    let mut peers = client.publish(code, &peer_id, &my_pub, mine).await?;

    let deadline = tokio::time::Instant::now() + timeout;
    while peers.is_empty() {
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "aucun correspondant n'a rejoint le code « {code} » dans le délai imparti"
            );
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        peers = client.fetch(code, &peer_id).await?;
    }

    let peer = &peers[0];
    let derived = identity
        .derive(&peer.public_key)
        .map_err(|e| anyhow::anyhow!("dérivation de la clé partagée: {e}"))?;
    // Les deux paquets, ou l'ancien format des deux côtés : un paquet
    // illisible compte comme absent (l'autre côté, qui le voit valide,
    // afficherait une autre empreinte — c'est le signal voulu).
    let v3 = mine.zip(peer.bundle.as_deref()).and_then(|(mine, theirs)| {
        duo_fingerprint(
            &derived.symmetric_key_base64,
            mine,
            theirs,
            peer.caps.as_deref(),
        )
        .map(|f| (f, theirs))
    });
    Ok(match v3 {
        Some(((fingerprint, v4), theirs)) => Paired {
            key_b64: derived.symmetric_key_base64,
            fingerprint,
            bundle: Some(theirs.trim().to_owned()),
            caps: peer
                .caps
                .as_deref()
                .filter(|_| v4)
                .map(|c| c.trim().to_owned()),
        },
        None => Paired {
            key_b64: derived.symmetric_key_base64,
            fingerprint: derived.fingerprint,
            bundle: None,
            caps: None,
        },
    })
}

/// Retire une clé du trousseau d'un document TOML : du champ `key` (clé
/// principale) comme du tableau `keys`. Commentaires et mise en forme sont
/// préservés. Renvoie le TOML mis à jour.
///
/// Retirer la clé principale ne promeut pas une autre clé à sa place : choisir
/// avec quelle clé chiffrer reste une décision de l'utilisateur.
pub fn remove_key_from_toml(existing: &str, key_b64: &str) -> Result<String, String> {
    let mut doc = existing
        .parse::<DocumentMut>()
        .map_err(|e| format!("TOML invalide : {e}"))?;
    if doc.get("key").and_then(|v| v.as_str()) == Some(key_b64) {
        doc.remove("key");
    }
    if let Some(arr) = doc.get_mut("keys").and_then(|v| v.as_array_mut()) {
        arr.retain(|v| v.as_str() != Some(key_b64));
    }
    Ok(doc.to_string())
}

/// Fait d'une clé la clé principale (champ `key`, celle qui chiffre). L'ancienne
/// clé principale rejoint le tableau `keys` : elle reste utilisable pour LIRE
/// ce qu'on a reçu avec elle.
pub fn set_primary_key_in_toml(existing: &str, key_b64: &str) -> Result<String, String> {
    let mut doc = existing
        .parse::<DocumentMut>()
        .map_err(|e| format!("TOML invalide : {e}"))?;
    let previous = doc.get("key").and_then(|v| v.as_str()).map(str::to_owned);
    if previous.as_deref() == Some(key_b64) {
        return Ok(doc.to_string());
    }
    doc["key"] = toml_edit::value(key_b64);
    let mut out = remove_from_keys(doc, key_b64);
    if let Some(prev) = previous {
        out = append_key_to_toml(&out, &prev)?;
    }
    Ok(out)
}

fn remove_from_keys(mut doc: DocumentMut, key_b64: &str) -> String {
    if let Some(arr) = doc.get_mut("keys").and_then(|v| v.as_array_mut()) {
        arr.retain(|v| v.as_str() != Some(key_b64));
    }
    doc.to_string()
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
    fn peer_bundle_is_optional() {
        let json = r#"{"peers":[{"peerId":"a","pub":"P","bundle":"B"},{"peerId":"b","pub":"Q"}]}"#;
        let resp: PairingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.peers[0].bundle.as_deref(), Some("B"));
        assert!(resp.peers[1].bundle.is_none());
        assert!(resp.peers[0].caps.is_none());
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
    fn remove_key_from_primary_and_array() {
        let toml = "# garde\nlisten = \"127.0.0.1:8080\"\nkey = \"AAA\"\nkeys = [\"BBB\", \"AAA\", \"CCC\"]\n";
        let out = remove_key_from_toml(toml, "AAA").unwrap();
        assert!(
            out.contains("# garde"),
            "commentaire des autres champs préservé"
        );
        let parsed: toml::Value = toml::from_str(&out).unwrap();
        assert!(parsed.get("key").is_none(), "clé principale retirée");
        let keys: Vec<_> = parsed["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(keys, ["BBB", "CCC"]);
    }

    #[test]
    fn set_primary_moves_previous_to_keys() {
        let toml = "key = \"AAA\"\nkeys = [\"BBB\"]\n";
        let out = set_primary_key_in_toml(toml, "BBB").unwrap();
        let parsed: toml::Value = toml::from_str(&out).unwrap();
        assert_eq!(parsed["key"].as_str(), Some("BBB"));
        let keys: Vec<_> = parsed["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            keys,
            ["AAA"],
            "l'ancienne principale reste lisible, la nouvelle n'est pas en double"
        );
    }

    #[test]
    fn set_primary_on_empty_document() {
        let out = set_primary_key_in_toml("", "ZZZ").unwrap();
        let parsed: toml::Value = toml::from_str(&out).unwrap();
        assert_eq!(parsed["key"].as_str(), Some("ZZZ"));
    }

    #[test]
    fn append_key_preserves_comments() {
        let toml = "# mon commentaire\nlisten = \"127.0.0.1:8080\"\n";
        let out = append_key_to_toml(toml, "ZZZ").unwrap();
        assert!(out.contains("# mon commentaire"));
    }
}
