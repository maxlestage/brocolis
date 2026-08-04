//! Accord de clé X25519 (ECDH) — port exact de `BrocolisKeyAgreement.swift`.
//!
//! Chaque appareil possède une paire de clés ; on échange les CLÉS PUBLIQUES
//! (via le serveur de rendez-vous), puis chacun dérive de son côté la MÊME clé
//! symétrique AES-256 (HKDF-SHA256). La clé secrète n'est jamais transmise.
//!
//! Une empreinte courte (issue des deux clés publiques) détecte un intercepteur
//! (MITM) en la comparant de vive voix.
//!
//! Byte-compatible avec CryptoKit (`Curve25519.KeyAgreement`) : mêmes sels
//! (`brocolis-x25519`, `brocolis-x25519-wrap`), même `info`
//! (`sortedConcat(myPub, peerPub)`), même empaquetage AES-GCM « combined » pour
//! les clés de groupe. Un correspondant iOS et un correspondant brocolis
//! dérivent ainsi exactement la même clé — on parle sans rien recopier.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use hkdf::Hkdf;
use rand::RngCore;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, SharedSecret, StaticSecret};

use crate::crypto::{CryptoError, SymKey, KEY_LEN};

const SALT: &[u8] = b"brocolis-x25519";
const WRAP_SALT: &[u8] = b"brocolis-x25519-wrap";
const PUB_LEN: usize = 32;

/// Erreur d'accord de clé.
#[derive(Debug, thiserror::Error)]
pub enum AgreementError {
    #[error("clé publique du correspondant invalide")]
    BadPeerKey,
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

/// Clé privée X25519 locale (persistée en base64 par l'appelant), équivalent de
/// `Curve25519.KeyAgreement.PrivateKey`.
pub struct PrivateKey {
    secret: StaticSecret,
}

/// Résultat d'une dérivation : clé symétrique partagée (base64) + empreinte.
pub struct Derived {
    pub symmetric_key_base64: String,
    pub fingerprint: String,
}

impl PrivateKey {
    /// Génère une nouvelle paire de clés aléatoire (CSPRNG).
    pub fn generate() -> Self {
        let mut bytes = [0u8; PUB_LEN];
        rand::rng().fill_bytes(&mut bytes);
        Self {
            secret: StaticSecret::from(bytes),
        }
    }

    /// Charge depuis la représentation brute base64 (32 octets), comme
    /// `Curve25519.KeyAgreement.PrivateKey(rawRepresentation:)`.
    pub fn from_base64(s: &str) -> Result<Self, AgreementError> {
        let raw = B64
            .decode(s.trim())
            .map_err(|_| AgreementError::BadPeerKey)?;
        let bytes: [u8; PUB_LEN] = raw.try_into().map_err(|_| AgreementError::BadPeerKey)?;
        Ok(Self {
            secret: StaticSecret::from(bytes),
        })
    }

    /// Représentation brute base64 de la clé privée (à persister localement).
    pub fn to_base64(&self) -> String {
        B64.encode(self.secret.to_bytes())
    }

    /// Clé publique base64, à transmettre au correspondant via le rendez-vous.
    pub fn public_key_base64(&self) -> String {
        B64.encode(PublicKey::from(&self.secret).to_bytes())
    }

    /// Dérive la clé AES partagée + l'empreinte à partir de la clé publique du
    /// correspondant (base64). Le résultat est identique des deux côtés.
    pub fn derive(&self, peer_public_base64: &str) -> Result<Derived, AgreementError> {
        let peer = peer_public_data(peer_public_base64)?;
        let shared = self.shared_secret(&peer)?;
        let info = sorted_concat(&self.public_bytes(), &peer);
        let key = hkdf_sha256(SALT, shared.as_bytes(), &info);
        Ok(Derived {
            symmetric_key_base64: B64.encode(key),
            fingerprint: fingerprint(&info),
        })
    }

    // --- Clé de groupe (key-wrapping X25519) ---

    /// Emballe (chiffre) la clé de groupe à destination d'un membre, via la clé
    /// pairwise dérivée de ma clé privée et de SA clé publique. Renvoie un blob
    /// base64 que seul ce membre pourra déballer.
    pub fn wrap_group_key(
        &self,
        group_key_base64: &str,
        peer_public_base64: &str,
    ) -> Result<String, AgreementError> {
        let group_key = B64
            .decode(group_key_base64.trim())
            .map_err(|_| AgreementError::BadPeerKey)?;
        let peer = peer_public_data(peer_public_base64)?;
        let wrap_key = self.pairwise_wrap_key(&peer)?;
        Ok(B64.encode(wrap_key.seal(&group_key)?))
    }

    /// Déballe la clé de groupe reçue, via la clé pairwise dérivée de ma clé
    /// privée et de la clé publique de l'émetteur. Renvoie la clé AES (base64).
    pub fn unwrap_group_key(
        &self,
        blob_base64: &str,
        from_public_base64: &str,
    ) -> Result<String, AgreementError> {
        let packed = B64
            .decode(blob_base64.trim())
            .map_err(|_| AgreementError::BadPeerKey)?;
        let from = peer_public_data(from_public_base64)?;
        let wrap_key = self.pairwise_wrap_key(&from)?;
        Ok(B64.encode(wrap_key.open(&packed)?))
    }

    fn public_bytes(&self) -> [u8; PUB_LEN] {
        PublicKey::from(&self.secret).to_bytes()
    }

    /// Secret partagé X25519, en REFUSANT les points d'ordre faible.
    ///
    /// Une clé publique d'ordre faible (0, 1, p-1…) force un secret partagé
    /// entièrement nul : l'attaquant qui l'envoie n'a donc besoin d'aucune clé
    /// privée pour le connaître. Comme la dérivation ne mélange ensuite que des
    /// valeurs publiques (les deux clés publiques), il peut recalculer la clé
    /// pairwise à l'identique et se faire passer pour un correspondant légitime
    /// — c'est-à-dire déballer les clés de groupe qui lui sont destinées.
    ///
    /// `was_contributory()` détecte exactement ce cas.
    fn shared_secret(&self, peer: &[u8; PUB_LEN]) -> Result<SharedSecret, AgreementError> {
        let shared = self.secret.diffie_hellman(&PublicKey::from(*peer));
        if !shared.was_contributory() {
            tracing::warn!(
                "clé publique d'ordre faible refusée : secret partagé dégénéré (contribution nulle)"
            );
            return Err(AgreementError::BadPeerKey);
        }
        Ok(shared)
    }

    /// Clé symétrique pairwise (même secret des deux côtés) servant à emballer.
    fn pairwise_wrap_key(&self, peer: &[u8; PUB_LEN]) -> Result<SymKey, AgreementError> {
        let shared = self.shared_secret(peer)?;
        let info = sorted_concat(&self.public_bytes(), peer);
        let raw = hkdf_sha256(WRAP_SALT, shared.as_bytes(), &info);
        Ok(SymKey::from_bytes(&raw)?)
    }
}

/// Génère une clé de groupe AES-256 aléatoire (base64).
pub fn generate_group_key_base64() -> String {
    SymKey::generate().to_base64()
}

/// Empreinte d'une clé de groupe (identique pour tous les membres).
pub fn group_fingerprint(group_key_base64: &str) -> String {
    let data = B64.decode(group_key_base64.trim()).unwrap_or_default();
    fingerprint(&data)
}

// --- Outils ---

fn peer_public_data(b64: &str) -> Result<[u8; PUB_LEN], AgreementError> {
    let raw = B64
        .decode(b64.trim())
        .map_err(|_| AgreementError::BadPeerKey)?;
    raw.try_into().map_err(|_| AgreementError::BadPeerKey)
}

fn hkdf_sha256(salt: &[u8], ikm: &[u8], info: &[u8]) -> [u8; KEY_LEN] {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut okm = [0u8; KEY_LEN];
    hk.expand(info, &mut okm)
        .expect("KEY_LEN est une longueur valide pour HKDF-SHA256");
    okm
}

fn sorted_concat(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    if lex_less_or_equal(a, b) {
        out.extend_from_slice(a);
        out.extend_from_slice(b);
    } else {
        out.extend_from_slice(b);
        out.extend_from_slice(a);
    }
    out
}

fn lex_less_or_equal(a: &[u8], b: &[u8]) -> bool {
    for (x, y) in a.iter().zip(b.iter()) {
        if x != y {
            return x < y;
        }
    }
    a.len() <= b.len()
}

/// Empreinte lisible (8 octets en hex majuscule, groupés par 4 avec des
/// tirets) à comparer de vive voix. Identique à
/// `BrocolisKeyAgreement.fingerprint`.
pub fn fingerprint(info: &[u8]) -> String {
    let digest = Sha256::digest(info);
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02X}")).collect();
    let mut out = String::with_capacity(hex.len() + 3);
    for (i, ch) in hex.chars().enumerate() {
        if i != 0 && i % 4 == 0 {
            out.push('-');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{decrypt_tokens, encrypt_marked};

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    /// Une clé publique d'ordre faible doit être REFUSÉE.
    ///
    /// Elle force un secret partagé nul, que l'attaquant connaît donc sans
    /// détenir la moindre clé privée. La dérivation ne mélangeant ensuite que
    /// des valeurs publiques, il recalculerait la clé pairwise à l'identique et
    /// déballerait les clés de groupe qui lui sont destinées.
    #[test]
    fn les_points_dordre_faible_sont_refuses() {
        let faibles: [[u8; 32]; 5] = [
            [0u8; 32],
            {
                let mut a = [0u8; 32];
                a[0] = 1;
                a
            },
            // p-1 (ordre 2)
            [
                0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
            ],
            // ordre 4
            [
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
            ],
            // ordre 8
            [
                0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1,
                0x9f, 0xc4, 0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62,
                0x05, 0x16, 0x5f, 0x49, 0xb8, 0x00,
            ],
        ];

        let moi = PrivateKey::generate();
        for (i, pt) in faibles.iter().enumerate() {
            let b64 = B64.encode(pt);
            assert!(
                moi.derive(&b64).is_err(),
                "le point d'ordre faible {i} doit être refusé par derive"
            );
            assert!(
                moi.wrap_group_key(&generate_group_key_base64(), &b64).is_err(),
                "le point d'ordre faible {i} doit être refusé par wrap_group_key"
            );
            assert!(
                moi.unwrap_group_key("AAAA", &b64).is_err(),
                "le point d'ordre faible {i} doit être refusé par unwrap_group_key"
            );
        }

        // Un échange honnête continue de fonctionner.
        let pair = PrivateKey::generate();
        assert!(moi.derive(&pair.public_key_base64()).is_ok());
    }

    /// Vecteur figé de la dérivation complète X25519 + HKDF-SHA256.
    ///
    /// `public_key_matches_rfc7748_vector` ne verrouille que la multiplication
    /// scalaire ; `derive_is_symmetric` ne compare que les deux côtés entre eux
    /// et passerait donc même si HKDF changeait des deux côtés à la fois. Ici on
    /// fige la SORTIE pour les clés RFC 7748 d'Alice et Bob : si le sel, l'info,
    /// l'ordre de concaténation ou la fonction de dérivation bougent, ce test
    /// casse — et avec lui la compatibilité des clés de groupe avec iOS.
    #[test]
    fn derive_matches_frozen_vector() {
        let alice = PrivateKey::from_base64(&B64.encode(hex(
            "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
        )))
        .unwrap();
        let bob = PrivateKey::from_base64(&B64.encode(hex(
            "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb",
        )))
        .unwrap();

        let d = alice.derive(&bob.public_key_base64()).unwrap();
        assert_eq!(
            d.symmetric_key_base64, "BQpdh50P8dL/dviLgZUEBhww21f1AT0MkIzGAweqwyE=",
            "la clé pairwise dérivée doit rester identique octet à octet"
        );
        assert_eq!(
            d.fingerprint, "9962-5B51-D4FD-490E",
            "l'empreinte affichée à l'utilisateur doit rester stable"
        );
    }

    /// Empreinte SHA-256 figée : verrouille la troncature à 8 octets et le
    /// formatage en groupes de 4, tels que les affiche aussi le client iOS.
    #[test]
    fn fingerprint_matches_frozen_vector() {
        assert_eq!(fingerprint(b"brocolis"), "AFC5-22D8-1B62-E36B");
    }

    /// Vecteur RFC 7748 §6.1 : la clé publique dérivée de la clé privée d'Alice
    /// doit être EXACTEMENT la clé publique standard. Cela verrouille la
    /// compatibilité de la représentation X25519 avec CryptoKit (qui utilise le
    /// même X25519 brut sur 32 octets).
    #[test]
    fn public_key_matches_rfc7748_vector() {
        let alice_priv = hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let alice_pub = hex("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");
        let priv_key = PrivateKey::from_base64(&B64.encode(&alice_priv)).unwrap();
        let derived_pub = B64.decode(priv_key.public_key_base64()).unwrap();
        assert_eq!(
            derived_pub, alice_pub,
            "X25519 public key must match RFC 7748"
        );
    }

    /// Les deux correspondants dérivent la MÊME clé et la MÊME empreinte.
    #[test]
    fn derive_is_symmetric() {
        let alice = PrivateKey::generate();
        let bob = PrivateKey::generate();

        let a = alice.derive(&bob.public_key_base64()).unwrap();
        let b = bob.derive(&alice.public_key_base64()).unwrap();

        assert_eq!(a.symmetric_key_base64, b.symmetric_key_base64);
        assert_eq!(a.fingerprint, b.fingerprint);
    }

    /// La clé dérivée par X25519 sert directement à chiffrer/déchiffrer un
    /// jeton : le pont avec la couche de jetons fonctionne.
    #[test]
    fn derived_key_drives_token_layer() {
        let alice = PrivateKey::generate();
        let bob = PrivateKey::generate();
        let shared = alice.derive(&bob.public_key_base64()).unwrap();

        let key = SymKey::from_base64(&shared.symmetric_key_base64).unwrap();
        let (enc, n) = encrypt_marked("rdv ((secret)) ok", &key);
        assert_eq!(n, 1);

        // Bob dérive la même clé et lit le clair.
        let bob_shared = bob.derive(&alice.public_key_base64()).unwrap();
        let bob_key = SymKey::from_base64(&bob_shared.symmetric_key_base64).unwrap();
        let (dec, m) = decrypt_tokens(&enc, &bob_key);
        assert_eq!(m, 1);
        assert_eq!(dec, "rdv secret ok");
    }

    /// Format d'empreinte : 8 octets → 16 hex majuscules, groupés par 4.
    #[test]
    fn fingerprint_format() {
        let fp = fingerprint(b"peu importe");
        assert_eq!(fp.len(), 19, "16 hex + 3 tirets");
        assert_eq!(fp.matches('-').count(), 3);
        let groups: Vec<&str> = fp.split('-').collect();
        assert_eq!(groups.len(), 4);
        for g in groups {
            assert_eq!(g.len(), 4);
            assert!(g
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_lowercase()));
        }
    }

    /// Clé de groupe : Alice l'emballe pour Bob, Bob la déballe à l'identique,
    /// et l'empreinte de groupe est commune.
    #[test]
    fn group_key_wrap_unwrap_roundtrip() {
        let alice = PrivateKey::generate();
        let bob = PrivateKey::generate();

        let group = generate_group_key_base64();
        let blob = alice
            .wrap_group_key(&group, &bob.public_key_base64())
            .unwrap();
        let unwrapped = bob
            .unwrap_group_key(&blob, &alice.public_key_base64())
            .unwrap();

        assert_eq!(unwrapped, group);
        assert_eq!(
            group_fingerprint(&group),
            group_fingerprint(&unwrapped),
            "empreinte de groupe commune"
        );
    }

    /// La clé privée se persiste et se recharge sans changer la clé publique.
    #[test]
    fn private_key_base64_roundtrip() {
        let key = PrivateKey::generate();
        let pub_before = key.public_key_base64();
        let reloaded = PrivateKey::from_base64(&key.to_base64()).unwrap();
        assert_eq!(reloaded.public_key_base64(), pub_before);
    }

    #[test]
    fn bad_peer_key_rejected() {
        let me = PrivateKey::generate();
        assert!(me.derive("pas du base64 !!").is_err());
        assert!(me.derive(&B64.encode([0u8; 8])).is_err()); // mauvaise longueur
    }
}
