//! Site vitrine Brocolis — application WebAssembly.
//!
//! Écrit avec [`active`](https://github.com/maxlestage/Active) et rendu
//! entièrement côté client : un seul `index.html`, aucune route serveur à
//! configurer. Le serveur statique (`server.js`, à la racine du dépôt) ne fait
//! que servir les fichiers et relayer les téléchargements.
//!
//! Les composants sont de simples fonctions qui renvoient une vue. Il n'y a ni
//! DOM virtuel ni second rendu : chaque texte, attribut ou classe qui dépend
//! d'un état (`active::State`) y est lié directement, et seul ce nœud change
//! quand l'état change.
//!
//! Organisation :
//!
//! - `i18n` : les trois langues, une `struct` par langue — l'exhaustivité des
//!   traductions est donc vérifiée à la compilation
//! - `theme` : les trois thèmes, pilotés par `<html data-theme>`
//! - `settings` : préférences (langue, thème) et leur persistance
//! - `route` : routage par fragment d'URL
//! - `dom` : les rares accès au DOM hors rendu (titre, `<meta>`, focus)
//! - `markup` : textes traduits, balisage contenu dans les traductions
//! - `motion` : tout le mouvement (défilement, curseur, poussière du héros),
//!   piloté hors des vues par des attributs et des variables CSS

mod components;
mod dom;
mod i18n;
mod markup;
mod motion;
mod proto_text;
mod route;
mod settings;
mod theme;

use active::html::{div, span};
use active::{Node, effect, fragment, fragment_dyn};

use components::{ambient, curtain, download_page, landing, protocol_page};
use dom::{set_meta, set_root_attribute, set_title};
use markup::keep;
use route::{Route, router};

fn main() {
    // Avant le premier rendu : la feuille de style en dépend pour savoir si
    // elle doit cacher ce qui apparaîtra ensuite.
    motion::flag_document();
    settings::init();
    route::init();
    document_effects();
    active::mount_to("body", app());
    // Le contrôleur de mouvement vit aussi longtemps que la page. Démarré
    // après le montage : son premier inventaire trouve tout le DOM en place.
    keep(motion::Motion::start());
}

/// Ce qui vit hors de `<body>` : thème, langue, titre de l'onglet, `<meta>`.
fn document_effects() {
    let (lang, theme) = (settings::lang(), settings::theme());
    let route = router().route;

    // `data-theme` sur `<html>` : toute la palette en découle, y compris le
    // `color-scheme` que le navigateur utilise pour ses propres éléments.
    effect(move || set_root_attribute("data-theme", theme.get().code()));

    // Métadonnées du document. Elles dépendent de la langue ET de la route :
    // en application monopage, rien de tout ça ne change tout seul.
    effect(move || {
        let lang = lang.get();
        let t = lang.strings();
        set_root_attribute("lang", lang.code());
        set_title(&match route.get() {
            Route::Home => t.meta_title.to_owned(),
            Route::Download => format!("{} — Brocolis 🥦", t.dl_h),
            Route::Protocol => t.proto.meta_title.to_owned(),
        });
        set_meta("meta[name=\"description\"]", t.meta_desc);
        set_meta("meta[property=\"og:title\"]", t.meta_og_title);
        set_meta("meta[property=\"og:description\"]", t.meta_og_desc);
    });
}

fn app() -> Node {
    let router = router();
    fragment([
        curtain(),
        // La page courante. Seule cette région lit la route : changer de page
        // la reconstruit, et libère tout ce que l'ancienne avait créé.
        fragment_dyn(move || {
            vec![match router.route.get() {
                Route::Home => landing(),
                Route::Download => download_page(),
                Route::Protocol => protocol_page(),
            }]
        }),
        // Un nouveau volet à chaque navigation : son animation rejoue.
        fragment_dyn(move || {
            if router.navigations.get() == 0 {
                return Vec::new();
            }
            vec![
                div()
                    .class("wipe")
                    .attr("aria-hidden", "true")
                    .child(span().class("wipe-brand").text("🥦 Brocolis"))
                    .into(),
            ]
        }),
        ambient(),
    ])
}
