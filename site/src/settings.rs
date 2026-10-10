//! Préférences utilisateur (langue, thème) partagées par tout le site.
//!
//! Deux `State` d'`active`, créés une fois au démarrage et vivant aussi
//! longtemps que la page. Tout composant qui affiche du texte lit la langue
//! — mais seulement dans les fermetures de liaison (`text_dyn`, `html_dyn`,
//! `attr_dyn`…) : un changement de langue réécrit alors les textes en place,
//! sans reconstruire le DOM qui les porte. Lire la langue en construisant une
//! vue abonnerait la région dynamique englobante, qui serait entièrement
//! reconstruite à chaque changement.

use std::cell::Cell;

use active::{State, use_state};

use crate::dom::{navigator_language, storage_get, storage_set};
use crate::i18n::{Lang, Strings};
use crate::theme::Theme;

const LANG_KEY: &str = "brocolis.lang";
const THEME_KEY: &str = "brocolis.theme";

thread_local! {
    static SETTINGS: Cell<Option<(State<Lang>, State<Theme>)>> = const { Cell::new(None) };
}

/// Crée les deux préférences. À appeler une fois, avant tout rendu et hors de
/// toute région dynamique : sans propriétaire, les états vivent aussi
/// longtemps que la page.
pub fn init() {
    let states = (use_state(detect_lang()), use_state(detect_theme()));
    SETTINGS.with(|s| s.set(Some(states)));
}

fn settings() -> (State<Lang>, State<Theme>) {
    SETTINGS
        .with(Cell::get)
        .expect("`settings::init` est appelé au démarrage")
}

/// La langue courante, en tant qu'état réactif.
pub fn lang() -> State<Lang> {
    settings().0
}

/// Le thème courant, en tant qu'état réactif.
pub fn theme() -> State<Theme> {
    settings().1
}

/// Les chaînes de la langue courante. Lu dans une fermeture de liaison, abonne
/// celle-ci à la langue.
pub fn t() -> &'static Strings {
    lang().get().strings()
}

/// Change de langue et mémorise le choix.
pub fn set_lang(choice: Lang) {
    storage_set(LANG_KEY, choice.code());
    lang().set(choice);
}

/// Change de thème et mémorise le choix.
pub fn set_theme(choice: Theme) {
    storage_set(THEME_KEY, choice.code());
    theme().set(choice);
}

/// Langue choisie précédemment, sinon celle du navigateur, sinon le français.
fn detect_lang() -> Lang {
    storage_get(LANG_KEY)
        .as_deref()
        .and_then(Lang::from_code)
        .or_else(|| navigator_language().as_deref().and_then(Lang::from_code))
        .unwrap_or_default()
}

/// Thème choisi précédemment, sinon le thème vert.
///
/// Pas de repli sur `prefers-color-scheme` : « vert » est l'identité du site,
/// pas un équivalent de « clair ». Qui préfère suivre son système choisit
/// explicitement clair ou sombre, et ce choix est mémorisé.
fn detect_theme() -> Theme {
    storage_get(THEME_KEY)
        .as_deref()
        .and_then(Theme::from_code)
        .unwrap_or_default()
}
