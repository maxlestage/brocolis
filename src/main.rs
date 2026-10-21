//! Brocolis 🥦 — interface en ligne de commande.
//!
//! Sous-commandes :
//! - `brocolis run`        : démarre le proxy.
//! - `brocolis auto`       : tout-en-un — configure le système, démarre le
//!   proxy, restaure tout à l'arrêt.
//! - `brocolis on` / `off` / `status` : interrupteur, pour un proxy qui tourne
//!   en service (l'état d'origine est mémorisé sur disque entre les deux).
//! - `brocolis keygen`     : génère une clé symétrique AES-256 partageable.
//! - `brocolis encrypt` / `decrypt` : la transformation du proxy, hors ligne.
//! - `brocolis contacts`   : les contacts et la protection de chacun.
//! - `brocolis protect` / `unprotect` : une clé protégée par une phrase
//!   secrète (`[[BROCOLIS-CLE:v2:…]]`, Argon2id), comme la messagerie.
//! - `brocolis pair`       : appairage automatique X25519 à deux (rendez-vous),
//!   en double cliquet (protocole v3) avec un contact nommé.
//! - `brocolis pair-group` : appairage de groupe (clé de groupe distribuée).
//! - `brocolis ca`         : génère/exporte le certificat racine à installer
//!   dans le navigateur.
//!
//! L'application de bureau (`app/`, `cargo run -p brocolis-app`) offre les
//! mêmes fonctions en quelques clics, et appelle cette ligne de commande pour
//! `on` / `off`, qui exigent les droits administrateur.

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
    /// Chiffre les segments `((...))` d'un texte, comme le ferait le proxy.
    ///
    /// Sert à vérifier une configuration sans monter de navigateur, et à
    /// préparer un message à coller ailleurs.
    Encrypt {
        /// Texte à traiter. Absent, le texte est lu sur l'entrée standard.
        #[arg(long)]
        text: Option<String>,
        /// Fichier de configuration TOML (pour le trousseau).
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
        /// Clé base64 (surcharge la configuration).
        #[arg(long)]
        key: Option<String>,
        /// Chiffrer pour ce contact (répétable). Pour un seul contact appairé,
        /// le jeton part en double cliquet (v3) ; sinon v1/v2.
        #[arg(long = "to", conflicts_with = "key")]
        to: Vec<String>,
        /// Chiffrer pour ce groupe : à cliquet (Sender Keys) si chaque membre
        /// est appairé, sinon un jeton v2 pour leurs clés.
        #[arg(long, conflicts_with_all = ["key", "to"])]
        group: Option<String>,
    },
    /// Déchiffre les jetons `[[BROCOLIS:…]]` d'un texte, comme le ferait le proxy.
    Decrypt {
        /// Texte à traiter. Absent, le texte est lu sur l'entrée standard.
        #[arg(long)]
        text: Option<String>,
        /// Fichier de configuration TOML (pour le trousseau).
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
        /// Clé base64 (surcharge la configuration).
        #[arg(long)]
        key: Option<String>,
    },
    /// Liste les contacts : empreinte de leur clé et protection du prochain
    /// envoi (double cliquet ou ancien format).
    Contacts {
        /// Fichier de configuration TOML.
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
    },
    /// Crée un groupe à cliquet avec des contacts appairés, ou le rejoint
    /// avec l'identifiant que son créateur affiche (`--id`).
    GroupCreate {
        /// Nom du groupe.
        #[arg(long)]
        name: String,
        /// Les autres membres (noms de contacts, séparés par des virgules).
        #[arg(long, value_delimiter = ',', required = true)]
        members: Vec<String>,
        /// L'identifiant du groupe, pour le rejoindre.
        #[arg(long)]
        id: Option<String>,
        /// Fichier de configuration TOML.
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
    },
    /// Change les membres d'un groupe : qui en sort ne lit plus la suite.
    GroupMembers {
        /// Nom du groupe.
        #[arg(long)]
        name: String,
        /// Les autres membres, désormais.
        #[arg(long, value_delimiter = ',', required = true)]
        members: Vec<String>,
        /// Fichier de configuration TOML.
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
    },
    /// « J'ai comparé l'empreinte » : après un ré-appairage qui a changé
    /// l'identité d'un contact, l'envoi vers lui est bloqué jusqu'à cette
    /// confirmation.
    ConfirmIdentity {
        /// Le contact.
        #[arg(long)]
        name: String,
        /// Fichier de configuration TOML.
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
    },
    /// Protège une clé par une phrase secrète : `[[BROCOLIS-CLE:v2:…]]`
    /// (Argon2id), à transmettre sans craindre qu'on la lise en chemin.
    ///
    /// La phrase (8 caractères au moins) est lue dans `BROCOLIS_PHRASE`, sinon
    /// sur la première ligne de l'entrée standard.
    Protect {
        /// La clé à protéger (base64).
        #[arg(long, conflicts_with = "contact")]
        key: Option<String>,
        /// Ou : la clé de ce contact.
        #[arg(long)]
        contact: Option<String>,
        /// Fichier de configuration TOML (pour `--contact`).
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
    },
    /// Retrouve la clé d'un `[[BROCOLIS-CLE:v1:…]]` ou `v2`, avec sa phrase
    /// (lue dans `BROCOLIS_PHRASE`, sinon sur l'entrée standard). Affiche la
    /// clé, ou en fait un contact avec `--name`.
    Unprotect {
        /// Le texte qui contient la clé protégée.
        #[arg(long)]
        text: String,
        /// Ajouter la clé comme ce contact.
        #[arg(long)]
        name: Option<String>,
        /// Fichier de configuration TOML (pour `--name`).
        #[arg(long, default_value = "brocolis.toml")]
        config: PathBuf,
    },
    /// Appairage automatique X25519 via le serveur de rendez-vous : échange les
    /// clés publiques avec un correspondant partageant le même CODE, dérive la
    /// clé partagée et l'ajoute au trousseau.
    ///
    /// Avec `--name`, la clé devient ce contact et les paquets de clés v3 sont
    /// échangés : ce qu'on lui envoie part en double cliquet (comme la
    /// messagerie). Sans, elle rejoint les clés sans nom (`keys`), en ancien
    /// format.
    Pair {
        /// Code court convenu de vive voix avec le correspondant.
        #[arg(long)]
        code: String,
        /// Nom du contact à créer (double cliquet).
        #[arg(long)]
        name: Option<String>,
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
    /// Active le chiffrement : dirige le système vers le proxy et fait
    /// confiance au CA. L'état d'origine est mémorisé pour `off`.
    ///
    /// À la différence de `auto`, la commande rend la main : rien à laisser
    /// ouvert. Le proxy lui-même tourne en service (LaunchDaemon sur macOS,
    /// systemd sur Linux), installé une fois pour toutes.
    On {
        /// Fichier de configuration TOML. Par défaut, celle du service :
        /// `/etc/brocolis/brocolis.toml` (macOS, Linux) ou
        /// `%ProgramData%\Brocolis\brocolis.toml` (Windows).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Où mémoriser l'état d'origine (relu par `off`).
        #[arg(long)]
        state: Option<PathBuf>,
    },
    /// Coupe le chiffrement et remet le système exactement dans l'état
    /// mémorisé par `on`. Le service, lui, continue de tourner.
    Off {
        /// Emplacement de l'état mémorisé par `on`.
        #[arg(long)]
        state: Option<PathBuf>,
    },
    /// Indique si le chiffrement est actif, et depuis quel état.
    Status {
        /// Emplacement de l'état mémorisé par `on`.
        #[arg(long)]
        state: Option<PathBuf>,
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
        // Les journaux sur stderr : stdout ne porte que le résultat
        // (`encrypt`, `decrypt`, `keygen` s'utilisent dans un tube).
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Keygen => {
            let key = SymKey::generate();
            // La clé SEULE sur stdout : c'est ce qui rend `key=$(brocolis
            // keygen)` et `brocolis keygen > cle.txt` utilisables. Les conseils
            // partent sur stderr, donc restent visibles à l'écran.
            println!("{}", key.to_base64());
            eprintln!("# Partagez cette clé (canal sûr) avec les utilisateurs autorisés,");
            eprintln!("# puis placez-la dans brocolis.toml (champ `key`).");
            Ok(())
        }
        Command::Encrypt {
            text,
            config,
            key,
            to,
            group,
        } => {
            let entree = read_text(text)?;
            let (sortie, n) = if let Some(g) = group {
                let cfg = Config::load(&config)?;
                let g = cfg
                    .group(&g)
                    .ok_or_else(|| anyhow::anyhow!("groupe « {g} » inconnu"))?;
                let v3 = brocolis::v3store::V3::for_config(&cfg, &config);
                eprintln!(
                    "# {}",
                    if v3.group_ratchet(g) {
                        "groupe à cliquet (Sender Keys)"
                    } else {
                        "groupe, ancien format (un membre sans session à deux)"
                    }
                );
                v3.encrypt_marked_group(&entree, g)
            } else if to.is_empty() {
                let keys = keys_for_tool(&config, key.as_deref())?;
                brocolis::crypto::encrypt_marked_multi(&entree, &keys)
            } else {
                let cfg = Config::load(&config)?;
                let v3 = brocolis::v3store::V3::for_config(&cfg, &config);
                let mut keys = Vec::new();
                for name in &to {
                    let r = brocolis::v3store::find(&v3.recipients, name)
                        .ok_or_else(|| anyhow::anyhow!("contact « {name} » inconnu"))?;
                    keys.push(r.key.clone());
                }
                eprintln!("# {}", v3.protection(&keys).label());
                v3.encrypt_marked(&entree, &keys)
            };
            print!("{sortie}");
            if !sortie.ends_with('\n') {
                println!();
            }
            // Le compte va sur stderr : stdout ne porte que le texte, pour
            // rester utilisable dans un tube.
            eprintln!("# {n} segment(s) chiffré(s)");
            Ok(())
        }
        Command::Decrypt { text, config, key } => {
            let keys = keys_for_tool(&config, key.as_deref())?;
            let entree = read_text(text)?;
            // Avec une clé imposée, seulement v1/v2 : le v3 lit l'état de la
            // configuration.
            let read = if key.is_some() {
                let (text, opened) = brocolis::crypto::decrypt_tokens_multi(&entree, &keys);
                brocolis::v3store::Read {
                    text,
                    opened,
                    ..Default::default()
                }
            } else {
                let cfg = Config::load(&config)?;
                brocolis::v3store::V3::for_config(&cfg, &config).decrypt(&entree, &keys)
            };
            let sortie = &read.text;
            print!("{sortie}");
            if !sortie.ends_with('\n') {
                println!();
            }
            eprintln!("# {} jeton(s) déchiffré(s)", read.opened);
            if !read.v3_from.is_empty() {
                let cliquet = if read.v4 { "triple" } else { "double" };
                eprintln!("# {cliquet} cliquet, de : {}", read.v3_from.join(", "));
            }
            if read.replayed > 0 {
                eprintln!(
                    "# {} jeton(s) v3 déjà lu(s) sur cet appareil (un message à double \
                     cliquet ne s'ouvre qu'une fois)",
                    read.replayed
                );
            }
            Ok(())
        }
        Command::Contacts { config } => run_contacts(&config),
        Command::GroupCreate {
            name,
            members,
            id,
            config,
        } => {
            let existing = std::fs::read_to_string(&config).unwrap_or_default();
            let (updated, id) =
                brocolis::contacts::add_group(&existing, &name, id.as_deref(), &members)
                    .map_err(|e| anyhow::anyhow!(e))?;
            std::fs::write(&config, updated)?;
            println!("✅ Groupe « {} » : {}", name.trim(), members.join(", "));
            println!("   Identifiant : {id}");
            println!(
                "   Chaque membre le rejoint avec `brocolis group-create --name … --members … --id {id}`."
            );
            Ok(())
        }
        Command::GroupMembers {
            name,
            members,
            config,
        } => {
            let existing = std::fs::read_to_string(&config)?;
            let updated = brocolis::contacts::set_group_members(&existing, &name, &members)
                .map_err(|e| anyhow::anyhow!(e))?;
            std::fs::write(&config, updated)?;
            println!("✅ Groupe « {} » : {}", name.trim(), members.join(", "));
            Ok(())
        }
        Command::ConfirmIdentity { name, config } => {
            let existing = std::fs::read_to_string(&config)?;
            let updated = brocolis::contacts::confirm_identity(&existing, &name)
                .map_err(|e| anyhow::anyhow!(e))?;
            std::fs::write(&config, updated)?;
            println!(
                "✅ Identité de « {} » confirmée : l'envoi reprend.",
                name.trim()
            );
            Ok(())
        }
        Command::Protect {
            key,
            contact,
            config,
        } => run_protect(key, contact, &config),
        Command::Unprotect { text, name, config } => run_unprotect(&text, name, &config),
        Command::On { config, state } => run_on(
            &config.unwrap_or_else(brocolis::system::default_config_path),
            state,
        ),
        Command::Off { state } => run_off(state),
        Command::Status { state } => run_status(state),
        Command::Pair {
            code,
            name,
            server,
            config,
            identity,
            timeout,
        } => run_pair(&code, name.as_deref(), &server, &config, &identity, timeout),
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
    let (state, keys) = brocolis::build_app_state_v3(cfg, &config_path)?;

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
    let (state, keys) = brocolis::build_app_state_v3(cfg, &config_path)?;

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
/// Emplacement de l'état mémorisé, surchargeable pour les tests.
fn chemin_etat(state: Option<PathBuf>) -> PathBuf {
    state.unwrap_or_else(brocolis::system::default_state_path)
}

/// `on` — dirige le système vers le proxy et mémorise l'état d'avant.
fn run_on(config: &Path, state: Option<PathBuf>) -> anyhow::Result<()> {
    use brocolis::system::SystemProxy;
    let chemin = chemin_etat(state);

    if chemin.exists() {
        println!("Déjà actif (état mémorisé dans {}).", chemin.display());
        println!("Rien à faire — `brocolis off` pour couper.");
        return Ok(());
    }

    let cfg = Config::load(config)?;
    let port = cfg.listen.port();

    // Le CA doit exister AVANT d'engager : c'est lui qu'on dépose dans le
    // magasin de confiance. Sans cela, une installation neuve échouait sur un
    // « No such file or directory » incompréhensible. `auto` fait de même.
    brocolis::ca::Ca::load_or_generate(&cfg.ca_cert, &cfg.ca_key)
        .map_err(|e| anyhow::anyhow!("CA indisponible ({}) : {e}", cfg.ca_cert.display()))?;

    match SystemProxy::engage(&cfg.ca_cert, port)? {
        Some(sp) => {
            // Mémorisé AVANT d'annoncer le succès : sans cet état, `off` ne
            // saurait rien restaurer et le système resterait détourné.
            sp.save_state(&chemin)?;
            println!("✅ Chiffrement actif (proxy 127.0.0.1:{port}).");
            println!("   `brocolis off` pour couper, `brocolis status` pour vérifier.");
        }
        None => {
            println!("Rien à activer : configuration système non prise en charge ici,");
            println!("ou droits administrateur manquants (essayez avec sudo).");
        }
    }
    Ok(())
}

/// `off` — restaure exactement ce que `on` avait mémorisé.
fn run_off(state: Option<PathBuf>) -> anyhow::Result<()> {
    use brocolis::system::SystemProxy;
    let chemin = chemin_etat(state);

    match SystemProxy::load_state(&chemin)? {
        Some(sp) => {
            sp.revert();
            // L'état n'est retiré qu'APRÈS restauration : si celle-ci échoue,
            // le fichier reste et l'opération peut être retentée.
            std::fs::remove_file(&chemin)?;
            println!("✅ Chiffrement coupé, état d'origine restauré.");
        }
        None => {
            println!(
                "Déjà inactif : aucun état mémorisé dans {}.",
                chemin.display()
            );
        }
    }
    Ok(())
}

/// `status` — dit simplement si c'est actif, sans rien modifier.
fn run_status(state: Option<PathBuf>) -> anyhow::Result<()> {
    let chemin = chemin_etat(state);
    if chemin.exists() {
        println!("actif");
        println!("   état d'origine mémorisé dans {}", chemin.display());
    } else {
        println!("inactif");
    }
    Ok(())
}

/// Texte à traiter : l'option `--text`, sinon l'entrée standard.
fn read_text(text: Option<String>) -> anyhow::Result<String> {
    match text {
        Some(t) => Ok(t),
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

/// Trousseau utilisé par `encrypt`/`decrypt`.
///
/// `--key` l'emporte ; sinon on prend celui de la configuration. Contrairement
/// au proxy, on NE génère PAS de clé éphémère en dernier recours : chiffrer
/// avec une clé jetable que personne ne possède ne rendrait service à personne,
/// et donnerait l'illusion d'avoir protégé quelque chose.
fn keys_for_tool(config: &Path, key: Option<&str>) -> anyhow::Result<Vec<SymKey>> {
    if let Some(k) = key {
        return Ok(vec![
            SymKey::from_base64(k).map_err(|e| anyhow::anyhow!("clé fournie invalide : {e}"))?
        ]);
    }
    let cfg = Config::load(config)?;
    let b64 = cfg.all_key_b64();
    if b64.is_empty() {
        anyhow::bail!(
            "aucune clé : renseignez `key` dans {} ou passez --key",
            config.display()
        );
    }
    b64.iter()
        .map(|k| {
            SymKey::from_base64(k)
                .map_err(|e| anyhow::anyhow!("clé invalide dans la configuration : {e}"))
        })
        .collect()
}

fn load_or_create_identity(path: &Path) -> anyhow::Result<brocolis::agreement::PrivateKey> {
    brocolis::agreement::load_or_create_identity(path)
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

/// Appairage automatique : publie ma clé publique (et, pour un contact
/// nommé, mon paquet de clés v3) sous `code`, attend le correspondant, dérive
/// la clé partagée et l'ajoute au trousseau.
#[tokio::main]
async fn run_pair(
    code: &str,
    name: Option<&str>,
    server: &str,
    config: &Path,
    identity: &Path,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    use brocolis::pairing::RendezvousClient;
    use std::time::Duration;

    // Le nom est vérifié avant d'aller sur le réseau : l'autre attendrait
    // pour rien.
    // Un contact déjà appairé (avec paquet) peut l'être de nouveau : son
    // identité est alors comparée à l'ancienne (§9.8).
    let mut repair = false;
    if let Some(n) = name {
        if let Some(c) = Config::load(config)?.contact(n) {
            if c.bundle.is_none() {
                anyhow::bail!("un contact « {} » existe déjà", n.trim());
            }
            repair = true;
        }
    }
    let priv_key = load_or_create_identity(identity)?;
    let bundle = match name {
        Some(_) => Some(brocolis::v3store::V3Store::for_config(config).published()?),
        None => None,
    };
    let client = RendezvousClient::new(server)?;
    tracing::info!("Appairage sur {server}…");
    let paired = brocolis::pairing::pair_two(
        &client,
        code,
        &priv_key,
        bundle.as_ref(),
        Duration::from_secs(timeout_secs),
    )
    .await?;

    println!("✅ Appairé.");
    println!("   Empreinte : {}", paired.fingerprint);
    println!("   ⚠️  Comparez-la de vive voix avec votre correspondant avant tout secret.");
    println!("   Clé partagée (base64) : {}", paired.key_b64);

    let Some(name) = name else {
        add_key_to_config(config, &paired.key_b64)?;
        println!(
            "   Clé ajoutée à {} (champ `keys`). Redémarrez brocolis pour l'activer.",
            config.display()
        );
        println!("   Avec --name, elle deviendrait un contact en double cliquet.");
        return Ok(());
    };
    let existing = std::fs::read_to_string(config).unwrap_or_default();
    let mut changed = false;
    let updated = match &paired.bundle {
        Some(b) if repair => brocolis::contacts::repair_paired(
            &existing,
            name,
            &paired.key_b64,
            b,
            paired.caps.as_deref(),
            brocolis::v3store::now(),
        )
        .map(|(out, c)| {
            changed = c;
            out
        }),
        None if repair => Err(format!(
            "« {} » n'a pas publié de paquet : contact laissé tel quel",
            name.trim()
        )),
        Some(b) => brocolis::contacts::add_paired(
            &existing,
            name,
            &paired.key_b64,
            b,
            paired.caps.as_deref(),
            brocolis::v3store::now(),
        ),
        None => brocolis::contacts::add(&existing, name, &paired.key_b64),
    }
    .map_err(|e| anyhow::anyhow!("écriture du contact dans {}: {e}", config.display()))?;
    std::fs::write(config, updated)?;
    let label = if paired.caps.is_some() {
        "🔐 triple cliquet"
    } else if paired.bundle.is_some() {
        "🔐 double cliquet"
    } else {
        "ancien format (le correspondant n'a pas publié de paquet v3)"
    };
    println!(
        "   Contact « {} » {} {} : {label}.",
        name.trim(),
        if repair {
            "mis à jour dans"
        } else {
            "ajouté à"
        },
        config.display()
    );
    if changed {
        println!(
            "   ⚠️  L'identité de ce contact a changé : rien ne lui sera envoyé avant\n      `brocolis confirm-identity --name {}` (comparez d'abord l'empreinte).",
            name.trim()
        );
    }
    println!("   Redémarrez brocolis pour l'activer.");
    Ok(())
}

/// `contacts` — chaque contact, l'empreinte de sa clé et sa protection.
fn run_contacts(config: &Path) -> anyhow::Result<()> {
    let cfg = Config::load(config)?;
    let v3 = brocolis::v3store::V3::for_config(&cfg, config);
    let named: Vec<_> = v3
        .recipients
        .iter()
        .filter(|r| r.name.is_some())
        .cloned()
        .collect();
    if named.is_empty() {
        println!("Aucun contact dans {}.", config.display());
        return Ok(());
    }
    let protections = brocolis::v3store::protections(&v3.store, &named);
    for (r, p) in named.iter().zip(protections) {
        println!(
            "{}\t{}\t{}",
            r.name.as_deref().unwrap_or_default(),
            brocolis::agreement::group_fingerprint(&r.key.to_base64()),
            p.label()
        );
    }
    Ok(())
}

/// La phrase secrète : `BROCOLIS_PHRASE`, sinon la première ligne de
/// l'entrée standard (sans son saut de ligne).
fn read_phrase() -> anyhow::Result<String> {
    if let Ok(p) = std::env::var("BROCOLIS_PHRASE") {
        return Ok(p);
    }
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}

/// `protect` — `[[BROCOLIS-CLE:v2:…]]` (Argon2id, 64 Mio, 3 passes).
fn run_protect(key: Option<String>, contact: Option<String>, config: &Path) -> anyhow::Result<()> {
    let b64 = match (key, contact) {
        (Some(k), _) => k,
        (None, Some(name)) => {
            let cfg = Config::load(config)?;
            let c = cfg
                .contact(&name)
                .ok_or_else(|| anyhow::anyhow!("contact « {name} » inconnu"))?;
            c.key()
                .ok_or_else(|| anyhow::anyhow!("le contact « {name} » n'a aucune clé"))?
                .to_owned()
        }
        (None, None) => anyhow::bail!("indiquez --key ou --contact"),
    };
    let key = SymKey::from_base64(&b64).map_err(|e| anyhow::anyhow!("clé invalide : {e}"))?;
    let phrase = read_phrase()?;
    let protected = brocolis_proto::cle_protegee::protect_key(
        &key.bytes(),
        &phrase,
        &mut brocolis_proto::hasard::system(),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{protected}");
    eprintln!("# Transmettez la phrase par un autre canal que la clé protégée.");
    Ok(())
}

/// `unprotect` — la clé d'un `[[BROCOLIS-CLE:…]]`, avec sa phrase.
fn run_unprotect(text: &str, name: Option<String>, config: &Path) -> anyhow::Result<()> {
    let phrase = read_phrase()?;
    let key = brocolis_proto::cle_protegee::unprotect_key(text, &phrase)
        .map_err(|e| anyhow::anyhow!("clé protégée illisible : {e}"))?;
    let b64 = SymKey::from_bytes(&key[..])
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .to_base64();
    match name {
        None => println!("{b64}"),
        Some(name) => {
            let existing = std::fs::read_to_string(config).unwrap_or_default();
            let updated = brocolis::contacts::add(&existing, &name, &b64)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            std::fs::write(config, updated)?;
            println!("Contact « {} » ajouté à {}.", name.trim(), config.display());
        }
    }
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

    // Tout le monde publie sa clé publique sous le code. Pas de paquet v3 :
    // une clé de groupe reste en v1/v2, et sans paquet l'empreinte reste
    // celle du groupe des deux côtés (§8.4).
    let mut peers = client.publish(code, &peer_id, &my_pub, None).await?;
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
