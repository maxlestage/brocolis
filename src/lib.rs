//! Brocolis 🥦 — bibliothèque du proxy MITM de chiffrement sélectif.
//!
//! Brocolis s'intercale entre le navigateur et les services en ligne (Twitter,
//! forums, messageries…) et chiffre **uniquement** le contenu que l'utilisateur
//! a explicitement marqué, sans rien changer côté serveur. Un segment
//! `((texte secret))` saisi par l'utilisateur est remplacé, avant l'envoi, par
//! un jeton `[[BROCOLIS:v1:…]]` en AES-256-GCM ; à la réception, ce jeton est
//! redéchiffré vers le clair. Les correspondants qui partagent la clé voient le
//! message, le service distant ne stocke que du chiffré.
//!
//! Le cœur est exposé sous forme de bibliothèque pour être réutilisé par
//! plusieurs frontends : la CLI ([`main`](../src/brocolis/main.rs.html)) et une
//! interface graphique optionnelle.
//!
//! # Carte des modules
//!
//! - [`proxy`] : le serveur MITM lui-même (HTTP direct + interception `CONNECT`
//!   / terminaison TLS pour le HTTPS), et les points d'entrée [`run`] /
//!   [`run_until`].
//! - [`crypto`] : chiffrement sélectif AES-256-GCM, format des jetons `v1`/`v2`
//!   et [`SymKey`].
//! - [`keys`] : trousseau [`KeyManager`] (plusieurs clés, rotation, chiffrement
//!   multi-destinataire).
//! - [`ca`] : autorité de certification interne (CA racine + certificats
//!   feuilles forgés à la volée).
//! - [`config`] : chargement de la [`Config`] TOML.
//! - [`agreement`] : accord de clé X25519 (ECDH + HKDF-SHA256), compatible
//!   octet pour octet avec CryptoKit côté iOS.
//! - [`pairing`] : client du serveur de rendez-vous pour l'appairage
//!   automatique (à deux ou en groupe).
//! - [`system`] : intégration système (magasin de confiance, réglages proxy de
//!   l'OS).
//!
//! # Exemple
//!
//! Construire l'état applicatif à partir d'une configuration, puis démarrer le
//! proxy jusqu'à `Ctrl-C` :
//!
//! ```no_run
//! # async fn demo() -> anyhow::Result<()> {
//! use std::path::Path;
//! use brocolis::{build_app_state, run, Config};
//!
//! let cfg = Config::load(Path::new("brocolis.toml"))?;
//! let listen = cfg.listen; // à capturer avant que `cfg` soit déplacée
//! let (state, _keys) = build_app_state(cfg)?;
//! run(state, listen).await?;
//! # Ok(())
//! # }
//! ```
#![deny(missing_docs)]

pub mod agreement;
pub mod ca;
pub mod config;
pub mod crypto;
pub mod keys;
pub mod pairing;
pub mod proxy;
pub mod system;

use std::sync::Arc;
use std::time::Duration;

pub use config::Config;
pub use crypto::SymKey;
pub use keys::KeyManager;
pub use proxy::{run, run_until, AppState};

/// Construit l'état applicatif (CA, gestionnaire de clés, client HTTP) à
/// partir d'une configuration. Renvoie aussi un handle vers le gestionnaire de
/// clés pour permettre la rotation/recharge depuis un frontend.
pub fn build_app_state(cfg: Config) -> anyhow::Result<(Arc<AppState>, Arc<KeyManager>)> {
    // Le fournisseur cryptographique rustls doit être installé AVANT la
    // construction du client reqwest.
    ca::ensure_crypto_provider();

    let ca = Arc::new(ca::Ca::load_or_generate(&cfg.ca_cert, &cfg.ca_key)?);

    let key_b64s = cfg.all_key_b64();
    let keys = Arc::new(if key_b64s.is_empty() {
        let km = KeyManager::ephemeral();
        tracing::warn!(
            "Aucune clé configurée — clé éphémère générée: {}",
            km.current().to_base64()
        );
        tracing::warn!("Le contenu chiffré ne sera lisible que durant cette session.");
        km
    } else {
        let mut parsed = Vec::with_capacity(key_b64s.len());
        for b64 in &key_b64s {
            parsed.push(
                SymKey::from_base64(b64)
                    .map_err(|e| anyhow::anyhow!("clé invalide dans la configuration: {e}"))?,
            );
        }
        let n = parsed.len();
        let km = KeyManager::from_keys(parsed, cfg.encrypt_multi);
        if n > 1 {
            tracing::info!(
                "Trousseau: {n} clés — déchiffrement multi-clés{}.",
                if cfg.encrypt_multi {
                    ", chiffrement v2 multi-destinataire"
                } else {
                    ""
                }
            );
        }
        km
    });

    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()?;

    if cfg.encrypt_hosts.is_empty() {
        tracing::info!("Portée: tous les hôtes");
    } else {
        tracing::info!("Portée: {:?}", cfg.encrypt_hosts);
    }

    let keys_handle = keys.clone();
    let state = Arc::new(AppState {
        ca,
        keys,
        cfg: Arc::new(cfg),
        http,
    });
    Ok((state, keys_handle))
}
