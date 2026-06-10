//! Interopérabilité du format de jeton avec l'app iOS « Brocolis 🥦 »
//! (extension de partage, CryptoKit) et tout client compatible.
//!
//! On fige un **vecteur** produit au format iOS : empaquetage
//! `nonce (12) || ciphertext || tag (16)`, base64 standard, encadré par
//! `[[BROCOLIS:v1: … ]]`. C'est exactement la représentation `combined` de
//! CryptoKit (`AES.GCM.SealedBox`) et de l'AES-256-GCM de Node.
//!
//! Si le format de brocolis dérive (préfixe, base64, ordre nonce/tag…), ces
//! tests cassent — c'est le but : un verrou anti-régression de la compat.
//!
//! Vecteur déterministe : clé = octets 0x00..=0x1f, nonce = 0x00..=0x0b.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use brocolis::crypto::decrypt_tokens;
use brocolis::SymKey;

const KEY_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
const TOKEN: &str =
    "[[BROCOLIS:v1:AAECAwQFBgcICQoLFWe4f6Cf723iNOSrQXbdy6MVJxTBQzdQGASK4XhJNIA4IZzUdgcFrApYMBr2o3Hk5jg=]]";
const PLAINTEXT: &str = "Rendez-vous 🥦 à 18h, code 4291";

const PREFIX: &str = "[[BROCOLIS:v1:";
const SUFFIX: &str = "]]";

/// Le jeton produit côté iOS doit se déchiffrer dans brocolis.
#[test]
fn ios_vector_decrypts_in_brocolis() {
    let key = SymKey::from_base64(KEY_B64).expect("clé valide");
    let (plain, n) = decrypt_tokens(TOKEN, &key);
    assert_eq!(n, 1, "exactement un jeton déchiffré");
    assert_eq!(plain, PLAINTEXT, "le clair doit correspondre au vecteur iOS");
}

/// L'empaquetage du vecteur est bien `nonce(12) || ciphertext || tag(16)`.
#[test]
fn ios_vector_layout_is_nonce_ct_tag() {
    let inner = TOKEN
        .strip_prefix(PREFIX)
        .and_then(|s| s.strip_suffix(SUFFIX))
        .expect("préfixe/suffixe BROCOLIS:v1");
    let packed = B64.decode(inner).expect("base64 standard");

    let pt_len = PLAINTEXT.len();
    assert_eq!(packed.len(), 12 + pt_len + 16, "nonce + ciphertext + tag");

    let expected_nonce: Vec<u8> = (0u8..12).collect();
    assert_eq!(&packed[..12], &expected_nonce[..], "nonce figé 0x00..0x0b");
}

/// Le chiffrement de brocolis produit le même empaquetage et fait l'aller-retour.
#[test]
fn brocolis_token_layout_matches_and_roundtrips() {
    let key = SymKey::generate();
    let secret = "hello";
    let token = key.encrypt_token(secret.as_bytes()).expect("chiffrement");

    assert!(token.starts_with(PREFIX) && token.ends_with(SUFFIX));
    let inner = token
        .strip_prefix(PREFIX)
        .and_then(|s| s.strip_suffix(SUFFIX))
        .unwrap();
    let packed = B64.decode(inner).expect("base64");
    assert_eq!(packed.len(), 12 + secret.len() + 16);

    let (plain, n) = decrypt_tokens(&token, &key);
    assert_eq!(n, 1);
    assert_eq!(plain, secret);
}

/// Une mauvaise clé laisse le jeton iOS intact (pas de panique, pas de fuite).
#[test]
fn wrong_key_keeps_ios_token_intact() {
    let other = SymKey::generate();
    let (out, n) = decrypt_tokens(TOKEN, &other);
    assert_eq!(n, 0);
    assert_eq!(out, TOKEN);
}
