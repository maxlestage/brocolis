//! Brocolis 🥦 — interface en ligne de commande.
//!
//! Sous-commandes :
//! - `brocolis run`        : démarre le proxy.
//! - `brocolis keygen`     : génère une clé symétrique AES-256 partageable.
//! - `brocolis pair`       : appairage automatique X25519 à deux (rendez-vous).
//! - `brocolis pair-group` : appairage de groupe (clé de groupe distribuée).
//! - `brocolis ca`         : génère/exporte le certificat racine à installer
//!   dans le navigateur.
//!
//! L'interface graphique équivalente est `brocolis-gui`
//! (`cargo run --features gui --bin brocolis-gui`).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::sync::Arc;

use clap::{Parser, Subcommand};

use brocolis::ca::Ca;
use brocolis::config::Config;
use brocolis::crypto::SymKey;
#[cfg(unix)]
use brocolis::keys::KeyManager;

#[derive(Parser)]
#[command(
    name = "brocolis",
    version,
    about = "Proxy MITM de chiffrement sélectif"
)]
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
    /// Tout-en-un : approuve le CA, règle le proxy système, démarre le
    /// proxy, et restaure l'état initial à l'arrêt (macOS : `sudo` requis).
    Auto {
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
        /// Ne pas toucher au système (équivaut à `run`).
        #[arg(long)]
        no_system: bool,
    },
    /// Génère une nouvelle clé symétrique AES-256 (base64) à partager.
    Keygen,
    /// Appairage automatique X25519 via le serveur de rendez-vous : échange les
    /// clés publiques avec un correspondant partageant le même CODE, dérive la
    /// clé partagée et l'ajoute au trousseau (`keys` de brocolis.toml).
    Pair {
        /// Code court convenu de vive voix avec le correspondant.
        #[arg(long)]
        code: String,
        /// URL du serveur de rendez-vous.
        #[arg(long, default_value_t = brocolis::pairing::DEFAULT_SERVER.to_string())]
        server: String,
        /// Fichier de configuration TOML où ajouter la clé dérivée.
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
        /// Fichier de la clé privée X25519 locale (créé s'il n'existe pas).
        #[arg(long, default_value = "brocolis-x25519.key")]
        identity: PathBuf,
        /// Délai max d'attente du correspondant, en secondes.
        #[arg(long, default_value_t = 120)]
        timeout: u64,
    },
    /// Appairage de GROUPE (multi-membres) via le serveur de rendez-vous : un
    /// fondateur (`--found`) génère une clé de groupe et la distribue, emballée
    /// X25519, à chaque membre appairé ; les membres la récupèrent. Toute la
    /// clé est ajoutée au trousseau — le groupe lit/écrit sans recopier de clé.
    PairGroup {
        /// Code court convenu de vive voix avec le groupe.
        #[arg(long)]
        code: String,
        /// URL du serveur de rendez-vous.
        #[arg(long, default_value_t = brocolis::pairing::DEFAULT_SERVER.to_string())]
        server: String,
        /// Fichier de configuration TOML où ajouter la clé de groupe.
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
        /// Fichier de la clé privée X25519 locale (créé s'il n'existe pas).
        #[arg(long, default_value = "brocolis-x25519.key")]
        identity: PathBuf,
        /// Mode fondateur : génère la clé de groupe et la distribue aux membres.
        #[arg(long)]
        found: bool,
        /// Délai max (secondes) : côté fondateur, fenêtre de rassemblement des
        /// membres ; côté membre, attente de la clé de groupe.
        #[arg(long, default_value_t = 120)]
        timeout: u64,
    },
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
            println!("# Partagez cette clé (canal sûr) avec les utilisateurs autorisés,");
            println!("# puis placez-la dans brocolis.toml (champ `key`).");
            Ok(())
        }
        Command::Pair {
            code,
            server,
            config,
            identity,
            timeout,
        } => run_pair(&code, &server, &config, &identity, timeout),
        Command::PairGroup {
            code,
            server,
            config,
            identity,
            found,
            timeout,
        } => run_pair_group(&code, &server, &config, &identity, found, timeout),
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
            println!("CA prêt: {} / {}", ca_cert.display(), ca_key.display());
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
        Command::Auto {
            config,
            listen,
            key,
            ca_cert,
            ca_key,
            encrypt_host,
            no_system,
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
            run_auto(cfg, config, !no_system)
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
    let listen = cfg.listen;
    let (state, keys) = brocolis::build_app_state(cfg)?;

    #[cfg(unix)]
    spawn_key_signal_handlers(keys.clone(), config_path);
    #[cfg(not(unix))]
    let _ = (keys, config_path);

    brocolis::proxy::run(state, listen).await
}

/// Future qui se résout au premier `Ctrl-C` / `SIGTERM`.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut int = signal(SignalKind::interrupt()).expect("SIGINT");
        let mut term = signal(SignalKind::terminate()).expect("SIGTERM");
        tokio::select! {
            _ = int.recv() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Commande tout-en-un : configure le système, démarre le proxy, et restaure
/// l'état initial à l'arrêt (Ctrl-C).
#[tokio::main]
async fn run_auto(cfg: Config, config_path: PathBuf, system: bool) -> anyhow::Result<()> {
    let listen = cfg.listen;
    let port = listen.port();
    let ca_cert = cfg.ca_cert.clone();
    let (state, keys) = brocolis::build_app_state(cfg)?;

    #[cfg(unix)]
    spawn_key_signal_handlers(keys.clone(), config_path);
    #[cfg(not(unix))]
    let _ = (keys, config_path);

    let sys = if system {
        brocolis::system::SystemProxy::engage(&ca_cert, port)?
    } else {
        None
    };

    tracing::info!("Ctrl-C pour arrêter et restaurer la configuration");
    let result = brocolis::proxy::run_until(state, listen, shutdown_signal()).await;

    if let Some(sys) = sys {
        sys.revert();
    }
    result
}

/// Charge la clé privée X25519 locale depuis `path`, ou en crée une (persistée
/// en base64, droits `0600` sous Unix).
fn load_or_create_identity(path: &Path) -> anyhow::Result<brocolis::agreement::PrivateKey> {
    use brocolis::agreement::PrivateKey;

    if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        return PrivateKey::from_base64(raw.trim()).map_err(|e| {
            anyhow::anyhow!("clé d'identité X25519 invalide ({}): {e}", path.display())
        });
    }
    let key = PrivateKey::generate();
    std::fs::write(path, key.to_base64())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    tracing::info!("Nouvelle identité X25519 créée: {}", path.display());
    Ok(key)
}

/// Ajoute une clé base64 au champ `keys` du fichier de configuration (créé si
/// absent), en préservant commentaires et mise en forme.
fn add_key_to_config(path: &Path, key_b64: &str) -> anyhow::Result<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let updated = brocolis::pairing::append_key_to_toml(&existing, key_b64)
        .map_err(|e| anyhow::anyhow!("écriture de la clé dans {}: {e}", path.display()))?;
    std::fs::write(path, updated)?;
    Ok(())
}

/// Appairage automatique : publie ma clé publique sous `code`, attend le
/// correspondant, dérive la clé partagée et l'ajoute au trousseau.
#[tokio::main]
async fn run_pair(
    code: &str,
    server: &str,
    config: &Path,
    identity: &Path,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    use brocolis::pairing::{random_peer_id, RendezvousClient};
    use std::time::{Duration, Instant};

    let priv_key = load_or_create_identity(identity)?;
    let my_pub = priv_key.public_key_base64();
    let peer_id = random_peer_id();

    let client = RendezvousClient::new(server)?;

    tracing::info!("Publication de la clé publique (code « {code} ») sur {server}…");
    let mut peers = client.publish(code, &peer_id, &my_pub).await?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while peers.is_empty() {
        if Instant::now() >= deadline {
            anyhow::bail!(
                "aucun correspondant n'a rejoint le code « {code} » dans le délai imparti"
            );
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        peers = client.fetch(code, &peer_id).await?;
    }

    // Appairage à deux : on dérive avec le premier correspondant arrivé.
    let peer = &peers[0];
    let derived = priv_key
        .derive(&peer.public_key)
        .map_err(|e| anyhow::anyhow!("dérivation de la clé partagée: {e}"))?;

    println!("✅ Appairé.");
    println!("   Empreinte : {}", derived.fingerprint);
    println!("   ⚠️  Comparez-la de vive voix avec votre correspondant avant tout secret.");
    println!(
        "   Clé partagée (base64) : {}",
        derived.symmetric_key_base64
    );

    add_key_to_config(config, &derived.symmetric_key_base64)?;
    println!(
        "   Clé ajoutée à {} (champ `keys`). Redémarrez brocolis pour l'activer.",
        config.display()
    );
    Ok(())
}

/// Appairage de groupe : le fondateur génère une clé de groupe et l'emballe pour
/// chaque membre (X25519) ; chaque membre la récupère et la déballe. La clé est
/// ajoutée au trousseau de chacun.
#[tokio::main]
async fn run_pair_group(
    code: &str,
    server: &str,
    config: &Path,
    identity: &Path,
    found: bool,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    use brocolis::agreement::{generate_group_key_base64, group_fingerprint};
    use brocolis::pairing::{random_peer_id, RendezvousClient, WrapItem};
    use std::time::{Duration, Instant};

    let priv_key = load_or_create_identity(identity)?;
    let my_pub = priv_key.public_key_base64();
    let peer_id = random_peer_id();
    let client = RendezvousClient::new(server)?;

    // Tout le monde publie sa clé publique sous le code.
    let mut peers = client.publish(code, &peer_id, &my_pub).await?;
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    if found {
        // Fondateur : rassemble les membres (jusqu'au délai, ou arrêt anticipé
        // si la liste est stable ~6 s avec au moins un membre), puis distribue.
        tracing::info!("Fondateur : attente des membres (code « {code} »)…");
        let mut stable_since = Instant::now();
        let mut last_count = peers.len();
        loop {
            peers = client.fetch(code, &peer_id).await?;
            if peers.len() != last_count {
                last_count = peers.len();
                stable_since = Instant::now();
            }
            let stable = stable_since.elapsed() >= Duration::from_secs(6);
            if (!peers.is_empty() && stable) || Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        if peers.is_empty() {
            anyhow::bail!("aucun membre n'a rejoint le code « {code} »");
        }

        let group_key = generate_group_key_base64();
        let fp = group_fingerprint(&group_key);

        let mut items = Vec::with_capacity(peers.len());
        for p in &peers {
            match priv_key.wrap_group_key(&group_key, &p.public_key) {
                Ok(blob) => items.push(WrapItem {
                    to_peer_id: p.peer_id.clone(),
                    blob,
                }),
                Err(e) => tracing::warn!("membre {} ignoré (emballage impossible): {e}", p.peer_id),
            }
        }
        let served = items.len();
        client.post_wraps(code, &peer_id, &my_pub, &items).await?;

        println!("✅ Groupe créé pour {served} membre(s).");
        println!("   Empreinte de groupe : {fp}");
        println!("   ⚠️  Comparez-la de vive voix avec tous les membres.");
        add_key_to_config(config, &group_key)?;
        println!(
            "   Clé de groupe ajoutée à {} (champ `keys`). Redémarrez brocolis pour l'activer.",
            config.display()
        );
    } else {
        // Membre : attend que le fondateur ait déposé la clé de groupe emballée.
        let _ = peers; // la liste a juste servi à confirmer la publication
        tracing::info!("Membre : attente de la clé de groupe (code « {code} »)…");
        let inbox = loop {
            if let Some(w) = client.fetch_wrap(code, &peer_id).await? {
                break w;
            }
            if Instant::now() >= deadline {
                anyhow::bail!("le fondateur n'a pas déposé de clé de groupe dans le délai imparti");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        };

        let group_key = priv_key
            .unwrap_group_key(&inbox.blob, &inbox.from_pub)
            .map_err(|e| anyhow::anyhow!("déballage de la clé de groupe: {e}"))?;
        let fp = group_fingerprint(&group_key);

        println!("✅ Clé de groupe reçue.");
        println!("   Empreinte de groupe : {fp}");
        println!("   ⚠️  Comparez-la de vive voix avec les autres membres.");
        add_key_to_config(config, &group_key)?;
        println!(
            "   Clé de groupe ajoutée à {} (champ `keys`). Redémarrez brocolis pour l'activer.",
            config.display()
        );
    }
    Ok(())
}
