//! Préférences utilisateur (langue, thème) partagées par tout l'arbre.
//!
//! Un seul contexte porte les deux valeurs et leurs modificateurs : tout
//! composant qui affiche du texte a besoin de la langue, et les deux
//! sélecteurs vivent côte à côte dans la même barre.

use yew::prelude::*;

use crate::dom::{navigator_language, storage_get, storage_set};
use crate::i18n::{Lang, Strings};
use crate::theme::Theme;

const LANG_KEY: &str = "brocolis.lang";
const THEME_KEY: &str = "brocolis.theme";

#[derive(Clone, PartialEq)]
pub struct Settings {
    pub lang: Lang,
    pub theme: Theme,
    pub set_lang: Callback<Lang>,
    pub set_theme: Callback<Theme>,
}

/// Langue choisie précédemment, sinon celle du navigateur, sinon le français.
pub fn detect_lang() -> Lang {
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
pub fn detect_theme() -> Theme {
    storage_get(THEME_KEY)
        .as_deref()
        .and_then(Theme::from_code)
        .unwrap_or_default()
}

pub fn remember_lang(lang: Lang) {
    storage_set(LANG_KEY, lang.code());
}

pub fn remember_theme(theme: Theme) {
    storage_set(THEME_KEY, theme.code());
}

#[hook]
pub fn use_settings() -> Settings {
    use_context::<Settings>().expect("le contexte `Settings` est fourni par <App>")
}

/// Raccourci pour le cas courant : seules les chaînes traduites comptent.
#[hook]
pub fn use_t() -> &'static Strings {
    use_settings().lang.strings()
}
