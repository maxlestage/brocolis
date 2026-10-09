//! L'application pilotée comme par un utilisateur : on clique sur des
//! libellés et on lit ce qui s'affiche (via l'arbre d'accessibilité, comme un
//! lecteur d'écran). La demande d'autorisation administrateur est remplacée
//! par une fonction qui imite `brocolis on`/`off` : elle crée ou retire le
//! fichier d'état, et note les arguments reçus.

use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use crate::app::{BrocolisApp, Elevator, Options, Tab};
use crate::files::Files;
use crate::i18n::{FR, Lang};

struct Bench {
    dir: PathBuf,
    files: Files,
    state: PathBuf,
    calls: Arc<Mutex<Vec<Vec<String>>>>,
}

impl Bench {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("brocolis-app-ui-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let files = Files::at(dir.join("donnees"));
        files.ensure().unwrap();
        // Un port libre : le test ne doit pas dépendre de ce qui tourne sur la
        // machine (un vrai service Brocolis écoute peut-être sur 8080).
        let port = free_port();
        let toml = std::fs::read_to_string(&files.config)
            .unwrap()
            .replace("127.0.0.1:8080", &format!("127.0.0.1:{port}"));
        std::fs::write(&files.config, toml).unwrap();
        Self {
            state: dir.join("etat-precedent.json"),
            dir,
            files,
            calls: Arc::default(),
        }
    }

    /// Élévation simulée : `on` crée l'état, `off` le retire. `refuse`
    /// imite l'utilisateur qui ferme la boîte d'autorisation.
    fn elevator(&self, refuse: bool) -> Elevator {
        let (calls, state) = (self.calls.clone(), self.state.clone());
        Arc::new(move |args: &[String]| {
            calls.lock().unwrap().push(args.to_vec());
            if refuse {
                return Err("execution error: User canceled. (-128)".into());
            }
            match args[0].as_str() {
                "on" => std::fs::write(&state, "{}").map_err(|e| e.to_string()),
                "off" => std::fs::remove_file(&state).map_err(|e| e.to_string()),
                _ => Err("commande inconnue".into()),
            }
        })
    }

    fn app(&self, refuse: bool) -> BrocolisApp {
        BrocolisApp::new(Options {
            files: self.files.clone(),
            state_path: self.state.clone(),
            service_config: self.dir.join("service.toml"),
            elevator: self.elevator(refuse),
            lang: Lang::Fr,
        })
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn harness(app: BrocolisApp) -> Harness<'static, BrocolisApp> {
    // Les polices déclarées par `style::apply` ne servent qu'à l'image
    // suivante (comme dans eframe, où elles sont posées avant la première) :
    // la première image du harnais ne fait donc que les installer.
    let mut ready = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(460.0, 900.0))
        .build_ui_state(
            move |ui, app: &mut BrocolisApp| {
                if !ready {
                    crate::style::apply(ui.ctx());
                    egui_extras::install_image_loaders(ui.ctx());
                    ready = true;
                    ui.ctx().request_repaint();
                    return;
                }
                app.show(ui);
            },
            app,
        );
    h.run_steps(2);
    h
}

/// Fait tourner l'interface jusqu'à ce que `done` soit vrai : les opérations
/// longues tournent sur un autre fil, il faut leur laisser le temps.
fn until(h: &mut Harness<'static, BrocolisApp>, done: impl Fn(&BrocolisApp) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(h.state()) {
        assert!(Instant::now() < deadline, "condition jamais atteinte");
        h.step();
        std::thread::sleep(Duration::from_millis(20));
    }
    h.run_steps(2);
}

#[test]
fn activer_puis_couper_la_protection() {
    let bench = Bench::new("interrupteur");
    let mut h = harness(bench.app(false));
    h.run_steps(2);
    h.get_by_label(FR.off_title);

    h.get_by_label(FR.activate).click();
    until(&mut h, |a| a.snapshot.engaged && a.snapshot.listening);
    h.get_by_label(FR.on_title);
    h.get_by_label(FR.deactivate);

    let on = bench.calls.lock().unwrap()[0].clone();
    assert_eq!(on[0], "on");
    assert!(
        on.windows(2)
            .any(|w| w[0] == "--config" && w[1] == bench.files.config.display().to_string()),
        "le proxy de l'application est engagé avec SA configuration : {on:?}"
    );
    assert!(
        on.windows(2)
            .any(|w| w[0] == "--state" && w[1] == bench.state.display().to_string())
    );

    h.get_by_label(FR.deactivate).click();
    until(&mut h, |a| !a.snapshot.engaged && !a.snapshot.listening);
    h.get_by_label(FR.off_title);
    assert_eq!(bench.calls.lock().unwrap()[1][0], "off");
}

#[test]
fn autorisation_refusee_rien_ne_change() {
    let bench = Bench::new("refus");
    let mut h = harness(bench.app(true));
    h.run_steps(2);
    h.get_by_label(FR.activate).click();
    until(&mut h, |a| {
        a.snapshot == Default::default() && bench.calls.lock().unwrap().len() == 1
    });
    // Le proxy démarré pour l'occasion est arrêté, le message est clair.
    until(&mut h, |a| !a.snapshot.listening);
    h.get_by_label(FR.canceled);
    h.get_by_label(FR.off_title);
}

#[test]
fn un_service_systeme_est_engage_avec_sa_propre_configuration() {
    let bench = Bench::new("service");
    // Un « service » écoute déjà sur le port configuré.
    let cfg = brocolis::Config::load(&bench.files.config).unwrap();
    let _service = TcpListener::bind(cfg.listen).unwrap();
    let mut h = harness(bench.app(false));
    h.run_steps(2);
    h.get_by_label(FR.activate).click();
    until(&mut h, |a| a.snapshot.engaged);
    let on = bench.calls.lock().unwrap()[0].clone();
    let service = bench.dir.join("service.toml").display().to_string();
    assert!(
        on.windows(2).any(|w| w[0] == "--config" && w[1] == service),
        "c'est la configuration du service (et donc son CA) qui est engagée : {on:?}"
    );
}

/// Le champ « Résultat » : le sous-titre porte le même libellé (il le
/// désigne), on cible donc le champ par son rôle.
fn resultat(h: &Harness<'static, BrocolisApp>) -> String {
    h.get_by_role_and_label(Role::TextInput, FR.result)
        .value()
        .expect("champ Résultat présent")
}

#[test]
fn chiffrer_puis_dechiffrer_un_message() {
    let bench = Bench::new("message");
    let mut h = harness(bench.app(false));
    h.state_mut().tab = Tab::Message;
    h.run_steps(2);

    h.get_by_label(FR.msg_hint).focus();
    h.get_by_label(FR.msg_hint)
        .type_text("Rendez-vous ((demain 18h)) au café");
    h.run_steps(2);
    h.get_by_label(FR.encrypt).click();
    h.run_steps(2);
    let chiffre = resultat(&h);
    assert!(
        chiffre.starts_with("Rendez-vous [[BROCOLIS:v1:") && chiffre.ends_with("]] au café"),
        "{chiffre}"
    );
    assert!(
        !chiffre.contains("demain"),
        "le secret ne doit plus apparaître"
    );
    h.get_by_label("1 segment(s) chiffré(s).");

    // Le message chiffré, recollé, se déchiffre.
    h.get_by_label(FR.msg_hint).focus();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_by_label(FR.msg_hint).type_text(&chiffre);
    h.run_steps(2);
    h.get_by_label(FR.decrypt).click();
    h.run_steps(2);
    assert_eq!(resultat(&h), "Rendez-vous demain 18h au café");
}

#[test]
fn gerer_le_trousseau() {
    let bench = Bench::new("cles");
    let mut h = harness(bench.app(false));
    h.state_mut().tab = Tab::Keys;
    h.run_steps(2);
    assert_eq!(
        h.query_all_by_label(FR.remove).count(),
        1,
        "une clé au premier lancement"
    );

    h.get_by_label(&format!("+  {}", FR.new_key)).click();
    h.run_steps(2);
    assert_eq!(h.query_all_by_label(FR.remove).count(), 2);
    h.get_by_label(FR.new_key_done);

    // Une clé reçue, collée.
    let recue = brocolis::SymKey::generate().to_base64();
    h.get_by_label(FR.add_hint).focus();
    h.get_by_label(FR.add_hint).type_text(&recue);
    h.run_steps(2);
    h.get_by_label(FR.add_btn).click();
    h.run_steps(2);
    assert_eq!(h.query_all_by_label(FR.remove).count(), 3);
    assert!(
        std::fs::read_to_string(&bench.files.config)
            .unwrap()
            .contains(&recue)
    );

    // Une clé invalide est refusée, le trousseau reste intact.
    h.get_by_label(FR.add_hint).focus();
    h.get_by_label(FR.add_hint).type_text("n'importe quoi");
    h.run_steps(2);
    h.get_by_label(FR.add_btn).click();
    h.run_steps(2);
    assert_eq!(h.query_all_by_label(FR.remove).count(), 3);
    h.get_by_label_contains(FR.invalid_key);

    // Retirer la dernière.
    h.get_all_by_label(FR.remove).last().unwrap().click();
    h.run_steps(2);
    assert_eq!(h.query_all_by_label(FR.remove).count(), 2);
}

#[test]
fn la_protection_restee_active_relance_le_proxy() {
    // Une session précédente s'est interrompue protection active : au
    // lancement, sans proxy, la navigation serait coupée. L'application
    // relance aussitôt le sien.
    let bench = Bench::new("reprise");
    std::fs::write(&bench.state, "{}").unwrap();
    let app = bench.app(false);
    assert!(app.snapshot.engaged && app.snapshot.listening);
}
