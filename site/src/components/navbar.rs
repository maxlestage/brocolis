use active::html::*;
use active::{Node, use_state};
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::components::switchers::switchers;
use crate::dom::{Listener, scroll_to_id, scroll_to_top};
use crate::i18n::{FR, Strings};
use crate::markup::{Translated, keep, rolling};
use crate::route::Route;

/// Barre de navigation de l'accueil : tiroir replié sur téléphone, menu
/// horizontal à partir de 960px (la CSS décide, pas le JS).
pub fn navbar() -> Node {
    let open = use_state(false);

    // Échap referme le tiroir. L'écouteur vit aussi longtemps que la barre :
    // il est retiré avec la page (`Drop` sur `Listener`).
    keep(Listener::on_window("keydown", move |ev| {
        let escape = ev
            .dyn_ref::<KeyboardEvent>()
            .is_some_and(|k| k.key() == "Escape");
        if escape && open.get() {
            open.set(false);
        }
    }));

    // Le fragment d'URL appartient au routeur : un `href="#platforms"` qui
    // aboutirait changerait de route. On défile donc nous-mêmes, et la douceur
    // vient de la CSS.
    //
    // L'identifiant des sections ne dépend pas de la langue : il est lu une
    // fois, dans n'importe laquelle. Seul le libellé est lié à la langue
    // courante.
    let links = FR
        .nav_items()
        .into_iter()
        .enumerate()
        .map(move |(i, item)| {
            let id = item.id;
            let label = move |t: &'static Strings| t.nav_items()[i].label;
            li().style(format!("--i: {i}")).child(
                a().href(format!("#{id}"))
                    .on_click(move |ev| {
                        ev.prevent_default();
                        open.set(false);
                        scroll_to_id(id);
                    })
                    .child(rolling(label)),
            )
        });

    nav()
        .class("navbar")
        .attr_tr("aria-label", |t| t.nav_menu)
        .child(
            div()
                .class("container nav-inner")
                .child(
                    a().class("nav-brand")
                        .href("#top")
                        .on_click(move |ev| {
                            ev.prevent_default();
                            open.set(false);
                            scroll_to_top();
                        })
                        .child(
                            span()
                                .class("nav-leaf")
                                .attr("aria-hidden", "true")
                                .text("🥦"),
                        )
                        .child(span().attr("data-scramble", "").text("Brocolis")),
                )
                .child(
                    button()
                        .attr("type", "button")
                        .class("nav-toggle")
                        .class_if("open", move || open.get())
                        .attr_tr("aria-label", |t| t.nav_menu)
                        .attr_dyn("aria-expanded", move || open.get().to_string())
                        .attr("aria-controls", "nav-drawer")
                        .on_click(move |_| open.update(|o| *o = !*o))
                        .children([span(), span(), span()]),
                )
                .child(
                    div()
                        .id("nav-drawer")
                        .class("nav-drawer")
                        .class_if("open", move || open.get())
                        .child(
                            ul().class("nav-links").children(links).child(
                                li().style("--i: 5").child(
                                    a().class("nav-download")
                                        .href(Route::Download.href())
                                        .on_click(move |_| open.set(false))
                                        .child(rolling(|t| t.nav_download)),
                                ),
                            ),
                        )
                        .child(div().class("nav-switchers").child(switchers())),
                ),
        )
        .into()
}
