//! Brocolis 🥦 — bibliothèque du proxy MITM de chiffrement sélectif.
//!
//! Le cœur est exposé sous forme de bibliothèque pour être réutilisé par
//! plusieurs frontends : la CLI (`src/main.rs`) et l'application de bureau
//! (`app/`, paquet `brocolis-app`).

pub mod agreement;
pub mod ca;
pub mod config;
pub mod contacts;
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
        // Destinataires choisis (`encrypt_to`) : la clé active de chacun.
        let km = match cfg.encryption_targets()? {
            Some(targets) => {
                let keys = targets
                    .iter()
                    .map(|b64| SymKey::from_base64(b64))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| anyhow::anyhow!("clé de destinataire invalide: {e}"))?;
                tracing::info!("Chiffrement pour : {}", cfg.encrypt_to.join(", "));
                km.with_recipients(keys)
            }
            None => km,
        };
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
