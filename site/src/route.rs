//! Routage par fragment d'URL (`#/...`).
//!
//! Le fragment est choisi plutôt que l'historique `pushState` pour la même
//! raison qu'avant la réécriture : le site doit fonctionner à l'identique à la
//! racine d'un domaine, sous un sous-chemin, ou ouvert depuis un fichier local,
//! sans réécriture d'URL côté serveur ni page 404 de repli.
//!
//! Les liens restent donc de vraies ancres `<a href="#/...">` : le navigateur
//! met le fragment à jour lui-même, l'historique et le clic-milieu
//! fonctionnent, et on se contente d'écouter `hashchange`.

use yew::prelude::*;

use crate::dom::{Listener, window};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Route {
    #[default]
    Home,
    Download,
}

impl Route {
    /// Valeur à mettre dans un `href`.
    pub const fn href(self) -> &'static str {
        match self {
            Route::Home => "#/",
            Route::Download => "#/telecharger",
        }
    }

    /// Tout fragment inconnu retombe sur l'accueil — y compris les ancres de
    /// section (`#platforms`), que la barre de navigation intercepte de toute
    /// façon avant qu'elles n'atteignent l'URL.
    fn parse(hash: &str) -> Self {
        let path = hash.strip_prefix('#').unwrap_or(hash);
        let path = path.split(['?', '&']).next().unwrap_or(path);
        match path.trim_end_matches('/') {
            "/telecharger" => Route::Download,
            _ => Route::Home,
        }
    }

    pub fn current() -> Self {
        Self::parse(&window().location().hash().unwrap_or_default())
    }
}

/// Route courante, réévaluée à chaque `hashchange`.
#[hook]
pub fn use_route() -> Route {
    let route = use_state(Route::current);

    {
        let route = route.clone();
        use_effect_with((), move |()| {
            let listener = Listener::on_window("hashchange", move |_| {
                route.set(Route::current());
            });
            move || drop(listener)
        });
    }

    *route
}

#[cfg(test)]
mod tests {
    use super::Route;

    #[test]
    fn les_fragments_connus_sont_reconnus() {
        assert_eq!(Route::parse("#/telecharger"), Route::Download);
        assert_eq!(Route::parse("#/telecharger/"), Route::Download);
        assert_eq!(Route::parse("#/telecharger?src=mail"), Route::Download);
        assert_eq!(Route::parse("/telecharger"), Route::Download);
    }

    #[test]
    fn tout_le_reste_est_laccueil() {
        for hash in ["", "#", "#/", "#/inconnu", "#platforms", "#/telecharger-x"] {
            assert_eq!(Route::parse(hash), Route::Home, "fragment : {hash:?}");
        }
    }
}
