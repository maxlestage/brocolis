use yew::prelude::*;

use crate::markup::raw;
use crate::route::Route;
use crate::settings::use_t;

#[derive(Properties, PartialEq)]
pub struct FooterProps {
    /// Appel final à télécharger. Absent de la page de téléchargement, où il
    /// renverrait vers elle-même.
    #[prop_or(true)]
    pub cta: bool,
}

/// Pied de page en affiche : appel final, mot-marque géant, attribution.
#[function_component]
pub fn Footer(props: &FooterProps) -> Html {
    let t = use_t();

    html! {
        <footer class="footer">
            if props.cta {
                <div class="container footer-cta">
                    <p class="footer-h" data-reveal="mask">{ raw(t.footer_h) }</p>
                    <a class="btn primary" data-magnetic="" href={Route::Download.href()}>
                        <span>{ t.nav_download }</span>
                        <span class="btn-arrow" aria-hidden="true">{ "→" }</span>
                    </a>
                </div>
            }
            <div class="footer-word" aria-hidden="true">
                <span data-speed="-0.12">{ "Brocolis" }</span>
            </div>
            <div class="container footer-meta">
                <p>{ t.footer }</p>
            </div>
        </footer>
    }
}
