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
//! décodage, il accepte aussi l'URL-safe et l'absence de padding (cf.
//! [`decode_payload`]) pour rester compatible avec les divers encodeurs mobiles.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::{
    STANDARD as B64, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
};
use base64::Engine;
use rand::RngCore;

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TOKEN_PREFIX: &str = "[[BROCOLIS:v1:";
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
        rand::thread_rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn from_base64(s: &str) -> Result<Self, CryptoError> {
        let raw = B64
            .decode(s.trim())
            .map_err(|e| CryptoError::BadKeyB64(e.to_string()))?;
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

    /// Chiffre `plaintext` et renvoie un jeton `[[BROCOLIS:v1:<base64>]]`.
    pub fn encrypt_token(&self, plaintext: &[u8]) -> Result<String, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = self
            .cipher()
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::Aead)?;
        let mut packed = Vec::with_capacity(NONCE_LEN + ct.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&ct);
        Ok(format!("{TOKEN_PREFIX}{}{TOKEN_SUFFIX}", B64.encode(packed)))
    }

    /// Déchiffre la partie base64 d'un jeton (sans préfixe/suffixe).
    fn decrypt_b64(&self, b64: &str) -> Result<Vec<u8>, CryptoError> {
        let packed = decode_payload(b64).ok_or(CryptoError::BadToken)?;
        if packed.len() < NONCE_LEN {
            return Err(CryptoError::BadToken);
        }
        let (nonce_bytes, ct) = packed.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher()
            .decrypt(nonce, ct)
            .map_err(|_| CryptoError::Aead)
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
        match key.encrypt_token(secret.as_bytes()) {
            Ok(token) => {
                out.push_str(&token);
                count += 1;
            }
            Err(_) => {
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
    let mut out = String::with_capacity(input.len());
    let mut count = 0usize;
    let mut rest = input;
    while let Some(start) = rest.find(TOKEN_PREFIX) {
        let after_prefix = &rest[start + TOKEN_PREFIX.len()..];
        let end = match after_prefix.find(TOKEN_SUFFIX) {
            Some(e) => e,
            None => break,
        };
        let b64 = &after_prefix[..end];
        out.push_str(&rest[..start]);
        match key.decrypt_b64(b64) {
            Ok(plain) => {
                out.push_str(&String::from_utf8_lossy(&plain));
                count += 1;
            }
            Err(_) => {
                // Jeton conservé tel quel si déchiffrement impossible.
                out.push_str(TOKEN_PREFIX);
                out.push_str(b64);
                out.push_str(TOKEN_SUFFIX);
            }
        }
        rest = &after_prefix[end + TOKEN_SUFFIX.len()..];
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
        assert!(!inner.contains('-') && !inner.contains('_'), "pas d'URL-safe");
    }

    #[test]
    fn decrypt_accepts_url_safe_base64() {
        // Un client (p. ex. Android `Base64.URL_SAFE`) peut émettre la même
        // charge en alphabet URL-safe : brocolis doit la déchiffrer.
        let k = SymKey::generate();
        let std_token = k.encrypt_token("café Android".as_bytes()).unwrap();
        let inner = &std_token[TOKEN_PREFIX.len()..std_token.len() - TOKEN_SUFFIX.len()];
        let packed = B64.decode(inner).unwrap();

        let url_token = format!(
            "{TOKEN_PREFIX}{}{TOKEN_SUFFIX}",
            URL_SAFE.encode(&packed)
        );
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
}
