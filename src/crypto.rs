//! Chiffrement sélectif AES-256-GCM.
//!
//! Schéma « sélectif » : seul le contenu explicitement marqué est chiffré.
//!
//! - Côté requête (ce que l'utilisateur envoie), tout segment de la forme
//!   `((texte secret))` est chiffré et remplacé par un jeton
//!   `[[BROCOLIS:v1:<base64>]]`.
//! - Côté réponse (ce que l'utilisateur reçoit), tout jeton
//!   `[[BROCOLIS:v1:<base64>]]` est déchiffré et remplacé par le texte clair.
//!
//! Le jeton encode `nonce (12 octets) || ciphertext || tag (16 octets)`. Deux
//! utilisateurs qui partagent la même clé symétrique voient ainsi le contenu en
//! clair, alors que le service distant (Twitter, forum, ...) ne stocke que le
//! jeton chiffré.
//!
//! Ce format est interopérable avec les clients iOS (CryptoKit,
//! `AES.GCM.SealedBox.combined`) et Android (`javax.crypto.Cipher`,
//! `iv || doFinal`). À l'émission, brocolis encode en base64 standard ; au
//! décodage, il accepte aussi l'URL-safe et l'absence de padding (fonction
//! `decode_payload`) pour rester compatible avec les divers encodeurs mobiles.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::{
    STANDARD as B64, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
};
use base64::Engine;
use rand::RngCore;

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
/// Taille fixe d'une clé de message emballée : nonce(12) + clé(32) + tag(16).
const WRAPPED_KEY_LEN: usize = NONCE_LEN + KEY_LEN + TAG_LEN;
/// Tête commune à toutes les versions de jeton (`v1`, `v2`).
const TOKEN_HEAD: &str = "[[BROCOLIS:";
const TOKEN_PREFIX: &str = "[[BROCOLIS:v1:";
const TOKEN_PREFIX_V2: &str = "[[BROCOLIS:v2:";
const TOKEN_SUFFIX: &str = "]]";
const PLAIN_OPEN: &str = "((";
const PLAIN_CLOSE: &str = "))";

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("la clé doit faire {KEY_LEN} octets, reçu {0}")]
    BadKeyLen(usize),
    #[error("clé base64 invalide: {0}")]
    BadKeyB64(String),
    #[error("échec du (dé)chiffrement AES-256-GCM")]
    Aead,
    #[error("jeton chiffré corrompu")]
    BadToken,
}

/// Clé symétrique AES-256 (32 octets) conservée en mémoire.
#[derive(Clone)]
pub struct SymKey {
    bytes: [u8; KEY_LEN],
}

impl SymKey {
    /// Génère une nouvelle clé aléatoire (CSPRNG).
    pub fn generate() -> Self {
        let mut bytes = [0u8; KEY_LEN];
        rand::rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn from_base64(s: &str) -> Result<Self, CryptoError> {
        let raw = B64
            .decode(s.trim())
            .map_err(|e| CryptoError::BadKeyB64(e.to_string()))?;
        Self::from_bytes(&raw)
    }

    /// Construit une clé à partir d'octets bruts (exactement [`KEY_LEN`]).
    pub fn from_bytes(raw: &[u8]) -> Result<Self, CryptoError> {
        let len = raw.len();
        let bytes: [u8; KEY_LEN] = raw.try_into().map_err(|_| CryptoError::BadKeyLen(len))?;
        Ok(Self { bytes })
    }

    pub fn to_base64(&self) -> String {
        B64.encode(self.bytes)
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.bytes))
    }

    /// Scelle `plaintext` en AES-256-GCM et renvoie l'empaquetage « combined »
    /// `nonce (12) || ciphertext || tag (16)` — identique à
    /// `AES.GCM.SealedBox.combined` de CryptoKit. Brique partagée par les jetons
    /// v1, l'enveloppe v2 et l'emballage des clés de groupe X25519.
    pub fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = self
            .cipher()
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::Aead)?;
        let mut packed = Vec::with_capacity(NONCE_LEN + ct.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&ct);
        Ok(packed)
    }

    /// Ouvre un empaquetage « combined » `nonce || ciphertext || tag`.
    pub fn open(&self, packed: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if packed.len() < NONCE_LEN + TAG_LEN {
            return Err(CryptoError::BadToken);
        }
        let (nonce_bytes, ct) = packed.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher()
            .decrypt(nonce, ct)
            .map_err(|_| CryptoError::Aead)
    }

    /// Chiffre `plaintext` et renvoie un jeton `[[BROCOLIS:v1:<base64>]]`.
    pub fn encrypt_token(&self, plaintext: &[u8]) -> Result<String, CryptoError> {
        let packed = self.seal(plaintext)?;
        Ok(format!(
            "{TOKEN_PREFIX}{}{TOKEN_SUFFIX}",
            B64.encode(packed)
        ))
    }

    /// Déchiffre la partie base64 d'un jeton v1 (sans préfixe/suffixe).
    fn decrypt_b64(&self, b64: &str) -> Result<Vec<u8>, CryptoError> {
        let packed = decode_payload(b64).ok_or(CryptoError::BadToken)?;
        self.open(&packed)
    }
}

// ---------------------------------------------------------------------------
// Jeton multi-destinataire v2 (chiffrement d'enveloppe)
// ---------------------------------------------------------------------------
//
// Aligné octet pour octet sur `BrocolisCrypto.swift`. Le texte est chiffré UNE
// seule fois sous une « clé de message » aléatoire, elle-même emballée sous
// CHAQUE clé destinataire. N'importe quel destinataire ouvre l'emballage qui
// lui correspond, récupère la clé de message et lit le texte. Un seul jeton,
// tout le monde lit en clair.
//
// Disposition du payload (puis base64) :
//   [0]                    format = 0x01
//   [1]                    N = nombre de destinataires (1..=255)
//   [2 .. 2+60N)           N clés de message emballées (60 octets chacune)
//   [2+60N .. fin)         corps = AES-GCM(texte, cléDeMessage) « combined »

/// Chiffre `plaintext` pour PLUSIEURS clés en un seul jeton `v2`.
pub fn encrypt_token_multi(plaintext: &[u8], keys: &[SymKey]) -> Result<String, CryptoError> {
    if keys.is_empty() || keys.len() > 255 {
        return Err(CryptoError::Aead);
    }
    let message_key = SymKey::generate();
    let body = message_key.seal(plaintext)?;

    let mut payload = Vec::with_capacity(2 + keys.len() * WRAPPED_KEY_LEN + body.len());
    payload.push(0x01);
    payload.push(keys.len() as u8);
    for key in keys {
        let wrapped = key.seal(&message_key.bytes)?;
        if wrapped.len() != WRAPPED_KEY_LEN {
            return Err(CryptoError::Aead);
        }
        payload.extend_from_slice(&wrapped);
    }
    payload.extend_from_slice(&body);
    Ok(format!(
        "{TOKEN_PREFIX_V2}{}{TOKEN_SUFFIX}",
        B64.encode(payload)
    ))
}

/// Tente d'ouvrir un jeton `v2` avec un jeu de clés. Renvoie `None` si aucune
/// des clés ne correspond (message « pour les autres »).
fn decrypt_token_v2(b64: &str, keys: &[SymKey]) -> Option<Vec<u8>> {
    let payload = decode_payload(b64)?;
    if payload.len() < 2 || payload[0] != 0x01 {
        return None;
    }
    let n = payload[1] as usize;
    let wraps_start = 2;
    let body_start = wraps_start + n * WRAPPED_KEY_LEN;
    if payload.len() < body_start + NONCE_LEN + TAG_LEN {
        return None;
    }
    let body = &payload[body_start..];
    for i in 0..n {
        let s = wraps_start + i * WRAPPED_KEY_LEN;
        let wrapped = &payload[s..s + WRAPPED_KEY_LEN];
        for key in keys {
            let Ok(message_key_raw) = key.open(wrapped) else {
                continue;
            };
            let Ok(message_key) = SymKey::from_bytes(&message_key_raw) else {
                continue;
            };
            if let Ok(plain) = message_key.open(body) {
                return Some(plain);
            }
        }
    }
    None
}

/// Scelle `plaintext` dans un jeton : `v1` pour une seule clé (compatible
/// proxy/iOS), `v2` (enveloppe) dès qu'il y a plusieurs destinataires.
pub fn seal_token(plaintext: &[u8], keys: &[SymKey]) -> Option<String> {
    match keys {
        [] => None,
        [only] => only.encrypt_token(plaintext).ok(),
        many => encrypt_token_multi(plaintext, many).ok(),
    }
}

/// Décode la charge base64 d'un jeton en tolérant les variantes que peuvent
/// émettre les clients tiers (iOS, Android, Web).
///
/// brocolis **émet** toujours du base64 standard avec padding (cf.
/// [`SymKey::encrypt_token`]), mais **accepte** en entrée :
/// l'alphabet standard (`+`/`/`) comme l'alphabet URL-safe (`-`/`_`), avec
/// ou sans padding `=`. Cela maximise l'interopérabilité : un jeton produit
/// par un client Android via `Base64.URL_SAFE`, ou ayant transité par un
/// service qui échappe `+`/`/`/`=`, reste déchiffrable.
fn decode_payload(b64: &str) -> Option<Vec<u8>> {
    // Ordre : standard d'abord (forme émise par brocolis et l'app iOS), puis
    // les variantes. Les moteurs sans padding rejettent le `=`, et
    // inversement, donc l'essai successif couvre les quatre combinaisons.
    for engine in [B64, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD] {
        if let Ok(bytes) = engine.decode(b64) {
            return Some(bytes);
        }
    }
    None
}

/// Remplace chaque `((secret))` par un jeton chiffré.
///
/// Renvoie `(texte_transformé, nb_segments_chiffrés)`. En cas d'échec de
/// chiffrement d'un segment, celui-ci est laissé tel quel (best effort).
pub fn encrypt_marked(input: &str, key: &SymKey) -> (String, usize) {
    encrypt_marked_multi(input, std::slice::from_ref(key))
}

/// Variante multi-destinataire : chaque `((segment))` est scellé pour TOUTES
/// les `keys` (jeton `v2` d'enveloppe si plusieurs, `v1` si une seule). Permet
/// d'écrire un message lisible par plusieurs correspondants sans rien changer.
pub fn encrypt_marked_multi(input: &str, keys: &[SymKey]) -> (String, usize) {
    let mut out = String::with_capacity(input.len());
    let mut count = 0usize;
    let mut rest = input;
    while let Some(open) = rest.find(PLAIN_OPEN) {
        // Cherche la fermeture après l'ouverture.
        let after_open = &rest[open + PLAIN_OPEN.len()..];
        let close = match after_open.find(PLAIN_CLOSE) {
            Some(c) => c,
            None => break,
        };
        let secret = &after_open[..close];
        out.push_str(&rest[..open]);
        match seal_token(secret.as_bytes(), keys) {
            Some(token) => {
                out.push_str(&token);
                count += 1;
            }
            None => {
                // Laisse le marqueur intact si le chiffrement échoue.
                out.push_str(PLAIN_OPEN);
                out.push_str(secret);
                out.push_str(PLAIN_CLOSE);
            }
        }
        rest = &after_open[close + PLAIN_CLOSE.len()..];
    }
    out.push_str(rest);
    (out, count)
}

/// Remplace chaque jeton `[[BROCOLIS:v1:...]]` par son texte clair.
///
/// Renvoie `(texte_transformé, nb_segments_déchiffrés)`. Un jeton illisible
/// (mauvaise clé, corruption) est laissé tel quel.
pub fn decrypt_tokens(input: &str, key: &SymKey) -> (String, usize) {
    decrypt_tokens_multi(input, std::slice::from_ref(key))
}

/// Déchiffre chaque jeton `[[BROCOLIS:…]]` (v1 ou v2) en essayant TOUTES les
/// `keys` : pour chaque jeton, la première clé qui réussit gagne. Un jeton
/// qu'aucune clé n'ouvre est laissé tel quel (message « pour les autres »).
///
/// C'est le cœur de l'alignement sur la messagerie : avec une clé par
/// correspondant dans le trousseau, on lit ce que chacun nous envoie sans
/// jamais changer de clé active.
pub fn decrypt_tokens_multi(input: &str, keys: &[SymKey]) -> (String, usize) {
    let mut out = String::with_capacity(input.len());
    let mut count = 0usize;
    let mut rest = input;
    // On cherche n'importe quel jeton `[[BROCOLIS:…]]` (v1 ou v2) et on
    // déchiffre selon sa version.
    while let Some(start) = rest.find(TOKEN_HEAD) {
        let after_head = &rest[start + TOKEN_HEAD.len()..];
        let end = match after_head.find(TOKEN_SUFFIX) {
            Some(e) => e,
            None => break,
        };
        let inner = &after_head[..end]; // "v1:…" / "v2:…"
        out.push_str(&rest[..start]);

        let decrypted: Option<String> = if let Some(b64) = inner.strip_prefix("v1:") {
            keys.iter().find_map(|k| {
                k.decrypt_b64(b64)
                    .ok()
                    .map(|p| String::from_utf8_lossy(&p).into_owned())
            })
        } else if let Some(b64) = inner.strip_prefix("v2:") {
            decrypt_token_v2(b64, keys).map(|p| String::from_utf8_lossy(&p).into_owned())
        } else {
            None
        };

        match decrypted {
            Some(plain) => {
                out.push_str(&plain);
                count += 1;
            }
            None => {
                // Jeton conservé tel quel si déchiffrement impossible.
                out.push_str(TOKEN_HEAD);
                out.push_str(inner);
                out.push_str(TOKEN_SUFFIX);
            }
        }
        rest = &after_head[end + TOKEN_SUFFIX.len()..];
    }
    out.push_str(rest);
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_base64_roundtrip() {
        let k = SymKey::generate();
        let s = k.to_base64();
        let k2 = SymKey::from_base64(&s).unwrap();
        assert_eq!(k.bytes, k2.bytes);
    }

    #[test]
    fn bad_key_length_rejected() {
        let short = B64.encode([0u8; 16]);
        assert!(SymKey::from_base64(&short).is_err());
    }

    #[test]
    fn token_roundtrip() {
        let k = SymKey::generate();
        let t = k.encrypt_token(b"hello world").unwrap();
        assert!(t.starts_with(TOKEN_PREFIX) && t.ends_with(TOKEN_SUFFIX));
        let inner = &t[TOKEN_PREFIX.len()..t.len() - TOKEN_SUFFIX.len()];
        assert_eq!(k.decrypt_b64(inner).unwrap(), b"hello world");
    }

    #[test]
    fn marked_roundtrip() {
        let k = SymKey::generate();
        let src = "Coucou ((mon secret)) et ((un autre)) fin";
        let (enc, n) = encrypt_marked(src, &k);
        assert_eq!(n, 2);
        assert!(!enc.contains("mon secret"));
        assert!(enc.contains(TOKEN_PREFIX));
        let (dec, m) = decrypt_tokens(&enc, &k);
        assert_eq!(m, 2);
        assert_eq!(dec, "Coucou mon secret et un autre fin");
    }

    #[test]
    fn wrong_key_keeps_token() {
        let k1 = SymKey::generate();
        let k2 = SymKey::generate();
        let (enc, _) = encrypt_marked("x ((s)) y", &k1);
        let (dec, m) = decrypt_tokens(&enc, &k2);
        assert_eq!(m, 0);
        assert_eq!(dec, enc); // jeton conservé, pas de panique
    }

    #[test]
    fn unclosed_marker_is_noop() {
        let k = SymKey::generate();
        let (enc, n) = encrypt_marked("debut ((pas ferme", &k);
        assert_eq!(n, 0);
        assert_eq!(enc, "debut ((pas ferme");
    }

    #[test]
    fn encrypt_emits_standard_base64_with_padding() {
        // Contrat d'émission : brocolis (et l'app iOS) produisent du base64
        // STANDARD. C'est la forme que tout client tiers doit savoir lire.
        let k = SymKey::generate();
        let t = k.encrypt_token(b"hello").unwrap();
        let inner = &t[TOKEN_PREFIX.len()..t.len() - TOKEN_SUFFIX.len()];
        // 12 + 5 + 16 = 33 octets -> 44 caractères base64 (avec padding).
        assert!(B64.decode(inner).is_ok(), "doit décoder en base64 standard");
        assert!(
            !inner.contains('-') && !inner.contains('_'),
            "pas d'URL-safe"
        );
    }

    #[test]
    fn decrypt_accepts_url_safe_base64() {
        // Un client (p. ex. Android `Base64.URL_SAFE`) peut émettre la même
        // charge en alphabet URL-safe : brocolis doit la déchiffrer.
        let k = SymKey::generate();
        let std_token = k.encrypt_token("café Android".as_bytes()).unwrap();
        let inner = &std_token[TOKEN_PREFIX.len()..std_token.len() - TOKEN_SUFFIX.len()];
        let packed = B64.decode(inner).unwrap();

        let url_token = format!("{TOKEN_PREFIX}{}{TOKEN_SUFFIX}", URL_SAFE.encode(&packed));
        let (dec, n) = decrypt_tokens(&url_token, &k);
        assert_eq!(n, 1);
        assert_eq!(dec, "café Android");
    }

    #[test]
    fn decrypt_accepts_url_safe_without_padding() {
        let k = SymKey::generate();
        let std_token = k.encrypt_token(b"no padding").unwrap();
        let inner = &std_token[TOKEN_PREFIX.len()..std_token.len() - TOKEN_SUFFIX.len()];
        let packed = B64.decode(inner).unwrap();

        let url_token = format!(
            "{TOKEN_PREFIX}{}{TOKEN_SUFFIX}",
            URL_SAFE_NO_PAD.encode(&packed)
        );
        let (dec, n) = decrypt_tokens(&url_token, &k);
        assert_eq!(n, 1);
        assert_eq!(dec, "no padding");
    }

    #[test]
    fn decode_payload_covers_four_variants() {
        let packed: Vec<u8> = (0u8..40).collect();
        for encoded in [
            B64.encode(&packed),
            STANDARD_NO_PAD.encode(&packed),
            URL_SAFE.encode(&packed),
            URL_SAFE_NO_PAD.encode(&packed),
        ] {
            assert_eq!(decode_payload(&encoded).as_deref(), Some(&packed[..]));
        }
    }

    // ---- Multi-clés / jeton v2 (alignement messagerie) -------------------

    #[test]
    fn single_recipient_seals_v1() {
        // Un seul destinataire ⇒ jeton v1 (compatible proxy/iOS).
        let k = SymKey::generate();
        let token = seal_token(b"hi", std::slice::from_ref(&k)).unwrap();
        assert!(token.starts_with(TOKEN_PREFIX));
    }

    #[test]
    fn multi_recipient_seals_v2_and_each_reads() {
        // Plusieurs destinataires ⇒ jeton v2 ; CHAQUE clé lit le clair, une clé
        // étrangère ne lit rien.
        let alice = SymKey::generate();
        let bob = SymKey::generate();
        let mallory = SymKey::generate();

        let token = seal_token("rdv 18h".as_bytes(), &[alice.clone(), bob.clone()]).unwrap();
        assert!(token.starts_with(TOKEN_PREFIX_V2));

        // Alice (avec aussi des clés sans rapport) déchiffre.
        let (a, na) = decrypt_tokens_multi(&token, &[mallory.clone(), alice.clone()]);
        assert_eq!(na, 1);
        assert_eq!(a, "rdv 18h");

        // Bob déchiffre.
        let (b, nb) = decrypt_tokens_multi(&token, std::slice::from_ref(&bob));
        assert_eq!(nb, 1);
        assert_eq!(b, "rdv 18h");

        // Mallory ne déchiffre rien : jeton conservé tel quel.
        let (m, nm) = decrypt_tokens_multi(&token, std::slice::from_ref(&mallory));
        assert_eq!(nm, 0);
        assert_eq!(m, token);
    }

    #[test]
    fn v2_payload_layout_matches_swift() {
        // Disposition : [0x01, N, 60*N octets emballés, corps combined].
        let keys = [SymKey::generate(), SymKey::generate(), SymKey::generate()];
        let token = encrypt_token_multi(b"hello", &keys).unwrap();
        let inner = token
            .strip_prefix(TOKEN_PREFIX_V2)
            .and_then(|s| s.strip_suffix(TOKEN_SUFFIX))
            .unwrap();
        let payload = B64.decode(inner).unwrap();
        assert_eq!(payload[0], 0x01);
        assert_eq!(payload[1] as usize, keys.len());
        let body_start = 2 + keys.len() * WRAPPED_KEY_LEN;
        // corps = nonce(12) + ciphertext("hello"=5) + tag(16) = 33.
        assert_eq!(payload.len(), body_start + NONCE_LEN + 5 + TAG_LEN);
    }

    #[test]
    fn decrypt_tokens_multi_picks_right_key_per_token() {
        // Deux correspondants, deux jetons v1 dans le même texte : on lit les
        // deux avec le trousseau, sans changer de clé.
        let k1 = SymKey::generate();
        let k2 = SymKey::generate();
        let t1 = k1.encrypt_token("pour un".as_bytes()).unwrap();
        let t2 = k2.encrypt_token("pour deux".as_bytes()).unwrap();
        let blob = format!("a {t1} b {t2} c");
        let (out, n) = decrypt_tokens_multi(&blob, &[k1, k2]);
        assert_eq!(n, 2);
        assert_eq!(out, "a pour un b pour deux c");
    }

    #[test]
    fn encrypt_marked_multi_roundtrips_for_all() {
        let alice = SymKey::generate();
        let bob = SymKey::generate();
        let (enc, n) = encrypt_marked_multi("salut ((secret)) fin", &[alice.clone(), bob.clone()]);
        assert_eq!(n, 1);
        assert!(!enc.contains("secret"));
        for k in [&alice, &bob] {
            let (dec, m) = decrypt_tokens(&enc, k);
            assert_eq!(m, 1);
            assert_eq!(dec, "salut secret fin");
        }
    }
}
