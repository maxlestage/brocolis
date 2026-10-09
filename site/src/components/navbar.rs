use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;
use yew::prelude::*;

use crate::components::switchers::Switchers;
use crate::dom::{Listener, scroll_to_id, scroll_to_top};
use crate::route::Route;
use crate::settings::use_t;

/// Barre de navigation de l'accueil : tiroir replié sur téléphone, menu
/// horizontal à partir de 960px (la CSS décide, pas le JS).
#[function_component]
pub fn Navbar() -> Html {
    let t = use_t();
    let open = use_state(|| false);

    // Échap referme le tiroir. L'écouteur n'existe que tant qu'il est ouvert —
    // il se retire tout seul à la fermeture (`Drop` sur `Listener`).
    {
        let open = open.clone();
        use_effect_with(*open, move |is_open| {
            let listener = is_open.then(|| {
                Listener::on_window("keydown", move |ev| {
                    let escape = ev
                        .dyn_ref::<KeyboardEvent>()
                        .is_some_and(|k| k.key() == "Escape");
                    if escape {
                        open.set(false);
                    }
                })
            });
            move || drop(listener)
        });
    }

    // Le fragment d'URL appartient au routeur : un `href="#platforms"` qui
    // aboutirait changerait de route. On défile donc nous-mêmes, et la douceur
    // vient de la CSS.
    let links = t.nav_items().into_iter().enumerate().map(|(i, item)| {
        let id = item.id;
        let onclick = {
            let open = open.clone();
            Callback::from(move |ev: MouseEvent| {
                ev.prevent_default();
                open.set(false);
                scroll_to_id(id);
            })
        };
        html! {
            <li style={format!("--i: {i}")}>
                <a href={format!("#{id}")} {onclick}>{ item.label }</a>
            </li>
        }
    });

    let on_brand = {
        let open = open.clone();
        Callback::from(move |ev: MouseEvent| {
            ev.prevent_default();
            open.set(false);
            scroll_to_top();
        })
    };

    let on_toggle = {
        let open = open.clone();
        Callback::from(move |_: MouseEvent| open.set(!*open))
    };

    let on_download = {
        let open = open.clone();
        Callback::from(move |_: MouseEvent| open.set(false))
    };

    html! {
        <nav class="navbar" aria-label={t.nav_menu}>
            <div class="container nav-inner">
                <a class="nav-brand" href="#top" onclick={on_brand}>
                    <span class="nav-leaf" aria-hidden="true">{ "🥦" }</span>
                    <span>{ "Brocolis" }</span>
                </a>
                <button
                    type="button"
                    class={classes!("nav-toggle", open.then_some("open"))}
                    aria-label={t.nav_menu}
                    aria-expanded={open.to_string()}
                    aria-controls="nav-drawer"
                    onclick={on_toggle}
                >
                    <span /><span /><span />
                </button>
                <div id="nav-drawer" class={classes!("nav-drawer", open.then_some("open"))}>
                    <ul class="nav-links">
                        { for links }
                        <li style="--i: 5">
                            <a
                                class="nav-download"
                                href={Route::Download.href()}
                                onclick={on_download}
                            >
                                { t.nav_download }
                            </a>
                        </li>
                    </ul>
                    <div class="nav-switchers">
                        <Switchers />
                    </div>
                </div>
            </div>
        </nav>
    }
}
