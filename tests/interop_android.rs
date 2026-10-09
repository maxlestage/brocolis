//! Interopérabilité du format de jeton avec un client **Android** (Kotlin /
//! Java `javax.crypto.Cipher`) et tout client compatible.
//!
//! Le format est identique à celui d'iOS (cf. `interop_ios.rs`) — c'est tout
//! l'intérêt : un seul format de jeton universel.
//!
//! Côté Android, le jeton se construit ainsi :
//!
//! ```kotlin
//! val cipher = Cipher.getInstance("AES/GCM/NoPadding")
//! val iv = ByteArray(12)                       // 96 bits, aléatoire en prod
//! cipher.init(Cipher.ENCRYPT_MODE, SecretKeySpec(key, "AES"),
//!             GCMParameterSpec(128, iv))       // tag de 128 bits
//! val ctAndTag = cipher.doFinal(plaintext)     // ciphertext || tag(16)
//! val packed = iv + ctAndTag                   // nonce(12) || ct || tag(16)
//! val token = "[[BROCOLIS:v1:" +
//!     Base64.encodeToString(packed, Base64.NO_WRAP) + "]]"
//! ```
//!
//! `Base64.NO_WRAP` (alphabet standard) comme `Base64.URL_SAFE` doivent tous
//! deux être déchiffrables : brocolis émet du base64 standard mais **accepte**
//! l'URL-safe en entrée. Ces vecteurs figés verrouillent les deux contrats.
//!
//! Vecteur déterministe : clé = octets 0x00..=0x1f, nonce = 0x00..=0x0b,
//! plaintext = "Rendez-vous a 18h, code 4291 !!".

use base64::engine::general_purpose::{STANDARD as B64, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use brocolis::crypto::decrypt_tokens;
use brocolis::SymKey;

const KEY_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
const PLAINTEXT: &str = "Rendez-vous a 18h, code 4291 !!";

const PREFIX: &str = "[[BROCOLIS:v1:";
const SUFFIX: &str = "]]";

/// Charge `nonce(12) || ciphertext || tag(16)` produite par AES-256-GCM
/// (clé/nonce déterministes ci-dessus) — exactement ce que `Cipher.doFinal`
/// d'Android renvoie une fois l'IV préfixé. Encodée ici en base64 standard,
/// choisie pour contenir `+` et `/` afin de distinguer réellement de
/// l'alphabet URL-safe.
const PAYLOAD_STD: &str =
    "AAECAwQFBgcICQoLFWe4f6Cf723iNOSr0MlJVev6p1efHzpcDFXctD1IIfO+CZIeR9w/YXdYpasn5bo=";

/// Jeton émis par un client Android utilisant l'alphabet **standard**
/// (`Base64.NO_WRAP`).
#[test]
fn android_standard_token_decrypts() {
    let key = SymKey::from_base64(KEY_B64).expect("clé valide");
    let token = format!("{PREFIX}{PAYLOAD_STD}{SUFFIX}");
    let (plain, n) = decrypt_tokens(&token, &key);
    assert_eq!(n, 1, "exactement un jeton déchiffré");
    assert_eq!(
        plain, PLAINTEXT,
        "le clair doit correspondre au vecteur Android"
    );
}

/// Jeton émis par un client Android utilisant `Base64.URL_SAFE` : la charge
/// est identique au bit près, seul l'alphabet change (`+/` -> `-_`).
#[test]
fn android_url_safe_token_decrypts() {
    let key = SymKey::from_base64(KEY_B64).expect("clé valide");
    let packed = B64.decode(PAYLOAD_STD).expect("base64 standard");

    // Confirme que l'URL-safe diffère réellement du standard pour ce vecteur.
    let url = URL_SAFE.encode(&packed);
    assert_ne!(
        url, PAYLOAD_STD,
        "le vecteur doit exercer l'alphabet URL-safe"
    );
    assert!(url.contains('-') || url.contains('_'));

    let token = format!("{PREFIX}{url}{SUFFIX}");
    let (plain, n) = decrypt_tokens(&token, &key);
    assert_eq!(n, 1);
    assert_eq!(plain, PLAINTEXT);
}

/// Variante URL-safe **sans padding** (`Base64.URL_SAFE | Base64.NO_PADDING`,
/// ou `kotlin.io.encoding.Base64.UrlSafe` avec `withPadding(Absent)`).
#[test]
fn android_url_safe_no_padding_decrypts() {
    let key = SymKey::from_base64(KEY_B64).expect("clé valide");
    let packed = B64.decode(PAYLOAD_STD).expect("base64 standard");

    let token = format!("{PREFIX}{}{SUFFIX}", URL_SAFE_NO_PAD.encode(&packed));
    let (plain, n) = decrypt_tokens(&token, &key);
    assert_eq!(n, 1);
    assert_eq!(plain, PLAINTEXT);
}

/// L'empaquetage est bien `nonce(12) || ciphertext || tag(16)`, nonce figé.
#[test]
fn android_vector_layout_is_nonce_ct_tag() {
    let packed = B64.decode(PAYLOAD_STD).expect("base64 standard");
    let pt_len = PLAINTEXT.len();
    assert_eq!(packed.len(), 12 + pt_len + 16, "nonce + ciphertext + tag");

    let expected_nonce: Vec<u8> = (0u8..12).collect();
    assert_eq!(&packed[..12], &expected_nonce[..], "nonce figé 0x00..0x0b");
}

/// Une mauvaise clé laisse le jeton Android intact (pas de panique, pas de fuite).
#[test]
fn wrong_key_keeps_android_token_intact() {
    let other = SymKey::generate();
    let token = format!("{PREFIX}{PAYLOAD_STD}{SUFFIX}");
    let (out, n) = decrypt_tokens(&token, &other);
    assert_eq!(n, 0);
    assert_eq!(out, token);
}
