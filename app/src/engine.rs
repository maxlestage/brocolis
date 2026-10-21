//! Le proxy, lancé dans l'application.
//!
//! Sans service système installé, c'est l'application elle-même qui fait
//! tourner le proxy, le temps qu'elle est ouverte — l'équivalent de
//! `brocolis auto`. Il tourne sur son propre fil d'exécution, avec son propre
//! moteur asynchrone : l'interface ne se fige jamais pendant qu'il travaille.

use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::oneshot;

/// Un proxy en marche. Le lâcher l'arrête.
pub struct Engine {
    pub listen: SocketAddr,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    failure: Arc<Mutex<Option<String>>>,
}

impl Engine {
    /// Démarre le proxy décrit par `config`. Le CA est créé s'il n'existe pas
    /// encore. Échoue tout de suite si le port est déjà pris.
    pub fn start(config: &Path) -> anyhow::Result<Self> {
        let cfg = brocolis::Config::load(config)?;
        let listen = cfg.listen;
        if is_listening(listen) {
            anyhow::bail!("le port {listen} est déjà utilisé");
        }
        let (state, _keys) = brocolis::build_app_state_v3(cfg, config)?;

        let (stop, stopped) = oneshot::channel::<()>();
        let failure = Arc::new(Mutex::new(None));
        let report = failure.clone();
        let thread = std::thread::Builder::new()
            .name("brocolis-proxy".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .map_err(anyhow::Error::from)
                    .and_then(|rt| {
                        rt.block_on(brocolis::run_until(state, listen, async {
                            let _ = stopped.await;
                        }))
                    });
                if let Err(e) = result {
                    *report.lock().expect("verrou du proxy") = Some(e.to_string());
                }
            })?;

        Ok(Self {
            listen,
            stop: Some(stop),
            thread: Some(thread),
            failure,
        })
    }

    /// Attend que le proxy accepte les connexions (au plus `timeout`).
    pub fn wait_ready(&self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if is_listening(self.listen) {
                return true;
            }
            if self.failure().is_some() {
                return false;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        false
    }

    /// Message d'erreur si le proxy s'est arrêté de lui-même.
    pub fn failure(&self) -> Option<String> {
        self.failure.lock().ok()?.clone()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Quelque chose écoute-t-il à cette adresse ? Une connexion locale refusée
/// répond immédiatement ; le délai ne joue que si la pile réseau est saturée.
pub fn is_listening(addr: SocketAddr) -> bool {
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::Files;

    #[test]
    fn demarre_ecoute_puis_s_arrete() {
        let dir = std::env::temp_dir().join(format!("brocolis-app-engine-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let files = Files::at(&dir);
        files.ensure().unwrap();
        // Un port libre, pour ne pas dépendre de ce qui tourne sur la machine.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let toml = std::fs::read_to_string(&files.config)
            .unwrap()
            .replace("127.0.0.1:8080", &format!("127.0.0.1:{port}"));
        std::fs::write(&files.config, toml).unwrap();

        let engine = Engine::start(&files.config).unwrap();
        let addr = engine.listen;
        assert!(
            engine.wait_ready(Duration::from_secs(5)),
            "le proxy écoute sur {addr}"
        );
        assert!(
            files.ca_cert.exists(),
            "le CA est créé au premier démarrage"
        );
        assert!(
            Engine::start(&files.config).is_err(),
            "un second proxy sur le même port est refusé"
        );

        drop(engine);
        assert!(!is_listening(addr), "arrêté et port libéré");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
