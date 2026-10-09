//! Brocolis 🥦 — application de bureau (macOS, Linux, Windows).
//!
//! Une petite fenêtre pour ce que la ligne de commande fait en plusieurs
//! commandes : activer ou couper la protection d'un clic, gérer ses clés
//! (créer, ajouter, appairer par un code), chiffrer ou déchiffrer un message.
//!
//! Elle s'appuie sur la bibliothèque `brocolis` (proxy, chiffrement,
//! appairage) et, pour ce qui exige les droits administrateur, sur la ligne de
//! commande `brocolis on` / `off`, livrée avec elle (voir `elevate.rs`).

// Pas de console noire derrière la fenêtre sous Windows (en version publiée ;
// la console reste en développement, pour voir les journaux).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod elevate;
mod engine;
mod files;
mod i18n;
mod keyring;
mod style;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use eframe::egui;

use app::{BrocolisApp, Options};
use files::Files;
use i18n::Lang;

fn main() -> eframe::Result {
    tracing_subscriber_init();

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/brocolis.png"))
        .expect("icône PNG valide");
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Brocolis")
            .with_app_id("brocolis")
            .with_inner_size([460.0, 720.0])
            .with_min_inner_size([400.0, 560.0])
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "Brocolis",
        native,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            style::apply(&cc.egui_ctx);
            Ok(Box::new(Window(BrocolisApp::new(options()))))
        }),
    )
}

/// Les réglages réels : dossier de l'utilisateur, état et configuration du
/// service aux emplacements système, et vraie demande d'autorisation.
fn options() -> Options {
    Options {
        files: Files::for_user(),
        state_path: brocolis::system::default_state_path(),
        service_config: brocolis::system::default_config_path(),
        elevator: Arc::new(|args: &[String]| {
            let cli = elevate::find_cli().ok_or_else(|| Lang::detect().t().no_cli.to_owned())?;
            elevate::run(&elevate::plan(elevate::Os::current(), &cli, args))
        }),
        lang: Lang::detect(),
    }
}

struct Window(BrocolisApp);

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.show(ui);
    }
}

/// Journaux de la bibliothèque (proxy, appairage) sur la sortie d'erreur,
/// visibles en lançant l'application depuis un terminal. `RUST_LOG` les règle.
fn tracing_subscriber_init() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,brocolis=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
