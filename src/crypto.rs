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
use rand::Rng;

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
/// Taille fixe d'une clé de message emballée : nonce(12) + clé(32) + tag(16).
const WRAPPED_KEY_LEN: usize = NONCE_LEN + KEY_LEN + TAG_LEN;
/// Tête commune à toutes les versions de jeton (`v1`, `v2`, `v3`).
const TOKEN_HEAD: &str = "[[BROCOLIS:";
const TOKEN_PREFIX: &str = "[[BROCOLIS:v1:";
const TOKEN_PREFIX_V2: &str = "[[BROCOLIS:v2:";
const TOKEN_SUFFIX: &str = "]]";
const PLAIN_OPEN: &str = "((";
const PLAIN_CLOSE: &str = "))";

/// Remplace un segment que l'on n'a PAS réussi à chiffrer.
///
/// Le principe est l'échec fermé : si le chiffrement échoue, le secret ne part
/// pas. Le marqueur est visible pour que l'utilisateur comprenne qu'il doit
/// corriger sa configuration, plutôt que de croire son message protégé.
const REDACTED_MARKER: &str = "[[BROCOLIS:ERREUR-CHIFFREMENT]]";

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
    #[error("aucune clé pour chiffrer")]
    NoKey,
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

    /// Les 32 octets de la clé.
    pub fn bytes(&self) -> [u8; KEY_LEN] {
        self.bytes
    }

    fn cipher(&self) -> Aes256Gcm {
        // aes-gcm 0.11 déprécie `from_slice` au profit de `From`/`TryFrom`
        // (hybrid-array). `self.bytes` fait exactement KEY_LEN, donc `From`
        // suffit et reste infaillible.
        Aes256Gcm::new(&Key::<Aes256Gcm>::from(self.bytes))
    }

    /// Scelle `plaintext` en AES-256-GCM et renvoie l'empaquetage « combined »
    /// `nonce (12) || ciphertext || tag (16)` — identique à
    /// `AES.GCM.SealedBox.combined` de CryptoKit. Brique partagée par les jetons
    /// v1, l'enveloppe v2 et l'emballage des clés de groupe X25519.
    pub fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);
        let ct = self
            .cipher()
            .encrypt(&nonce, plaintext)
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
        // `split_at(NONCE_LEN)` garantit la longueur, mais on reste sur
        // `try_from` (l'API non dépréciée) plutôt que de déballer.
        let nonce = Nonce::try_from(nonce_bytes).map_err(|_| CryptoError::BadToken)?;
        self.cipher()
            .decrypt(&nonce, ct)
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
    encrypt_marked_with(input, &mut |secret| seal_token(secret.as_bytes(), keys))
}

/// Comme [`encrypt_marked_multi`], mais chaque segment est scellé par `seal`
/// (le protocole v3 : [`crate::v3store`]). `None` : échec fermé.
pub fn encrypt_marked_with(
    input: &str,
    seal: &mut dyn FnMut(&str) -> Option<String>,
) -> (String, usize) {
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
        match seal(secret) {
            Some(token) => {
                out.push_str(&token);
                count += 1;
            }
            None => {
                // ÉCHEC FERMÉ. Ce bloc réémettait le segment en clair, ce qui
                // envoyait le secret au serveur — exactement ce que l'outil
                // existe pour empêcher. Un chiffrement impossible (trousseau de
                // plus de 255 clés en mode multi, erreur AEAD…) ne doit JAMAIS
                // dégrader vers le texte lisible.
                tracing::error!(
                    "chiffrement impossible pour un segment de {} octets : \
                     segment remplacé par un marqueur d'erreur (le clair n'est PAS émis)",
                    secret.len()
                );
                out.push_str(REDACTED_MARKER);
            }
        }
        rest = &after_open[close + PLAIN_CLOSE.len()..];
    }
    out.push_str(rest);
    (out, count)
}

/// Le texte contient-il au moins un segment `((…))` complet ?
pub fn has_marked_segment(input: &str) -> bool {
    input
        .find(PLAIN_OPEN)
        .is_some_and(|open| input[open + PLAIN_OPEN.len()..].contains(PLAIN_CLOSE))
}

/// Ce que [`encrypt_message`] a chiffré.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sealed {
    /// Les `n` segments `((marqués))`, le reste du message intact.
    Segments(usize),
    /// Le message entier, en un seul jeton.
    Whole,
    /// Rien : message vide.
    Nothing,
}

/// Chiffre un message pour `keys` (une clé : jeton `v1` ; plusieurs : un seul
/// jeton `v2` que chaque destinataire ouvre avec la sienne).
///
/// - S'il contient des segments `((…))`, seuls ceux-là sont chiffrés : on peut
///   laisser le début de la phrase lisible et protéger un détail.
/// - Sinon, c'est le message ENTIER qui est chiffré, en un seul jeton. Les
///   espaces et sauts de ligne qui l'entourent restent en dehors, pour qu'il
///   se colle proprement.
///
/// C'est la règle des outils (CLI `encrypt`, application). Le proxy, lui, ne
/// chiffre QUE les segments marqués : chiffrer tout ce qu'un navigateur
/// envoie casserait les sites.
pub fn encrypt_message(input: &str, keys: &[SymKey]) -> Result<(String, Sealed), CryptoError> {
    if keys.is_empty() {
        return Err(CryptoError::NoKey);
    }
    encrypt_message_with(input, &mut |text| seal_token(text.as_bytes(), keys))
}

/// Comme [`encrypt_message`], mais le message (ou chaque segment) est scellé
/// par `seal` (le protocole v3 : [`crate::v3store`]).
pub fn encrypt_message_with(
    input: &str,
    seal: &mut dyn FnMut(&str) -> Option<String>,
) -> Result<(String, Sealed), CryptoError> {
    if has_marked_segment(input) {
        let (out, n) = encrypt_marked_with(input, seal);
        return Ok((out, Sealed::Segments(n)));
    }
    let body = input.trim();
    if body.is_empty() {
        return Ok((input.to_string(), Sealed::Nothing));
    }
    let start = input.len() - input.trim_start().len();
    let end = start + body.len();
    // Échec fermé, comme pour les segments : jamais de repli vers le clair.
    let token = seal(body).ok_or(CryptoError::Aead)?;
    Ok((
        format!("{}{token}{}", &input[..start], &input[end..]),
        Sealed::Whole,
    ))
}

// ---------------------------------------------------------------------------
// Corps de formulaire (application/x-www-form-urlencoded)
// ---------------------------------------------------------------------------
//
// Un navigateur qui soumet <input value="((mon secret))"> n'envoie PAS ces
// caractères tels quels : il émet `t=%28%28mon+secret%29%29`. Les marqueurs
// étaient donc invisibles pour la détection, et le secret partait en clair —
// exactement le cas d'usage principal de l'outil (écrire un secret dans un
// champ de formulaire).
//
// On décode donc chaque valeur de champ, on y cherche les marqueurs, et on ne
// réencode QUE les champs réellement modifiés : un formulaire sans marqueur
// ressort octet pour octet identique, ce qui limite au strict minimum le risque
// d'abîmer un envoi.

/// Décode une valeur de champ `application/x-www-form-urlencoded`.
fn form_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                // Un `%` non suivi de deux chiffres hexa rend la valeur
                // indécodable : on préfère ne pas y toucher du tout.
                let h = b.get(i + 1)?;
                let l = b.get(i + 2)?;
                let hv = (*h as char).to_digit(16)?;
                let lv = (*l as char).to_digit(16)?;
                out.push((hv * 16 + lv) as u8);
                i += 3;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Réencode une valeur de champ. On échappe tout sauf les caractères non
/// réservés : le jeton contient `+`, `/` et `=`, qui seraient sinon réinterprétés
/// par le serveur et détruiraient le base64.
fn form_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3 / 2);
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Chiffre les segments `((...))` d'un corps de formulaire urlencodé.
///
/// Renvoie `(corps_transformé, nb_segments_chiffrés)`. Les champs sans marqueur
/// sont laissés tels quels, à l'octet près.
pub fn encrypt_marked_form(body: &str, keys: &[SymKey]) -> (String, usize) {
    encrypt_marked_form_with(body, &mut |secret| seal_token(secret.as_bytes(), keys))
}

/// Comme [`encrypt_marked_form`], chaque segment étant scellé par `seal`.
pub fn encrypt_marked_form_with(
    body: &str,
    seal: &mut dyn FnMut(&str) -> Option<String>,
) -> (String, usize) {
    let mut total = 0usize;
    let parts: Vec<String> = body
        .split('&')
        .map(|pair| {
            // Un champ se découpe au PREMIER `=` ; la valeur peut en contenir.
            let (name, raw_value) = match pair.find('=') {
                Some(i) => (&pair[..i], &pair[i + 1..]),
                None => return pair.to_string(),
            };
            let Some(decoded) = form_decode(raw_value) else {
                return pair.to_string();
            };
            let (transformed, n) = encrypt_marked_with(&decoded, seal);
            if n == 0 {
                // Rien à chiffrer : on ne touche pas au champ d'origine.
                return pair.to_string();
            }
            total += n;
            format!("{name}={}", form_encode(&transformed))
        })
        .collect();
    (parts.join("&"), total)
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
    decrypt_tokens_escaped(input, keys, PlainEscape::None)
}

/// Comment échapper le clair réinjecté, selon le document qui l'accueille.
///
/// Le texte déchiffré est inséré dans un document que l'on ne contrôle pas. S'il
/// y est recopié tel quel, il n'est plus du texte : il devient de la syntaxe.
/// Deux conséquences, toutes deux constatées :
///
/// - dans une page HTML, un secret contenant `<script>` était injecté comme du
///   balisage et s'exécutait dans l'origine du site consulté ;
/// - dans une réponse JSON, un secret contenant `"` cassait le document, qui
///   devenait invalide et faisait échouer la page.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlainEscape {
    /// Aucun échappement (texte brut).
    None,
    /// Entités HTML : le clair s'affiche, il ne s'interprète pas.
    Html,
    /// Échappement de chaîne JSON : le document reste valide.
    Json,
}

impl PlainEscape {
    fn apply(self, s: &str) -> String {
        match self {
            PlainEscape::None => s.to_string(),
            // `&` en premier, sinon on ré-échapperait les entités produites.
            PlainEscape::Html => {
                let mut out = String::with_capacity(s.len() + 16);
                for c in s.chars() {
                    match c {
                        '&' => out.push_str("&amp;"),
                        '<' => out.push_str("&lt;"),
                        '>' => out.push_str("&gt;"),
                        '"' => out.push_str("&quot;"),
                        '\'' => out.push_str("&#39;"),
                        _ => out.push(c),
                    }
                }
                out
            }
            PlainEscape::Json => {
                let mut out = String::with_capacity(s.len() + 16);
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c if (c as u32) < 0x20 => {
                            out.push_str(&format!("\\u{:04x}", c as u32));
                        }
                        c => out.push(c),
                    }
                }
                out
            }
        }
    }
}

/// Comme [`decrypt_tokens_multi`], mais échappe le clair pour le document qui
/// l'accueille (voir [`PlainEscape`]).
pub fn decrypt_tokens_escaped(
    input: &str,
    keys: &[SymKey],
    escape: PlainEscape,
) -> (String, usize) {
    decrypt_tokens_with(input, keys, escape, &mut |_| None)
}

/// Comme [`decrypt_tokens_escaped`], les jetons v3 et v4
/// (`[[BROCOLIS:v3:…]]`, `[[BROCOLIS:v4:…]]`, donnés entiers) étant ouverts
/// par `v3` ([`crate::v3store`]).
pub fn decrypt_tokens_with(
    input: &str,
    keys: &[SymKey],
    escape: PlainEscape,
    v3: &mut dyn FnMut(&str) -> Option<String>,
) -> (String, usize) {
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
        } else if inner.starts_with("v3:") || inner.starts_with("v4:") {
            v3(&format!("{TOKEN_HEAD}{inner}{TOKEN_SUFFIX}"))
        } else {
            None
        };

        match decrypted {
            Some(plain) => {
                // Le clair est du TEXTE : on l'échappe pour le document qui
                // l'accueille, faute de quoi il y devient de la syntaxe.
                out.push_str(&escape.apply(&plain));
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
mod tests_echec_ferme {
    use super::*;

    /// Un chiffrement impossible ne doit JAMAIS laisser sortir le clair.
    ///
    /// Le déclencheur ici est un trousseau de plus de 255 clés en mode
    /// multi-destinataire (le format v2 code le nombre sur un octet), mais le
    /// repli concerne toute erreur de chiffrement.
    #[test]
    fn segment_non_chiffrable_nest_jamais_emis_en_clair() {
        let keys: Vec<SymKey> = (0..256).map(|_| SymKey::generate()).collect();
        let (sortie, n) = encrypt_marked_multi("rdv ((code secret 4291)) ok", &keys);

        assert_eq!(n, 0, "aucun segment n'a pu être chiffré");
        assert!(
            !sortie.contains("code secret 4291"),
            "le secret ne doit pas sortir en clair : {sortie}"
        );
        assert!(
            sortie.contains(REDACTED_MARKER),
            "un marqueur d'erreur doit remplacer le segment : {sortie}"
        );
        assert!(
            sortie.starts_with("rdv ") && sortie.ends_with(" ok"),
            "le texte hors marqueur est conservé : {sortie}"
        );
    }

    /// Le marqueur d'erreur n'est pas un jeton : le déchiffrement le laisse tel
    /// quel au lieu de planter ou de le consommer.
    #[test]
    fn le_marqueur_derreur_traverse_le_dechiffrement() {
        let k = SymKey::generate();
        let entree = format!("avant {REDACTED_MARKER} après");
        let (sortie, n) = decrypt_tokens(&entree, &k);
        assert_eq!(n, 0);
        assert_eq!(sortie, entree);
    }
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

#[cfg(test)]
mod tests_message {
    use super::*;

    #[test]
    fn sans_parentheses_tout_le_message_est_chiffre() {
        let k = SymKey::generate();
        let (out, what) =
            encrypt_message("\n  Rendez-vous demain à 18h  \n", std::slice::from_ref(&k)).unwrap();
        assert_eq!(what, Sealed::Whole);
        assert!(
            out.starts_with("\n  [[BROCOLIS:v1:") && out.ends_with("]]  \n"),
            "{out:?}"
        );
        assert!(!out.contains("Rendez-vous"));
        let (clair, n) = decrypt_tokens(&out, &k);
        assert_eq!((clair.as_str(), n), ("\n  Rendez-vous demain à 18h  \n", 1));
    }

    #[test]
    fn avec_parentheses_seuls_les_segments_sont_chiffres() {
        let k = SymKey::generate();
        let (out, what) =
            encrypt_message("rdv ((18h)) au ((café))", std::slice::from_ref(&k)).unwrap();
        assert_eq!(what, Sealed::Segments(2));
        assert!(out.starts_with("rdv [[BROCOLIS:v1:") && out.contains("]] au [[BROCOLIS:v1:"));
        assert_eq!(decrypt_tokens(&out, &k).0, "rdv 18h au café");
    }

    #[test]
    fn plusieurs_destinataires_un_seul_jeton_lisible_par_chacun() {
        let (alice, bob, eve) = (SymKey::generate(), SymKey::generate(), SymKey::generate());
        let (out, _) = encrypt_message("on se voit à 18h", &[alice.clone(), bob.clone()]).unwrap();
        assert!(
            out.starts_with("[[BROCOLIS:v2:"),
            "un seul jeton multi-destinataire"
        );
        assert_eq!(decrypt_tokens(&out, &alice).0, "on se voit à 18h");
        assert_eq!(decrypt_tokens(&out, &bob).0, "on se voit à 18h");
        assert_eq!(decrypt_tokens(&out, &eve).1, 0, "les autres ne lisent rien");
    }

    #[test]
    fn marqueur_incomplet_et_message_vide() {
        let k = SymKey::generate();
        // Une parenthèse ouvrante sans fermeture n'est pas un segment : tout
        // le message est chiffré, rien ne part en clair par erreur.
        assert!(!has_marked_segment("prix ((à confirmer"));
        assert_eq!(
            encrypt_message("prix ((à confirmer", std::slice::from_ref(&k))
                .unwrap()
                .1,
            Sealed::Whole
        );
        assert_eq!(
            encrypt_message("   ", std::slice::from_ref(&k)).unwrap().1,
            Sealed::Nothing
        );
        assert!(matches!(encrypt_message("x", &[]), Err(CryptoError::NoKey)));
    }
}
