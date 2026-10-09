//! Site vitrine Brocolis — application WebAssembly.
//!
//! Rendu entièrement côté client avec Yew : un seul `index.html`, aucune route
//! serveur à configurer. Le serveur statique (`server.js`, à la racine du
//! dépôt) ne fait que servir les fichiers et relayer les téléchargements.
//!
//! Organisation :
//!
//! - `i18n` : les trois langues, une `struct` par langue — l'exhaustivité des
//!   traductions est donc vérifiée à la compilation
//! - `theme` : les trois thèmes, pilotés par `<html data-theme>`
//! - `settings` : préférences partagées par contexte, et leur persistance
//! - `route` : routage par fragment d'URL
//! - `dom` : les rares accès au DOM hors rendu (titre, `<meta>`, focus)
//! - `markup` : insertion du balisage contenu dans les traductions
//! - `motion` : tout le mouvement (défilement, curseur, poussière du héros),
//!   piloté hors de Yew par des attributs et des variables CSS

mod components;
mod dom;
mod i18n;
mod markup;
mod motion;
mod route;
mod settings;
mod theme;

use yew::prelude::*;

use components::{Ambient, Curtain, DownloadPage, Landing};
use dom::{scroll_to_top, set_meta, set_root_attribute, set_title};
use i18n::Lang;
use route::{Route, use_route};
use settings::{Settings, detect_lang, detect_theme, remember_lang, remember_theme};
use theme::Theme;

fn main() {
    // Avant le premier rendu : la feuille de style en dépend pour savoir si
    // elle doit cacher ce qui apparaîtra ensuite.
    motion::flag_document();
    yew::Renderer::<App>::new().render();
}

#[function_component]
fn App() -> Html {
    let lang = use_state(detect_lang);
    let theme = use_state(detect_theme);
    let route = use_route();
    // Nombre de changements de page : sert de clé au volet de transition,
    // qui rejoue ainsi son animation à chaque navigation (et pas au
    // chargement, que le rideau d'ouverture couvre déjà).
    let navigations = use_mut_ref(|| (route, 0u32));
    {
        let mut nav = navigations.borrow_mut();
        if nav.0 != route {
            *nav = (route, nav.1 + 1);
        }
    }
    let wipe = navigations.borrow().1;

    // Le contrôleur de mouvement vit aussi longtemps que l'application.
    use_effect_with((), |()| {
        let motion = motion::Motion::start();
        move || drop(motion)
    });

    let set_lang = {
        let lang = lang.clone();
        Callback::from(move |choice: Lang| {
            remember_lang(choice);
            lang.set(choice);
        })
    };

    let set_theme = {
        let theme = theme.clone();
        Callback::from(move |choice: Theme| {
            remember_theme(choice);
            theme.set(choice);
        })
    };

    // `data-theme` sur `<html>` : toute la palette en découle, y compris le
    // `color-scheme` que le navigateur utilise pour ses propres éléments.
    use_effect_with(*theme, |theme| {
        set_root_attribute("data-theme", theme.code());
    });

    // Métadonnées du document. Elles dépendent de la langue ET de la route :
    // en application monopage, rien de tout ça ne change tout seul.
    use_effect_with((*lang, route), |(lang, route)| {
        let t = lang.strings();
        set_root_attribute("lang", lang.code());
        set_title(&match route {
            Route::Home => t.meta_title.to_owned(),
            Route::Download => format!("{} — Brocolis 🥦", t.dl_h),
        });
        set_meta("meta[name=\"description\"]", t.meta_desc);
        set_meta("meta[property=\"og:title\"]", t.meta_og_title);
        set_meta("meta[property=\"og:description\"]", t.meta_og_desc);
    });

    // Un lien vers `#/telecharger` ne désigne aucune ancre : le navigateur
    // laisse donc la page où elle était, parfois au milieu de l'accueil.
    use_effect_with(route, |route| {
        if matches!(route, Route::Download) {
            scroll_to_top();
        }
    });

    let settings = Settings {
        lang: *lang,
        theme: *theme,
        set_lang,
        set_theme,
    };

    html! {
        <ContextProvider<Settings> context={settings}>
            <Curtain />
            { match route {
                Route::Home => html! { <Landing /> },
                Route::Download => html! { <DownloadPage /> },
            } }
            if wipe > 0 {
                <div class="wipe" key={wipe.to_string()} aria-hidden="true" />
            }
            <Ambient />
        </ContextProvider<Settings>>
    }
}
