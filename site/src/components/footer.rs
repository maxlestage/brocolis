use active::Node;
use active::html::*;

use crate::components::marquee::marquee;
use crate::dom::scroll_to_top;
use crate::markup::{Translated, letters, rolling};
use crate::route::Route;

/// Pied de page en affiche : bandeau évidé à contresens, appel final, mot-marque
/// géant dont les lettres se rassemblent en arrivant (`data-progress="enter"`)
/// et sautent au survol, bouton de retour en haut.
///
/// `cta` : l'appel final à télécharger. Absent de la page de téléchargement,
/// où il renverrait vers elle-même.
pub fn site_footer(cta: bool) -> Node {
    let call = cta.then(|| {
        div()
            .class("container footer-cta")
            .child(
                p().class("footer-h")
                    .attr("data-reveal", "mask")
                    .raw(|t| t.footer_h),
            )
            .child(
                a().class("btn primary")
                    .attr("data-magnetic", "")
                    .href(Route::Download.href())
                    .child(rolling(|t| t.nav_download))
                    .child(
                        span()
                            .class("btn-arrow")
                            .attr("aria-hidden", "true")
                            .text("→"),
                    ),
            )
    });

    footer()
        .class("footer")
        .child(marquee(true, true, None))
        .child(call)
        .child(
            div()
                .class("footer-word")
                .attr("aria-hidden", "true")
                .attr("data-progress", "enter")
                .children(letters("Brocolis")),
        )
        .child(
            div()
                .class("container footer-meta")
                .child(p().tr(|t| t.footer))
                .child(
                    button()
                        .attr("type", "button")
                        .class("to-top")
                        .attr("data-magnetic", "")
                        .on_click(|_| scroll_to_top())
                        .child(
                            span()
                                .class("to-top-arrow")
                                .attr("aria-hidden", "true")
                                .text("↑"),
                        )
                        .child(rolling(|t| t.footer_top)),
                ),
        )
        .into()
}
