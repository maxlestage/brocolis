use yew::prelude::*;

use crate::dom::focus_id;
use crate::settings::use_t;

/// Premier élément focalisable de chaque page : il saute la barre de
/// navigation pour aller droit au contenu.
///
/// Le clic est intercepté plutôt que laissé au navigateur : `href="#contenu"`
/// écrirait dans le fragment d'URL, qui appartient au routeur, et renverrait
/// donc à l'accueil depuis la page de téléchargement.
#[function_component]
pub fn SkipLink() -> Html {
    let t = use_t();
    let onclick = Callback::from(|ev: MouseEvent| {
        ev.prevent_default();
        focus_id("contenu");
    });

    html! {
        <a class="skip-link" href="#contenu" {onclick}>{ t.nav_skip }</a>
    }
}
