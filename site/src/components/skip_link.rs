use active::Node;
use active::html::*;

use crate::dom::focus_id;
use crate::markup::Translated;

/// Premier élément focalisable de chaque page : il saute la barre de
/// navigation pour aller droit au contenu.
///
/// Le clic est intercepté plutôt que laissé au navigateur : `href="#contenu"`
/// écrirait dans le fragment d'URL, qui appartient au routeur, et renverrait
/// donc à l'accueil depuis la page de téléchargement.
pub fn skip_link() -> Node {
    a().class("skip-link")
        .href("#contenu")
        .on_click(|ev| {
            ev.prevent_default();
            focus_id("contenu");
        })
        .tr(|t| t.nav_skip)
        .into()
}
