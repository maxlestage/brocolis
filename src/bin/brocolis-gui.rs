//! Brocolis 🥦 — application de bureau (Linux / macOS / Windows).
//!
//! Interface graphique 100 % Rust (eframe/egui) qui pilote le même cœur que la
//! CLI : génération/partage de la clé, génération/export du CA, configuration
//! des hôtes et du port, démarrage/arrêt du proxy et journal en direct.
//!
//! Lancement : `cargo run --features gui --bin brocolis-gui`

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use tokio::sync::oneshot;

use brocolis::config::Config;
use brocolis::crypto::SymKey;

/// Tampon de logs partagé entre le thread du proxy et l'UI.
type LogBuffer = Arc<Mutex<Vec<String>>>;
const MAX_LOG_LINES: usize = 2000;

/// `MakeWriter` qui redirige les traces vers le tampon affiché dans l'UI.
#[derive(Clone)]
struct UiWriter(LogBuffer);

impl std::io::Write for UiWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        let trimmed = text.trim_end_matches('\n');
        if !trimmed.is_empty() {
            let mut g = self.0.lock().expect("log mutex");
            for line in trimmed.split('\n') {
                g.push(line.to_string());
            }
            let len = g.len();
            if len > MAX_LOG_LINES {
                g.drain(0..len - MAX_LOG_LINES);
            }
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Handle du proxy en cours d'exécution (thread + canal d'arrêt).
struct ProxyRuntime {
    stop_tx: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ProxyRuntime {
    fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct BrocolisApp {
    listen: String,
    ca_cert: String,
    ca_key: String,
    key_b64: String,
    encrypt_hosts: String,
    ca_pem: String,
    notice: String,
    logs: LogBuffer,
    running: Arc<AtomicBool>,
    runtime: Option<ProxyRuntime>,
}

impl BrocolisApp {
    fn new(logs: LogBuffer) -> Self {
        Self {
            listen: "127.0.0.1:8080".to_string(),
            ca_cert: "brocolis-ca.crt".to_string(),
            ca_key: "brocolis-ca.key".to_string(),
            key_b64: String::new(),
            encrypt_hosts: "twitter.com, x.com".to_string(),
            ca_pem: String::new(),
            notice: String::new(),
            logs,
            running: Arc::new(AtomicBool::new(false)),
            runtime: None,
        }
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn build_config(&self) -> Result<Config, String> {
        let listen: SocketAddr = self
            .listen
            .trim()
            .parse()
            .map_err(|e| format!("adresse d'écoute invalide: {e}"))?;
        let key = {
            let k = self.key_b64.trim();
            if k.is_empty() {
                None
            } else {
                // Validation immédiate de la clé.
                SymKey::from_base64(k).map_err(|e| format!("clé invalide: {e}"))?;
                Some(k.to_string())
            }
        };
        let encrypt_hosts = self
            .encrypt_hosts
            .split([',', ' ', '\n', '\t'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Ok(Config {
            listen,
            ca_cert: PathBuf::from(self.ca_cert.trim()),
            ca_key: PathBuf::from(self.ca_key.trim()),
            key,
            encrypt_hosts,
        })
    }

    fn start(&mut self) {
        let cfg = match self.build_config() {
            Ok(c) => c,
            Err(e) => {
                self.notice = e;
                return;
            }
        };
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        let running = self.running.clone();
        let thread = std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("création du runtime tokio impossible: {e}");
                    return;
                }
            };
            running.store(true, Ordering::SeqCst);
            let result: anyhow::Result<()> = rt.block_on(async move {
                let listen = cfg.listen;
                let (state, _keys) = brocolis::build_app_state(cfg)?;
                brocolis::proxy::run_until(state, listen, async move {
                    let _ = stop_rx.await;
                })
                .await
            });
            if let Err(e) = result {
                tracing::error!("Proxy arrêté sur erreur: {e:#}");
            }
            running.store(false, Ordering::SeqCst);
        });
        self.runtime = Some(ProxyRuntime {
            stop_tx: Some(stop_tx),
            thread: Some(thread),
        });
        self.notice = "Proxy démarré".to_string();
    }

    fn stop(&mut self) {
        if let Some(mut rt) = self.runtime.take() {
            rt.stop();
        }
        self.notice = "Proxy arrêté".to_string();
    }

    fn generate_key(&mut self) {
        self.key_b64 = SymKey::generate().to_base64();
        self.notice = "Nouvelle clé générée — partagez-la par un canal sûr".to_string();
    }

    fn load_or_generate_ca(&mut self) {
        match brocolis::ca::Ca::load_or_generate(
            Path::new(self.ca_cert.trim()),
            Path::new(self.ca_key.trim()),
        ) {
            Ok(ca) => {
                self.ca_pem = ca.root_cert_pem().to_string();
                self.notice = format!(
                    "CA prêt ({}). Installez ce certificat dans le navigateur.",
                    self.ca_cert.trim()
                );
            }
            Err(e) => self.notice = format!("erreur CA: {e}"),
        }
    }
}

impl eframe::App for BrocolisApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let running = self.is_running();

        egui::TopBottomPanel::top("entete").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Brocolis 🥦");
                ui.separator();
                if running {
                    ui.colored_label(egui::Color32::from_rgb(60, 180, 75), "● En écoute");
                } else {
                    ui.colored_label(egui::Color32::GRAY, "● Arrêté");
                }
            });
        });

        egui::TopBottomPanel::bottom("journal")
            .resizable(true)
            .default_height(200.0)
            .show(ctx, |ui| {
                ui.label("Journal");
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let logs = self.logs.lock().expect("log mutex");
                        for line in logs.iter() {
                            ui.label(egui::RichText::new(line).monospace());
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(!running, |ui| {
                egui::Grid::new("config")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Adresse d'écoute");
                        ui.add(egui::TextEdit::singleline(&mut self.listen));
                        ui.end_row();

                        ui.label("Certificat CA");
                        ui.add(egui::TextEdit::singleline(&mut self.ca_cert));
                        ui.end_row();

                        ui.label("Clé CA");
                        ui.add(egui::TextEdit::singleline(&mut self.ca_key));
                        ui.end_row();

                        ui.label("Hôtes (séparés par virgule)");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.encrypt_hosts)
                                .desired_width(f32::INFINITY),
                        );
                        ui.end_row();

                        ui.label("Clé symétrique (base64)");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.key_b64)
                                    .desired_width(360.0)
                                    .password(false),
                            );
                            if ui.button("Générer").clicked() {
                                self.generate_key();
                            }
                            if ui.button("Copier").clicked() {
                                ui.ctx().copy_text(self.key_b64.clone());
                                self.notice = "Clé copiée dans le presse-papiers".to_string();
                            }
                        });
                        ui.end_row();
                    });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if running {
                    if ui.button("⏹ Arrêter le proxy").clicked() {
                        self.stop();
                    }
                } else if ui.button("▶ Démarrer le proxy").clicked() {
                    self.start();
                }
                if ui.button("Charger / générer le CA").clicked() {
                    self.load_or_generate_ca();
                }
            });

            if !self.notice.is_empty() {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(&self.notice).italics());
            }

            if !self.ca_pem.is_empty() {
                ui.add_space(8.0);
                ui.collapsing("Certificat racine (PEM)", |ui| {
                    if ui.button("Copier le PEM").clicked() {
                        ui.ctx().copy_text(self.ca_pem.clone());
                    }
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.ca_pem.clone())
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                        });
                });
            }
        });

        // Rafraîchit régulièrement pour faire défiler le journal en direct.
        if running {
            ctx.request_repaint_after(Duration::from_millis(400));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(mut rt) = self.runtime.take() {
            rt.stop();
        }
    }
}

fn main() -> eframe::Result<()> {
    let logs: LogBuffer = Arc::new(Mutex::new(Vec::new()));

    let writer_logs = logs.clone();
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(move || UiWriter(writer_logs.clone()))
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Brocolis",
        native_options,
        Box::new(move |_cc| Ok(Box::new(BrocolisApp::new(logs)))),
    )
}
