//! L'interface : trois onglets — Protection, Contacts, Message.
//!
//! Tout ce qui peut bloquer (demande d'autorisation, appairage réseau) part sur
//! un fil d'exécution à part ; l'interface interroge le résultat à chaque image
//! et ne se fige jamais. L'état du système (protection active ou non, proxy à
//! l'écoute ou non) est relu toutes les secondes : il peut changer en dehors de
//! l'application, par `brocolis on`/`off` dans un terminal.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontFamily, Frame, Layout, Margin, RichText, Sense, Stroke,
    Vec2, ViewportCommand, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::engine::{Engine, is_listening};
use crate::files::Files;
use crate::i18n::{LANGS, Lang, Strings};
use crate::keyring::{self, Book, Entry};

// Palette du site (thème vert) : forêt profonde, crème, citron vert.
pub const BG: Color32 = Color32::from_rgb(0x0f, 0x2a, 0x18);
pub const CARD: Color32 = Color32::from_rgb(0x15, 0x36, 0x1f);
pub const SUNK: Color32 = Color32::from_rgb(0x0a, 0x1d, 0x10);
pub const INK: Color32 = Color32::from_rgb(0xf2, 0xee, 0xdf);
pub const MUTED: Color32 = Color32::from_rgb(0xb8, 0xc6, 0xb4);
pub const ACCENT: Color32 = Color32::from_rgb(0xc8, 0xf2, 0x5c);
pub const ACCENT_INK: Color32 = Color32::from_rgb(0x0f, 0x2a, 0x18);
pub const LINE: Color32 = Color32::from_rgba_premultiplied(0x27, 0x26, 0x24, 0x29);
pub const DANGER: Color32 = Color32::from_rgb(0xff, 0xb4, 0xa6);

/// Lance `brocolis <args…>` avec les droits administrateur. Injecté pour que
/// les tests remplacent la vraie demande d'autorisation.
pub type Elevator = Arc<dyn Fn(&[String]) -> Result<(), String> + Send + Sync>;

pub struct Options {
    pub files: Files,
    /// Où `brocolis on` mémorise l'état d'origine (relu par `off`).
    pub state_path: PathBuf,
    /// Configuration du service système, s'il en tourne un.
    pub service_config: PathBuf,
    pub elevator: Elevator,
    pub lang: Lang,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Protection,
    Contacts,
    Message,
}

/// Le petit champ de saisie ouvert dans une carte.
#[derive(Clone, Debug, PartialEq)]
enum Edit {
    /// Changer la clé de ce contact.
    Rekey(String),
    Rename(String),
    /// Ranger cette clé sans nom sous un contact.
    Assign(String),
}

/// Ce qu'un clic demande, appliqué une fois l'image dessinée.
enum Act {
    Copy(String),
    OwnPrimary(String),
    OwnRemove(String),
    Open(Edit),
    Save,
    CancelEdit,
    AskDelete(String),
    Delete(String),
    CancelDelete,
    /// Double cliquet à partir de la clé partagée : oui ou non.
    Psk(String, bool),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ok,
    Info,
    Err,
}

struct Notice {
    text: String,
    kind: Kind,
}

enum JobKind {
    Activate { started_engine: bool },
    Deactivate { then_quit: bool },
    Pair,
}

enum JobDone {
    Elevated,
    Paired { fingerprint: String },
}

struct Job {
    kind: JobKind,
    rx: Receiver<Result<JobDone, String>>,
}

/// Ce que l'on sait du système, relu régulièrement.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// `brocolis on` a été lancé et pas encore défait.
    pub engaged: bool,
    /// Un proxy écoute à l'adresse configurée.
    pub listening: bool,
}

pub struct BrocolisApp {
    opts: Options,
    pub tab: Tab,
    lang: Lang,
    pub book: Book,
    listen: SocketAddr,
    engine: Option<Engine>,
    pub snapshot: Snapshot,
    last_probe: Option<f64>,
    job: Option<Job>,
    notice: Option<Notice>,
    add_input: String,
    edit: Option<Edit>,
    edit_input: String,
    confirm_delete: Option<String>,
    new_name: String,
    new_key: String,
    pair_name: String,
    pair_code: String,
    pair_fingerprint: Option<String>,
    /// Destinataires du message : vous (clé principale) et des contacts.
    pub send_me: bool,
    pub send_to: Vec<String>,
    message: String,
    output: String,
    output_note: Option<String>,
    /// La fermeture a été refusée faute d'avoir pu couper la protection.
    quit_blocked: bool,
    quit_confirmed: bool,
}

impl BrocolisApp {
    pub fn new(opts: Options) -> Self {
        let _ = opts.files.ensure();
        let listen = brocolis::Config::load(&opts.files.config)
            .map(|c| c.listen)
            .unwrap_or_else(|_| "127.0.0.1:8080".parse().expect("adresse valide"));
        let mut app = Self {
            lang: opts.lang,
            opts,
            tab: Tab::Protection,
            book: Book::default(),
            listen,
            engine: None,
            snapshot: Snapshot::default(),
            last_probe: None,
            job: None,
            notice: None,
            add_input: String::new(),
            edit: None,
            edit_input: String::new(),
            confirm_delete: None,
            new_name: String::new(),
            new_key: String::new(),
            pair_name: String::new(),
            pair_code: String::new(),
            pair_fingerprint: None,
            send_me: true,
            send_to: Vec::new(),
            message: String::new(),
            output: String::new(),
            output_note: None,
            quit_blocked: false,
            quit_confirmed: false,
        };
        app.reload_keys();
        // Par défaut, un message va aux mêmes destinataires que le navigateur.
        if !app.book.encrypt_to.is_empty() {
            app.send_me = false;
            app.send_to = app.book.encrypt_to.clone();
        }
        app.probe();
        // Protection restée active alors que plus aucun proxy n'écoute (session
        // précédente interrompue) : sans proxy, la navigation est coupée. On
        // le relance aussitôt plutôt que de laisser l'utilisateur sans réseau.
        if app.snapshot.engaged && !app.snapshot.listening {
            if let Ok(engine) = Engine::start(&app.opts.files.config) {
                engine.wait_ready(Duration::from_secs(2));
                app.engine = Some(engine);
            }
            app.probe();
        }
        app
    }

    fn t(&self) -> &'static Strings {
        self.lang.t()
    }

    fn probe(&mut self) {
        self.snapshot = Snapshot {
            engaged: self.opts.state_path.exists(),
            listening: is_listening(self.listen),
        };
    }

    fn reload_keys(&mut self) {
        self.book = keyring::load(&self.opts.files.config).unwrap_or_default();
        // Un destinataire supprimé, renommé ou sans clé n'est plus choisi.
        let book = &self.book;
        self.send_to.retain(|n| book.person(n).is_some());
        for n in &mut self.send_to {
            if let Some(p) = book.person(n) {
                n.clone_from(&p.name);
            }
        }
    }

    fn notify(&mut self, kind: Kind, text: impl Into<String>) {
        self.notice = Some(Notice {
            text: text.into(),
            kind,
        });
    }

    /// Le proxy qui tourne n'est pas le nôtre : un service système l'a lancé.
    fn service_running(&self) -> bool {
        self.engine.is_none() && self.snapshot.listening
    }

    fn busy(&self) -> bool {
        self.job.is_some()
    }

    fn spawn<F>(&mut self, kind: JobKind, work: F)
    where
        F: FnOnce() -> Result<JobDone, String> + Send + 'static,
    {
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(work());
        });
        self.job = Some(Job { kind, rx });
    }

    fn elevate(&mut self, kind: JobKind, args: Vec<String>) {
        let elevator = self.opts.elevator.clone();
        self.spawn(kind, move || elevator(&args).map(|()| JobDone::Elevated));
    }

    fn state_args(&self) -> [String; 2] {
        ["--state".into(), self.opts.state_path.display().to_string()]
    }

    pub fn activate(&mut self) {
        if self.busy() {
            return;
        }
        self.probe();
        let mut args = vec!["on".to_string()];
        args.extend(self.state_args());
        let started_engine = if self.service_running() {
            // Le service a son propre CA : c'est SA configuration qu'il faut
            // engager, sinon le système ferait confiance à un autre certificat
            // que celui qui signe réellement.
            args.extend([
                "--config".into(),
                self.opts.service_config.display().to_string(),
            ]);
            false
        } else {
            if self.engine.is_none() {
                match Engine::start(&self.opts.files.config) {
                    Ok(e) => self.engine = Some(e),
                    Err(e) => {
                        let msg = format!("{}{e}", self.t().failed);
                        self.notify(Kind::Err, msg);
                        return;
                    }
                }
            }
            args.extend([
                "--config".into(),
                self.opts.files.config.display().to_string(),
            ]);
            true
        };
        self.notice = None;
        self.elevate(JobKind::Activate { started_engine }, args);
    }

    pub fn deactivate(&mut self, then_quit: bool) {
        if self.busy() {
            return;
        }
        let mut args = vec!["off".to_string()];
        args.extend(self.state_args());
        self.notice = None;
        self.elevate(JobKind::Deactivate { then_quit }, args);
    }

    fn finish_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else { return };
        let result = match job.rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("interrompu".into()),
        };
        let Some(job) = self.job.take() else { return };
        let t = self.t();
        let refused = |e: &str| crate::elevate::is_cancel(e);
        match (job.kind, result) {
            (JobKind::Activate { .. }, Ok(_)) => {}
            (JobKind::Activate { started_engine }, Err(e)) => {
                if started_engine {
                    self.engine = None;
                }
                let msg = if refused(&e) {
                    t.canceled.to_owned()
                } else {
                    format!("{}{e}", t.failed)
                };
                self.notify(if refused(&e) { Kind::Info } else { Kind::Err }, msg);
            }
            (JobKind::Deactivate { then_quit }, Ok(_)) => {
                self.engine = None;
                if then_quit {
                    self.quit_confirmed = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
            (JobKind::Deactivate { then_quit }, Err(e)) => {
                if then_quit {
                    self.quit_blocked = true;
                } else {
                    let msg = if refused(&e) {
                        t.canceled.to_owned()
                    } else {
                        format!("{}{e}", t.failed)
                    };
                    self.notify(if refused(&e) { Kind::Info } else { Kind::Err }, msg);
                }
            }
            (JobKind::Pair, Ok(JobDone::Paired { fingerprint })) => {
                self.pair_fingerprint = Some(fingerprint);
                self.pair_code.clear();
                self.pair_name.clear();
                self.keys_changed();
            }
            (JobKind::Pair, Ok(JobDone::Elevated)) => {}
            (JobKind::Pair, Err(e)) => {
                let msg = format!("{}{e}", t.failed);
                self.notify(Kind::Err, msg);
            }
        }
        self.probe();
    }

    /// Après toute modification du trousseau : relire, et relancer le proxy
    /// de l'application pour qu'il chiffre avec les nouvelles clés.
    fn keys_changed(&mut self) {
        self.reload_keys();
        if self.engine.take().is_some() {
            match Engine::start(&self.opts.files.config) {
                Ok(e) => self.engine = Some(e),
                Err(e) => {
                    let msg = format!("{}{e}", self.t().failed);
                    self.notify(Kind::Err, msg);
                }
            }
        }
    }

    fn start_pairing(&mut self) {
        let code = self.pair_code.trim().to_owned();
        let name = self.pair_name.trim().to_owned();
        if code.is_empty() || name.is_empty() || self.busy() {
            return;
        }
        // Vérifié avant d'aller sur le réseau : l'autre attendrait pour rien.
        if self.book.person(&name).is_some() {
            let msg = self.t().contact_exists.replace("{name}", &name);
            self.notify(Kind::Err, msg);
            return;
        }
        self.pair_fingerprint = None;
        self.notice = None;
        let files = self.opts.files.clone();
        self.spawn(JobKind::Pair, move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            rt.block_on(async {
                let identity = brocolis::agreement::load_or_create_identity(&files.identity)?;
                // Le paquet de clés v3 part avec la clé publique : un
                // correspondant à jour passe en double cliquet (§8.4).
                let bundle = brocolis::v3store::V3Store::for_config(&files.config)
                    .bundle_b64()
                    .ok();
                let client =
                    brocolis::pairing::RendezvousClient::new(brocolis::pairing::DEFAULT_SERVER)?;
                let paired = brocolis::pairing::pair_two(
                    &client,
                    &code,
                    &identity,
                    bundle.as_deref(),
                    Duration::from_secs(120),
                )
                .await?;
                keyring::contact_add_paired(
                    &files.config,
                    &name,
                    &paired.key_b64,
                    paired.bundle.as_deref(),
                )?;
                anyhow::Ok(JobDone::Paired {
                    fingerprint: paired.fingerprint,
                })
            })
            .map_err(|e| e.to_string())
        });
    }

    // ------------------------------------------------------------------
    // Rendu
    // ------------------------------------------------------------------

    /// Une image complète. Appelé par eframe, et par les tests.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.finish_job(&ctx);
        if self.last_probe.is_none_or(|t| now - t > 1.0) {
            self.last_probe = Some(now);
            if !self.busy() {
                self.probe();
            }
        }
        if let Some(engine) = &self.engine
            && let Some(err) = engine.failure()
        {
            self.engine = None;
            let msg = format!("{}{err}", self.t().failed);
            self.notify(Kind::Err, msg);
        }
        self.guard_close(&ctx);

        // Pied de fenêtre (messages, signature) dans un panneau à part : il
        // garde sa place en bas quoi qu'il arrive au contenu au-dessus.
        egui::Panel::bottom("pied")
            .frame(Frame::new().fill(BG).inner_margin(Margin {
                left: 22,
                right: 22,
                top: 6,
                bottom: 12,
            }))
            .show_separator_line(false)
            .resizable(false)
            .show(ui, |ui| {
                self.notice_bar(ui);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Brocolis {} · {}",
                            env!("CARGO_PKG_VERSION"),
                            self.t().footer
                        ))
                        .size(11.5)
                        .color(MUTED),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(22, 18)),
            )
            .show(ui, |ui| {
                self.header(ui);
                ui.add_space(14.0);
                self.tabs(ui);
                ui.add_space(14.0);
                if self.quit_blocked {
                    self.quit_dialog(ui);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        Tab::Protection => self.protection(ui),
                        Tab::Contacts => self.contacts_tab(ui),
                        Tab::Message => self.message_tab(ui),
                    });
            });

        // Tant qu'une opération est en cours, ou que l'état peut changer hors
        // de l'application, on reste à l'écoute.
        if self.busy() {
            ctx.request_repaint_after(Duration::from_millis(80));
        } else {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }

    /// Quitter avec la protection active ET le proxy de l'application : le
    /// système resterait dirigé vers un proxy arrêté, donc plus de navigation.
    /// On coupe d'abord, on ferme ensuite.
    fn guard_close(&mut self, ctx: &egui::Context) {
        let wants_close = ctx.input(|i| i.viewport().close_requested());
        if !wants_close || self.quit_confirmed {
            return;
        }
        if self.snapshot.engaged && self.engine.is_some() {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if !self.busy() && !self.quit_blocked {
                self.deactivate(true);
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(egui::include_image!("../assets/mascotte.png"))
                    .fit_to_exact_size(vec2(52.0, 52.0))
                    .corner_radius(26),
            );
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Brocolis")
                        .family(bold())
                        .size(26.0)
                        .color(INK),
                );
                ui.label(RichText::new(self.t().tagline).size(12.5).color(MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                for lang in LANGS.iter().rev() {
                    let on = *lang == self.lang;
                    let b =
                        egui::Button::new(RichText::new(lang.label()).size(11.5).color(if on {
                            ACCENT_INK
                        } else {
                            INK
                        }))
                        .fill(if on { ACCENT } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(12))
                        .min_size(vec2(32.0, 24.0));
                    if ui.add(b).clicked() {
                        self.lang = *lang;
                    }
                }
            });
        });
    }

    fn tabs(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        Frame::new()
            .fill(SUNK)
            .corner_radius(CornerRadius::same(22))
            .inner_margin(Margin::same(4))
            .show(ui, |ui| {
                ui.columns(3, |cols| {
                    for (col, (tab, label)) in cols.iter_mut().zip([
                        (Tab::Protection, t.tab_protection),
                        (Tab::Contacts, t.tab_contacts),
                        (Tab::Message, t.tab_message),
                    ]) {
                        let on = self.tab == tab;
                        let b = egui::Button::new(
                            RichText::new(label)
                                .size(14.0)
                                .family(if on { bold() } else { FontFamily::Proportional })
                                .color(if on { ACCENT_INK } else { INK }),
                        )
                        .fill(if on { ACCENT } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(18))
                        .min_size(vec2(0.0, 34.0));
                        // Justifié : le bouton occupe toute sa colonne et son
                        // libellé y est centré, quelle que soit sa longueur.
                        let resp = col
                            .with_layout(Layout::top_down_justified(Align::Center), |ui| ui.add(b))
                            .inner;
                        resp.widget_info(|| {
                            WidgetInfo::selected(WidgetType::Button, true, on, label)
                        });
                        if resp.clicked() {
                            self.tab = tab;
                        }
                    }
                });
            });
    }

    fn protection(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        let engaged = self.snapshot.engaged;
        let waiting = matches!(
            self.job,
            Some(Job {
                kind: JobKind::Activate { .. } | JobKind::Deactivate { .. },
                ..
            })
        );

        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            let label = if engaged { t.deactivate } else { t.activate };
            if power_button(ui, engaged, waiting, label).clicked() && !waiting {
                if engaged {
                    self.deactivate(false);
                } else {
                    self.activate();
                }
            }
            ui.add_space(14.0);
            ui.label(
                RichText::new(if engaged { t.on_title } else { t.off_title })
                    .family(bold())
                    .size(24.0)
                    .color(if engaged { ACCENT } else { INK }),
            );
            ui.add_space(4.0);
            ui.label(RichText::new(if engaged { t.on_desc } else { t.off_desc }).color(MUTED));
            if waiting {
                let quitting = matches!(
                    self.job,
                    Some(Job {
                        kind: JobKind::Deactivate { then_quit: true },
                        ..
                    })
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 260.0).max(0.0) / 2.0);
                    ui.add(egui::Spinner::new().color(ACCENT));
                    let text = if quitting { t.quitting } else { t.waiting_auth };
                    ui.label(RichText::new(text).color(INK));
                });
            }
        });

        ui.add_space(18.0);
        card(ui, |ui| {
            let addr = self.listen.to_string();
            let (proxy_on, proxy_text) = if let Some(engine) = &self.engine {
                (
                    true,
                    t.proxy_app.replace("{addr}", &engine.listen.to_string()),
                )
            } else if self.snapshot.listening {
                (true, t.proxy_service.replace("{addr}", &addr))
            } else {
                (false, t.proxy_off.to_owned())
            };
            status_row(ui, t.row_proxy, &proxy_text, proxy_on);
            ui.add_space(6.0);
            status_row(
                ui,
                t.row_system,
                if engaged { t.system_on } else { t.system_off },
                engaged,
            );
        });
        // Pour qui le navigateur chiffre (`encrypt_to`). Sans objet quand un
        // service système tourne : il a sa propre configuration.
        if !self.service_running() && !self.book.contacts.is_empty() {
            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(RichText::new(t.browser_to).family(bold()).color(INK));
                ui.add_space(4.0);
                let mut targets = self.book.encrypt_to.clone();
                let mut changed = false;
                ui.horizontal_wrapped(|ui| {
                    if chip(ui, t.me, targets.is_empty(), true).clicked() && !targets.is_empty() {
                        targets.clear();
                        changed = true;
                    }
                    for p in &self.book.contacts {
                        let on = self.book.is_target(&p.name);
                        if chip(ui, &p.name, on, true).clicked() {
                            if on {
                                targets.retain(|n| !brocolis::config::same_name(n, &p.name));
                            } else {
                                targets.push(p.name.clone());
                            }
                            changed = true;
                        }
                    }
                });
                if targets.len() > 1 || (!changed && self.book.encrypt_to.len() > 1) {
                    ui.label(RichText::new(t.browser_to_note).size(12.0).color(MUTED));
                }
                if changed {
                    match keyring::set_encrypt_to(&self.opts.files.config, &targets) {
                        Ok(()) => self.keys_changed(),
                        Err(e) => self.notify(Kind::Err, format!("{}{e}", t.failed)),
                    }
                }
            });
        }
        ui.add_space(10.0);
        ui.label(RichText::new(t.auth_note).size(12.0).color(MUTED));
    }

    fn contacts_tab(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        title(ui, t.contacts_title);
        ui.label(RichText::new(t.contacts_intro).color(MUTED));
        if self.service_running() {
            ui.add_space(4.0);
            let note = t
                .service_note
                .replace("{path}", &self.opts.service_config.display().to_string());
            ui.label(RichText::new(note).size(12.0).color(MUTED));
        }
        ui.add_space(10.0);

        // Le carnet est copié : les cartes le lisent pendant que les champs de
        // saisie, eux, modifient l'application.
        let book = self.book.clone();
        let mut acts: Vec<Act> = Vec::new();

        if book.contacts.is_empty() {
            card(ui, |ui| {
                ui.label(RichText::new(t.contacts_empty).color(INK));
            });
            ui.add_space(8.0);
        }
        for person in &book.contacts {
            let name = &person.name;
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).family(bold()).size(17.0).color(INK));
                    if book.is_target(name) {
                        badge(ui, t.browser_badge);
                    }
                });
                ui.add_space(4.0);
                key_line(ui, &person.key, "");
                let label = t.protection(person.protection);
                let color = if person.protection.is_v3() {
                    ACCENT
                } else {
                    MUTED
                };
                ui.label(RichText::new(label).size(12.5).color(color));
                ui.add_space(8.0);
                let editing = |e: &Edit| match e {
                    Edit::Rekey(n) | Edit::Rename(n) => n == name,
                    Edit::Assign(_) => false,
                };
                match self.edit.clone().filter(editing) {
                    Some(Edit::Rekey(_)) => {
                        ui.label(RichText::new(t.rekey_note).size(12.0).color(DANGER));
                        edit_row(ui, &mut self.edit_input, t.rekey_hint, true, t, &mut acts);
                    }
                    Some(_) => edit_row(
                        ui,
                        &mut self.edit_input,
                        t.new_name_hint,
                        false,
                        t,
                        &mut acts,
                    ),
                    None if self.confirm_delete.as_ref() == Some(name) => {
                        ui.label(RichText::new(t.confirm_delete).color(DANGER));
                        ui.horizontal(|ui| {
                            if danger_button(ui, t.delete_contact).clicked() {
                                acts.push(Act::Delete(name.clone()));
                            }
                            if small_button(ui, t.cancel).clicked() {
                                acts.push(Act::CancelDelete);
                            }
                        });
                    }
                    None => {
                        ui.horizontal_wrapped(|ui| {
                            if small_button(ui, t.copy).clicked() {
                                acts.push(Act::Copy(person.key.b64.clone()));
                            }
                            if small_button(ui, t.change_key).clicked() {
                                acts.push(Act::Open(Edit::Rekey(name.clone())));
                            }
                            if small_button(ui, t.rename).clicked() {
                                acts.push(Act::Open(Edit::Rename(name.clone())));
                            }
                            if small_button(ui, t.delete_contact).clicked() {
                                acts.push(Act::AskDelete(name.clone()));
                            }
                            // Sans paquet échangé, le double cliquet part de
                            // la clé partagée : au choix de l'utilisateur.
                            if !person.paired && !person.group {
                                let (text, on) = if person.psk {
                                    (t.psk_off, false)
                                } else {
                                    (t.psk_on, true)
                                };
                                if small_button(ui, text).on_hover_text(t.psk_note).clicked() {
                                    acts.push(Act::Psk(name.clone(), on));
                                }
                            }
                        });
                    }
                }
            });
            ui.add_space(8.0);
        }

        // Nouveau contact : un nom, et sa clé (collée, ou créée ici).
        ui.add_space(10.0);
        subtitle(ui, t.new_contact_title);
        let name_edit = egui::TextEdit::singleline(&mut self.new_name)
            .hint_text(t.name_hint)
            .margin(Margin::symmetric(10, 8))
            .desired_width(f32::INFINITY);
        ui.add(name_edit)
            .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, t.name_hint));
        ui.add_space(4.0);
        let key_edit = egui::TextEdit::singleline(&mut self.new_key)
            .hint_text(t.new_contact_key_hint)
            .font(egui::TextStyle::Monospace)
            .margin(Margin::symmetric(10, 8))
            .desired_width(f32::INFINITY);
        ui.add(key_edit).widget_info(|| {
            WidgetInfo::labeled(WidgetType::TextEdit, true, t.new_contact_key_hint)
        });
        ui.add_space(6.0);
        let can_create = !self.new_name.trim().is_empty();
        if ui
            .add_enabled(can_create, accent(&format!("+  {}", t.create_contact)))
            .clicked()
        {
            self.create_contact(ui.ctx());
        }

        // Appairage : la clé obtenue va au contact nommé (ou reste sans nom).
        ui.add_space(18.0);
        subtitle(ui, t.pair_title);
        ui.label(RichText::new(t.pair_intro).size(12.5).color(MUTED));
        ui.add_space(4.0);
        let pairing = matches!(
            self.job,
            Some(Job {
                kind: JobKind::Pair,
                ..
            })
        );
        let pair_name = egui::TextEdit::singleline(&mut self.pair_name)
            .hint_text(t.pair_name_hint)
            .margin(Margin::symmetric(10, 8))
            .desired_width(f32::INFINITY);
        ui.add_enabled(!pairing, pair_name)
            .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, t.pair_name_hint));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut self.pair_code)
                .hint_text(t.pair_hint)
                .margin(Margin::symmetric(10, 8))
                .desired_width(ui.available_width() - 110.0);
            let resp = ui.add_enabled(!pairing, edit);
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, t.pair_hint));
            if ui
                .add_enabled(
                    !pairing
                        && !self.pair_code.trim().is_empty()
                        && !self.pair_name.trim().is_empty(),
                    small(t.pair_btn),
                )
                .clicked()
            {
                self.start_pairing();
            }
        });
        if pairing {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().color(ACCENT));
                ui.label(RichText::new(t.pair_wait).color(INK));
            });
        }
        if let Some(fp) = &self.pair_fingerprint {
            ui.add_space(6.0);
            card(ui, |ui| {
                ui.label(RichText::new(t.pair_done).color(INK));
                ui.label(RichText::new(fp).monospace().size(22.0).color(ACCENT));
            });
        }

        // Vos clés sans nom : la principale chiffre par défaut.
        ui.add_space(18.0);
        subtitle(ui, t.own_title);
        ui.label(RichText::new(t.own_intro).size(12.5).color(MUTED));
        ui.add_space(6.0);
        if book.own.is_empty() {
            card(ui, |ui| {
                ui.label(RichText::new(t.keys_empty).color(INK));
            });
            ui.add_space(8.0);
        }
        for key in &book.own {
            card(ui, |ui| {
                key_line(ui, key, t.primary);
                ui.horizontal_wrapped(|ui| {
                    if small_button(ui, t.copy).clicked() {
                        acts.push(Act::Copy(key.b64.clone()));
                    }
                    if !key.primary && small_button(ui, t.make_primary).clicked() {
                        acts.push(Act::OwnPrimary(key.b64.clone()));
                    }
                    if small_button(ui, t.assign).clicked() {
                        acts.push(Act::Open(Edit::Assign(key.b64.clone())));
                    }
                    if small_button(ui, t.remove).clicked() {
                        acts.push(Act::OwnRemove(key.b64.clone()));
                    }
                });
                if self.edit == Some(Edit::Assign(key.b64.clone())) {
                    ui.add_space(4.0);
                    edit_row(ui, &mut self.edit_input, t.assign_hint, false, t, &mut acts);
                }
            });
            ui.add_space(8.0);
        }
        if accent_button(ui, &format!("+  {}", t.new_key)).clicked() {
            match keyring::generate(&self.opts.files.config) {
                Ok(b64) => {
                    ui.ctx().copy_text(b64);
                    self.keys_changed();
                    self.notify(Kind::Ok, t.new_key_done);
                }
                Err(e) => self.notify(Kind::Err, format!("{}{e}", t.failed)),
            }
        }

        ui.add_space(18.0);
        subtitle(ui, t.add_title);
        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut self.add_input)
                .hint_text(t.add_hint)
                .font(egui::TextStyle::Monospace)
                .margin(Margin::symmetric(10, 8))
                .desired_width(ui.available_width() - 100.0);
            let resp = ui.add(edit);
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, t.add_hint));
            if small_button(ui, t.add_btn).clicked() {
                match keyring::add(&self.opts.files.config, &self.add_input) {
                    Ok(()) => {
                        self.add_input.clear();
                        self.keys_changed();
                        self.notify(Kind::Ok, t.add_done);
                    }
                    Err(e) => self.notify(Kind::Err, format!("{}{e}", t.invalid_key)),
                }
            }
        });

        for act in acts {
            self.apply(act, ui.ctx());
        }
    }

    fn create_contact(&mut self, ctx: &egui::Context) {
        let t = self.t();
        let name = self.new_name.trim().to_owned();
        let config = self.opts.files.config.clone();
        let result = if self.new_key.trim().is_empty() {
            keyring::contact_generate(&config, &name).map(|b64| {
                ctx.copy_text(b64);
                t.contact_created_key
            })
        } else {
            keyring::contact_add(&config, &name, &self.new_key)
                .map(|()| t.contact_created)
                .map_err(|e| anyhow::anyhow!("{}{e}", t.invalid_key.trim_end_matches(": ")))
        };
        match result {
            Ok(msg) => {
                self.new_name.clear();
                self.new_key.clear();
                self.keys_changed();
                self.notify(Kind::Ok, msg.replace("{name}", &name));
            }
            Err(e) => self.notify(Kind::Err, format!("{}{e}", t.failed)),
        }
    }

    /// Applique un clic de l'onglet Contacts.
    fn apply(&mut self, act: Act, ctx: &egui::Context) {
        let t = self.t();
        let config = self.opts.files.config.clone();
        let result: anyhow::Result<Option<String>> = match act {
            Act::Copy(b64) => {
                ctx.copy_text(b64);
                self.notify(Kind::Ok, t.copied);
                return;
            }
            Act::Open(edit) => {
                self.edit_input.clear();
                self.edit = Some(edit);
                self.confirm_delete = None;
                return;
            }
            Act::CancelEdit => {
                self.edit = None;
                return;
            }
            Act::AskDelete(name) => {
                self.edit = None;
                self.confirm_delete = Some(name);
                return;
            }
            Act::CancelDelete => {
                self.confirm_delete = None;
                return;
            }
            Act::Save => {
                let input = self.edit_input.trim().to_owned();
                let done = match self.edit.clone() {
                    // Champ vide : une clé neuve, copiée pour la lui transmettre.
                    Some(Edit::Rekey(name)) if input.is_empty() => {
                        keyring::contact_set_key(&config, &name, None).map(|b64| {
                            ctx.copy_text(b64);
                            Some(t.rekey_done.replace("{name}", &name))
                        })
                    }
                    Some(Edit::Rekey(name)) => {
                        keyring::contact_set_key(&config, &name, Some(&input))
                            .map(|_| Some(t.rekey_set.replace("{name}", &name)))
                    }
                    Some(Edit::Rename(old)) => {
                        keyring::contact_rename(&config, &old, &input).map(|()| None)
                    }
                    Some(Edit::Assign(b64)) => {
                        keyring::assign(&config, &b64, &input).map(|()| None)
                    }
                    None => Ok(None),
                };
                if done.is_ok() {
                    self.edit = None;
                    self.edit_input.clear();
                }
                done
            }
            Act::OwnPrimary(b64) => keyring::make_primary(&config, &b64).map(|()| None),
            Act::OwnRemove(b64) => keyring::remove(&config, &b64).map(|()| None),
            Act::Delete(name) => {
                self.confirm_delete = None;
                keyring::contact_delete(&config, &name).map(|()| None)
            }
            Act::Psk(name, on) => keyring::contact_set_psk(&config, &name, on).map(|()| None),
        };
        match result {
            Ok(note) => {
                self.keys_changed();
                if let Some(note) = note {
                    self.notify(Kind::Ok, note);
                }
            }
            Err(e) => self.notify(Kind::Err, format!("{}{e}", t.failed)),
        }
    }

    fn message_tab(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        title(ui, t.msg_title);
        ui.label(RichText::new(t.msg_intro).color(MUTED));
        ui.add_space(10.0);

        let input = egui::TextEdit::multiline(&mut self.message)
            .hint_text(t.msg_hint)
            .margin(Margin::same(10))
            .desired_rows(6)
            .desired_width(f32::INFINITY);
        let resp = ui.add(input);
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, t.msg_hint));
        ui.add_space(8.0);

        // Pour qui : vous, et/ou un ou plusieurs contacts.
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(t.to_label).family(bold()).color(INK));
            if self.book.primary().is_some() && chip(ui, t.me, self.send_me, true).clicked() {
                self.send_me = !self.send_me;
            }
            for p in &self.book.contacts {
                let on = self
                    .send_to
                    .iter()
                    .any(|n| brocolis::config::same_name(n, &p.name));
                if chip(ui, &p.name, on, true).clicked() {
                    if on {
                        self.send_to
                            .retain(|n| !brocolis::config::same_name(n, &p.name));
                    } else {
                        self.send_to.push(p.name.clone());
                    }
                }
            }
        });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if accent_button(ui, t.encrypt).clicked() {
                self.output_note = Some(match self.encrypt() {
                    Ok(note) => note,
                    Err(e) => format!("{}{e}", t.failed),
                });
            }
            if small_button(ui, t.decrypt).clicked() {
                self.output_note = Some(match self.decrypt() {
                    Ok(note) => note,
                    Err(e) => format!("{}{e}", t.failed),
                });
            }
        });

        if let Some(note) = &self.output_note {
            ui.add_space(4.0);
            ui.label(RichText::new(note).size(12.5).color(MUTED));
        }
        if !self.output.is_empty() {
            ui.add_space(8.0);
            subtitle(ui, t.result);
            let mut view = self.output.as_str();
            let resp = ui.add(
                egui::TextEdit::multiline(&mut view)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, t.result));
            if small_button(ui, t.copy).clicked() {
                ui.ctx().copy_text(self.output.clone());
                self.notify(Kind::Ok, t.copied);
            }
        }
    }

    /// Chiffre le message pour les destinataires choisis : les segments
    /// `((…))` s'il y en a, sinon le message entier. Renvoie la note à
    /// afficher.
    fn encrypt(&mut self) -> anyhow::Result<String> {
        use brocolis::crypto::Sealed;
        let t = self.t();
        let me = self.send_me && self.book.primary().is_some();
        let keys = keyring::recipients(&self.opts.files.config, me, &self.send_to)?;
        if keys.is_empty() {
            let empty = self.book.own.is_empty() && self.book.contacts.is_empty();
            return Ok(if empty { t.no_keys } else { t.no_recipient }.to_owned());
        }
        let (out, sealed, protection) =
            keyring::seal(&self.opts.files.config, &self.message, &keys)?;
        let who = me
            .then_some(t.me_short.to_owned())
            .into_iter()
            .chain(self.send_to.iter().cloned())
            .collect::<Vec<_>>()
            .join(", ");
        let done = match sealed {
            Sealed::Segments(n) => {
                self.output = out;
                t.encrypted_n
                    .replace("{n}", &n.to_string())
                    .replace("{who}", &who)
            }
            Sealed::Whole => {
                self.output = out;
                t.encrypted_whole.replace("{who}", &who)
            }
            Sealed::Nothing => return Ok(t.empty_message.to_owned()),
        };
        // Le double cliquet se signale ; l'ancien format est la règle d'avant.
        Ok(if protection.is_v3() {
            format!("{done} {}", t.protection(protection))
        } else {
            done
        })
    }

    /// Déchiffre avec toutes les clés et dit de qui vient le message.
    fn decrypt(&mut self) -> anyhow::Result<String> {
        let t = self.t();
        let Some(read) = keyring::read(&self.opts.files.config, &self.message)? else {
            return Ok(t.no_keys.to_owned());
        };
        self.output = read.text;
        let n = read.opened;
        let note = if n == 0 {
            t.nothing_readable.to_owned()
        } else if read.from.is_empty() {
            t.decrypted_n.replace("{n}", &n.to_string())
        } else {
            t.decrypted_from
                .replace("{n}", &n.to_string())
                .replace("{who}", &read.from.join(", "))
        };
        Ok(if read.replayed > 0 {
            let replayed = t.replayed.replace("{n}", &read.replayed.to_string());
            format!("{note} {replayed}")
        } else {
            note
        })
    }

    fn notice_bar(&mut self, ui: &mut egui::Ui) {
        let Some(n) = &self.notice else {
            return;
        };
        let color = match n.kind {
            Kind::Ok => ACCENT,
            Kind::Info => INK,
            Kind::Err => DANGER,
        };
        let mut close = false;
        Frame::new()
            .fill(SUNK)
            .stroke(Stroke::new(1.0, color.gamma_multiply(0.6)))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(RichText::new(&n.text).size(12.5).color(color)).wrap());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("×").size(18.0).color(MUTED))
                                    .frame(false),
                            )
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
            });
        ui.add_space(6.0);
        if close {
            self.notice = None;
        }
    }

    fn quit_dialog(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        Frame::new()
            .fill(SUNK)
            .stroke(Stroke::new(1.0, DANGER))
            .corner_radius(CornerRadius::same(16))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.label(RichText::new(t.quit_failed).color(DANGER));
                ui.horizontal(|ui| {
                    if small_button(ui, t.stay).clicked() {
                        self.quit_blocked = false;
                    }
                    if small_button(ui, t.quit_anyway).clicked() {
                        self.quit_confirmed = true;
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
        ui.add_space(10.0);
    }
}

// ----------------------------------------------------------------------
// Éléments graphiques
// ----------------------------------------------------------------------

pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

fn title(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).family(bold()).size(22.0).color(INK));
}

fn subtitle(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).family(bold()).size(16.0).color(INK));
}

fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui);
        });
}

fn badge(ui: &mut egui::Ui, text: &str) {
    Frame::new()
        .fill(ACCENT)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(ACCENT_INK));
        });
}

fn small(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).size(13.0).color(INK))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, INK.gamma_multiply(0.35)))
        .corner_radius(CornerRadius::same(16))
        .min_size(vec2(0.0, 30.0))
}

fn small_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(small(text))
}

fn accent(text: &str) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.to_owned())
            .family(bold())
            .size(14.0)
            .color(ACCENT_INK),
    )
    .fill(ACCENT)
    .stroke(Stroke::NONE)
    .corner_radius(CornerRadius::same(18))
    .min_size(vec2(0.0, 36.0))
}

fn accent_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(accent(text))
}

fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(ACCENT_INK))
            .fill(DANGER)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(16))
            .min_size(vec2(0.0, 30.0)),
    )
}

/// Une clé : son empreinte (à comparer de vive voix), un badge si c'est elle
/// qui chiffre, et la clé masquée.
fn key_line(ui: &mut egui::Ui, key: &Entry, badge_text: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&key.fingerprint)
                .monospace()
                .size(15.0)
                .color(INK),
        );
        if key.primary {
            badge(ui, badge_text);
        }
    });
    ui.label(
        RichText::new(mask(&key.b64))
            .monospace()
            .size(11.5)
            .color(MUTED),
    );
}

/// Un champ ouvert dans une carte, avec Valider (ou Entrée) et Annuler.
fn edit_row(
    ui: &mut egui::Ui,
    input: &mut String,
    hint: &str,
    monospace: bool,
    t: &Strings,
    acts: &mut Vec<Act>,
) {
    let mut edit = egui::TextEdit::singleline(input)
        .hint_text(hint)
        .margin(Margin::symmetric(10, 8))
        .desired_width(f32::INFINITY);
    if monospace {
        edit = edit.font(egui::TextStyle::Monospace);
    }
    let resp = ui.add(edit);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, hint));
    let entered = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    ui.horizontal(|ui| {
        if small_button(ui, t.save).clicked() || entered {
            acts.push(Act::Save);
        }
        if small_button(ui, t.cancel).clicked() {
            acts.push(Act::CancelEdit);
        }
    });
}

/// Une pastille à cocher (destinataire choisi ou non).
fn chip(ui: &mut egui::Ui, text: &str, on: bool, enabled: bool) -> egui::Response {
    let b = egui::Button::new(
        RichText::new(if on {
            format!("✔ {text}")
        } else {
            text.to_owned()
        })
        .size(13.0)
        .color(if on { ACCENT_INK } else { INK }),
    )
    .fill(if on { ACCENT } else { Color32::TRANSPARENT })
    .stroke(if on {
        Stroke::NONE
    } else {
        Stroke::new(1.0, INK.gamma_multiply(0.35))
    })
    .corner_radius(CornerRadius::same(14))
    .min_size(vec2(0.0, 28.0));
    let resp = ui.add_enabled(enabled, b);
    // Pour les lecteurs d'écran : le nom seul, et l'état « coché ».
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, on, text));
    resp
}

fn status_row(ui: &mut egui::Ui, name: &str, value: &str, on: bool) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
        ui.painter().circle_filled(
            rect.center(),
            4.5,
            if on {
                ACCENT
            } else {
                MUTED.gamma_multiply(0.5)
            },
        );
        ui.label(RichText::new(name).family(bold()).color(INK));
        ui.label(RichText::new(value).color(MUTED));
    });
}

/// Une clé n'est jamais affichée en entier à l'écran : début et fin suffisent
/// à la reconnaître, et « Copier » la donne entière.
fn mask(b64: &str) -> String {
    let n = b64.chars().count();
    if n <= 12 {
        return "•".repeat(n);
    }
    let head: String = b64.chars().take(6).collect();
    let tail: String = b64.chars().skip(n - 4).collect();
    format!("{head}••••••••••••{tail}")
}

/// Le grand interrupteur : un disque qui passe du vert sombre au citron vert,
/// avec un halo qui respire tant que la protection est active, et le libellé
/// de l'action dessiné dessous (un seul élément pour les lecteurs d'écran).
fn power_button(ui: &mut egui::Ui, on: bool, waiting: bool, label: &str) -> egui::Response {
    let size = Vec2::new(ui.available_width().min(320.0), 196.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, !waiting, label));
    let ctx = ui.ctx();
    let k = ctx.animate_bool_with_time(resp.id, on, 0.45);
    let hover = ctx.animate_bool_with_time(resp.id.with("survol"), resp.hovered(), 0.2);
    let time = ctx.input(|i| i.time) as f32;
    let center = pos2(rect.center().x, rect.top() + 78.0);
    let r = 64.0 + hover * 3.0;
    let painter = ui.painter();

    // Halo : deux ondes qui s'éloignent du disque, seulement quand c'est actif.
    if k > 0.01 {
        for phase in [0.0f32, 0.5] {
            let p = (time * 0.55 + phase).fract();
            painter.circle_stroke(
                center,
                r + 4.0 + p * 26.0,
                Stroke::new(2.0, ACCENT.gamma_multiply((1.0 - p) * 0.55 * k)),
            );
        }
        ctx.request_repaint_after(Duration::from_millis(30));
    }
    // Pendant l'attente : un arc qui tourne autour du disque.
    if waiting {
        let start = time * 4.0;
        let points: Vec<_> = (0..=24)
            .map(|i| {
                let a = start + i as f32 / 24.0 * 1.6;
                center + vec2(a.cos(), a.sin()) * (r + 9.0)
            })
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(3.0, ACCENT)));
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    let fill = lerp(CARD, ACCENT, k);
    painter.circle_filled(center, r, fill);
    painter.circle_stroke(
        center,
        r,
        Stroke::new(1.5, lerp(INK.gamma_multiply(0.25), ACCENT, k)),
    );

    // Pictogramme « marche/arrêt » : un arc ouvert en haut et un trait.
    let glyph = lerp(INK, ACCENT_INK, k);
    let arc: Vec<_> = (0..=40)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2
                + 0.75
                + i as f32 / 40.0 * (std::f32::consts::TAU - 1.5);
            center + vec2(a.cos(), a.sin()) * 24.0
        })
        .collect();
    painter.add(egui::Shape::line(arc, Stroke::new(4.5, glyph)));
    painter.line_segment(
        [center + vec2(0.0, -32.0), center + vec2(0.0, -8.0)],
        Stroke::new(4.5, glyph),
    );

    painter.text(
        pos2(rect.center().x, rect.bottom() - 18.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::new(15.0, bold()),
        if waiting { MUTED } else { INK },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}
