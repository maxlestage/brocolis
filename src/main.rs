//! Brocolis 🥦 — proxy MITM Rust pour le chiffrement sélectif du trafic.
//!
//! Sous-commandes :
//! - `brocolis run`    : démarre le proxy.
//! - `brocolis keygen` : génère une clé symétrique AES-256 partageable.
//! - `brocolis ca`     : génère/exporte le certificat racine à installer dans
//!   le navigateur.

mod ca;
mod config;
mod crypto;
mod keys;
mod proxy;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};

use crate::ca::Ca;
use crate::config::Config;
use crate::crypto::SymKey;
use crate::keys::KeyManager;
use crate::proxy::AppState;

#[derive(Parser)]
#[command(name = "brocolis", version, about = "Proxy MITM de chiffrement sélectif")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Démarre le proxy MITM.
    Run {
        /// Fichier de configuration TOML.
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
        /// Adresse d'écoute (surcharge la config).
        #[arg(long)]
        listen: Option<SocketAddr>,
        /// Clé symétrique base64 (surcharge la config).
        #[arg(long)]
        key: Option<String>,
        /// Chemin du certificat CA (surcharge la config).
        #[arg(long)]
        ca_cert: Option<PathBuf>,
        /// Chemin de la clé CA (surcharge la config).
        #[arg(long)]
        ca_key: Option<PathBuf>,
        /// Hôtes à traiter (répétable ; surcharge la config).
        #[arg(long = "encrypt-host")]
        encrypt_host: Vec<String>,
    },
    /// Génère une nouvelle clé symétrique AES-256 (base64) à partager.
    Keygen,
    /// Génère/charge le CA et l'exporte pour le navigateur.
    Ca {
        #[arg(long, default_value = "brocolis-ca.crt")]
        ca_cert: PathBuf,
        #[arg(long, default_value = "brocolis-ca.key")]
        ca_key: PathBuf,
        /// Écrit le certificat racine (PEM) à ce chemin.
        #[arg(long)]
        export: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Keygen => {
            let key = SymKey::generate();
            println!("{}", key.to_base64());
            println!(
                "# Partagez cette clé (canal sûr) avec les utilisateurs autorisés,"
            );
            println!("# puis placez-la dans brocolis.toml (champ `key`).");
            Ok(())
        }
        Command::Ca {
            ca_cert,
            ca_key,
            export,
        } => {
            let ca = Ca::load_or_generate(&ca_cert, &ca_key)?;
            if let Some(path) = export {
                std::fs::write(&path, ca.root_cert_pem())?;
                println!("Certificat racine exporté: {}", path.display());
            }
            println!(
                "CA prêt: {} / {}",
                ca_cert.display(),
                ca_key.display()
            );
            println!(
                "Installez {} comme autorité de confiance dans votre navigateur.",
                ca_cert.display()
            );
            Ok(())
        }
        Command::Run {
            config,
            listen,
            key,
            ca_cert,
            ca_key,
            encrypt_host,
        } => {
            let mut cfg = Config::load(&config)?;
            if let Some(l) = listen {
                cfg.listen = l;
            }
            if let Some(c) = ca_cert {
                cfg.ca_cert = c;
            }
            if let Some(k) = ca_key {
                cfg.ca_key = k;
            }
            if key.is_some() {
                cfg.key = key;
            }
            if !encrypt_host.is_empty() {
                cfg.encrypt_hosts = encrypt_host;
            }
            run_proxy(cfg, config)
        }
    }
}

/// Écoute les signaux de gestion des clés (README §5) :
/// - `SIGHUP`  : rotation immédiate (clé compromise) ; la nouvelle clé
///   partageable est journalisée.
/// - `SIGUSR1` : rechargement de la clé depuis le fichier de configuration
///   (clé reçue d'un autre utilisateur).
#[cfg(unix)]
fn spawn_key_signal_handlers(keys: Arc<KeyManager>, config_path: PathBuf) {
    use tokio::signal::unix::{signal, SignalKind};

    if let Ok(mut sighup) = signal(SignalKind::hangup()) {
        let keys = keys.clone();
        tokio::spawn(async move {
            while sighup.recv().await.is_some() {
                let fresh = keys.rotate();
                tracing::warn!("SIGHUP: clé régénérée. Nouvelle clé partageable: {fresh}");
            }
        });
    }
    if let Ok(mut sigusr1) = signal(SignalKind::user_defined1()) {
        tokio::spawn(async move {
            while sigusr1.recv().await.is_some() {
                match Config::load(&config_path) {
                    Ok(c) => match c.key {
                        Some(b64) => match keys.set_from_base64(&b64) {
                            Ok(()) => tracing::info!(
                                "SIGUSR1: clé rechargée depuis {}",
                                config_path.display()
                            ),
                            Err(e) => tracing::error!("SIGUSR1: clé invalide: {e}"),
                        },
                        None => tracing::warn!(
                            "SIGUSR1: aucun champ `key` dans {}",
                            config_path.display()
                        ),
                    },
                    Err(e) => tracing::error!("SIGUSR1: lecture config échouée: {e}"),
                }
            }
        });
    }
}

#[tokio::main]
async fn run_proxy(cfg: Config, config_path: PathBuf) -> anyhow::Result<()> {
    // Installe le fournisseur cryptographique rustls AVANT de construire le
    // client reqwest (sinon son magasin de racines TLS n'est pas configuré).
    ca::ensure_crypto_provider();

    let ca = Arc::new(Ca::load_or_generate(&cfg.ca_cert, &cfg.ca_key)?);

    let keys = Arc::new(match &cfg.key {
        Some(b64) => KeyManager::from_base64(b64)
            .map_err(|e| anyhow::anyhow!("clé invalide dans la configuration: {e}"))?,
        None => {
            let km = KeyManager::ephemeral();
            tracing::warn!(
                "Aucune clé configurée — clé éphémère générée: {}",
                km.current().to_base64()
            );
            tracing::warn!(
                "Le contenu chiffré ne sera lisible que durant cette session."
            );
            km
        }
    });

    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()?;

    #[cfg(unix)]
    spawn_key_signal_handlers(keys.clone(), config_path);

    if cfg.encrypt_hosts.is_empty() {
        tracing::info!("Portée: tous les hôtes");
    } else {
        tracing::info!("Portée: {:?}", cfg.encrypt_hosts);
    }

    let listen = cfg.listen;
    let state = Arc::new(AppState {
        ca,
        keys,
        cfg: Arc::new(cfg),
        http,
    });

    proxy::run(state, listen).await
}
