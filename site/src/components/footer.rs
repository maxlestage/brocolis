use yew::prelude::*;

use crate::components::marquee::Marquee;
use crate::dom::scroll_to_top;
use crate::markup::{letters, raw, rolling};
use crate::route::Route;
use crate::settings::use_t;

#[derive(Properties, PartialEq)]
pub struct FooterProps {
    /// Appel final à télécharger. Absent de la page de téléchargement, où il
    /// renverrait vers elle-même.
    #[prop_or(true)]
    pub cta: bool,
}

/// Pied de page en affiche : bandeau évidé à contresens, appel final, mot-marque
/// géant dont les lettres se rassemblent en arrivant (`data-progress="enter"`)
/// et sautent au survol, bouton de retour en haut.
#[function_component]
pub fn Footer(props: &FooterProps) -> Html {
    let t = use_t();
    let to_top = Callback::from(|_: MouseEvent| scroll_to_top());

    html! {
        <footer class="footer">
            <Marquee reverse={true} outline={true} />
            if props.cta {
                <div class="container footer-cta">
                    <p class="footer-h" data-reveal="mask">{ raw(t.footer_h) }</p>
                    <a class="btn primary" data-magnetic="" href={Route::Download.href()}>
                        { rolling(t.nav_download) }
                        <span class="btn-arrow" aria-hidden="true">{ "→" }</span>
                    </a>
                </div>
            }
            <div class="footer-word" aria-hidden="true" data-progress="enter">
                { letters("Brocolis") }
            </div>
            <div class="container footer-meta">
                <p>{ t.footer }</p>
                <button type="button" class="to-top" data-magnetic="" onclick={to_top}>
                    <span class="to-top-arrow" aria-hidden="true">{ "↑" }</span>
                    { rolling(t.footer_top) }
                </button>
            </div>
        </footer>
    }
}
