//! Thèmes. L'attribut `data-theme` de `<html>` est la seule source de vérité
//! côté CSS ; ce module le tient à jour et le persiste.

/// Les thèmes proposés, dans l'ordre d'affichage du sélecteur.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Theme {
    #[default]
    Green,
    Light,
    Dark,
}

pub const THEMES: [Theme; 3] = [Theme::Green, Theme::Light, Theme::Dark];

impl Theme {
    /// Valeur écrite dans `data-theme` et dans le stockage local.
    pub fn code(self) -> &'static str {
        match self {
            Theme::Green => "green",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// Pictogramme du sélecteur.
    pub fn icon(self) -> &'static str {
        match self {
            Theme::Green => "🥦",
            Theme::Light => "☀",
            Theme::Dark => "🌙",
        }
    }

    pub fn from_code(raw: &str) -> Option<Self> {
        match raw {
            "green" => Some(Theme::Green),
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
}
