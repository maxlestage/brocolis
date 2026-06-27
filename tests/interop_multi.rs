//! Interopérabilité du jeton multi-destinataire `[[BROCOLIS:v2:…]]` avec l'app
//! iOS « Brocolis 🥦 » (`BrocolisCrypto.encryptTokenMulti` / `decryptTokenV2`).
//!
//! Le jeton v2 chiffre le texte UNE fois sous une « clé de message » aléatoire,
//! elle-même emballée sous CHAQUE clé destinataire. Disposition du payload
//! (puis base64) :
//!
//!   [0]                    format = 0x01
//!   [1]                    N = nombre de destinataires (1..=255)
//!   [2 .. 2+60N)           N clés de message emballées (60 octets chacune :
//!                          nonce 12 + clé 32 + tag 16, format « combined »)
//!   [2+60N .. fin)         corps = AES-GCM(texte, cléDeMessage) « combined »
//!
//! Si la disposition dérive (octet de format, taille d'emballage, ordre…), ces
//! tests cassent — c'est un verrou de compat avec la messagerie.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use brocolis::crypto::{decrypt_tokens_multi, encrypt_token_multi};
use brocolis::SymKey;

const PREFIX_V2: &str = "[[BROCOLIS:v2:";
const SUFFIX: &str = "]]";
const WRAPPED_KEY_LEN: usize = 12 + 32 + 16; // 60

/// La disposition du payload v2 est exactement celle de l'app iOS.
#[test]
fn v2_payload_layout_matches_ios() {
    let recipients = [SymKey::generate(), SymKey::generate()];
    let plaintext = "Rendez-vous 🥦 à 18h";
    let token = encrypt_token_multi(plaintext.as_bytes(), &recipients).unwrap();

    let inner = token
        .strip_prefix(PREFIX_V2)
        .and_then(|s| s.strip_suffix(SUFFIX))
        .expect("préfixe/suffixe BROCOLIS:v2");
    let payload = B64.decode(inner).expect("base64 standard");

    assert_eq!(payload[0], 0x01, "octet de format = 0x01");
    assert_eq!(payload[1] as usize, recipients.len(), "N destinataires");

    let body_start = 2 + recipients.len() * WRAPPED_KEY_LEN;
    // corps = nonce(12) + ciphertext(len utf8) + tag(16).
    assert_eq!(payload.len(), body_start + 12 + plaintext.len() + 16);
}

/// Chaque destinataire lit le clair ; un tiers (mauvaise clé) ne lit rien et le
/// jeton est laissé tel quel.
#[test]
fn v2_each_recipient_reads_outsider_does_not() {
    let alice = SymKey::generate();
    let bob = SymKey::generate();
    let outsider = SymKey::generate();

    let token = encrypt_token_multi(b"secret partage", &[alice.clone(), bob.clone()]).unwrap();

    for k in [&alice, &bob] {
        let (dec, n) = decrypt_tokens_multi(&token, std::slice::from_ref(k));
        assert_eq!(n, 1);
        assert_eq!(dec, "secret partage");
    }

    let (out, n) = decrypt_tokens_multi(&token, std::slice::from_ref(&outsider));
    assert_eq!(n, 0);
    assert_eq!(out, token, "jeton conservé intact pour un tiers");
}

/// Un texte mêlant un jeton v1 (pour Alice) et un jeton v2 (pour Alice+Bob) se
/// lit intégralement avec le trousseau, sans changer de clé.
#[test]
fn mixed_v1_and_v2_decrypt_with_keyring() {
    let alice = SymKey::generate();
    let bob = SymKey::generate();

    let v1 = alice.encrypt_token("coucou".as_bytes()).unwrap();
    let v2 = encrypt_token_multi("groupe".as_bytes(), &[alice.clone(), bob.clone()]).unwrap();
    let blob = format!("{v1} et {v2}");

    let (out, n) = decrypt_tokens_multi(&blob, &[alice, bob]);
    assert_eq!(n, 2);
    assert_eq!(out, "coucou et groupe");
}
