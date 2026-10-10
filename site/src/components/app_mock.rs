//! Maquettes de l'application de bureau, en HTML.
//!
//! Le site présente l'application plutôt que la ligne de commande : on y voit
//! la même fenêtre, les mêmes couleurs (fixes, quel que soit le thème du
//! site — c'est une image de l'application) et les mêmes mots.
//!
//! - [`app_bar`] et [`power`] : la barre de titre et le grand interrupteur,
//!   repris par la section « Un clic ».
//! - [`SectionApp`] : la visite des trois onglets. Les onglets sont de vrais
//!   onglets (motif WAI-ARIA : flèches, Origine, Fin) ; l'interrupteur de
//!   l'onglet Protection se manipule. Le reste est un décor : des libellés,
//!   pas des boutons qui ne feraient rien.

use web_sys::HtmlElement;
use yew::prelude::*;

use crate::components::demo::token;
use crate::markup::{raw, words};
use crate::settings::use_t;

/// Barre de titre de la fenêtre : les trois pastilles et le nom.
pub fn app_bar() -> Html {
    html! {
        <div class="app-bar" aria-hidden="true">
            <span class="app-dots" />
            <span class="app-name">{ "Brocolis" }</span>
        </div>
    }
}

/// Le disque « marche/arrêt » et ses deux ondes, sans le bouton qui l'entoure.
pub fn power_disc() -> Html {
    html! {
        <>
            <span class="power-wave" aria-hidden="true" />
            <span class="power-wave" aria-hidden="true" />
            <span class="power-disc" aria-hidden="true">
                <svg class="power-glyph" viewBox="0 0 48 48">
                    <path d="M15.6 16 A13 13 0 1 0 32.4 16" />
                    <path d="M24 8 V24" />
                </svg>
            </span>
        </>
    }
}

/// Les contacts de la maquette : nom, empreinte, clé masquée, destinataire
/// du navigateur ou non.
const CONTACTS: [(&str, &str, &str, bool); 2] = [
    (
        "Alice",
        "4551-E6C8-D1F7-51B4",
        "XUtoh1••••••••••••E2s=",
        true,
    ),
    (
        "Bob",
        "A4C2-F284-61D7-074C",
        "3UxkkR••••••••••••o1w=",
        false,
    ),
];

/// Pastilles de destinataires : vous, Alice (choisie), Bob.
fn chips(me: &str) -> Html {
    html! {
        <div class="mock-chips">
            <span class="mock-chip">{ me.to_owned() }</span>
            <span class="mock-chip on">{ "✔ Alice" }</span>
            <span class="mock-chip">{ "Bob" }</span>
        </div>
    }
}

#[function_component]
pub fn SectionApp() -> Html {
    let t = use_t();
    let m = &t.mock;
    let tab = use_state(|| 0usize);
    let on = use_state(|| true);
    let refs = [use_node_ref(), use_node_ref(), use_node_ref()];

    // Onglets au clavier : flèches gauche/droite (en boucle), Origine, Fin.
    // Le focus suit l'onglet choisi, comme dans une vraie barre d'onglets.
    let select = {
        let tab = tab.clone();
        let refs = refs.clone();
        move |i: usize| {
            tab.set(i);
            if let Some(el) = refs[i].cast::<HtmlElement>() {
                let _ = el.focus();
            }
        }
    };
    let tabs = t.app_tabs.iter().enumerate().map(|(i, &label)| {
        let current = *tab == i;
        let onclick = {
            let select = select.clone();
            Callback::from(move |_: MouseEvent| select(i))
        };
        let onkeydown = {
            let select = select.clone();
            Callback::from(move |e: KeyboardEvent| {
                let next = match e.key().as_str() {
                    "ArrowRight" => (i + 1) % 3,
                    "ArrowLeft" => (i + 2) % 3,
                    "Home" => 0,
                    "End" => 2,
                    _ => return,
                };
                e.prevent_default();
                select(next);
            })
        };
        html! {
            <button
                type="button"
                role="tab"
                id={format!("app-tab-{i}")}
                aria-selected={current.to_string()}
                aria-controls="app-panel"
                tabindex={if current { "0" } else { "-1" }}
                ref={refs[i].clone()}
                {onclick}
                {onkeydown}
            >
                { label }
            </button>
        }
    });

    let toggle = {
        let on = on.clone();
        Callback::from(move |_: MouseEvent| on.set(!*on))
    };

    let panel = match *tab {
        0 => html! {
            <div class="mock-protect">
                <button
                    type="button"
                    class="power"
                    data-on={(*on).then_some("")}
                    aria-pressed={on.to_string()}
                    onclick={toggle}
                >
                    { power_disc() }
                    <span class="power-label">{ if *on { m.deactivate } else { m.activate } }</span>
                </button>
                <p class={classes!("mock-title", (*on).then_some("on"))}>
                    { if *on { m.on_title } else { m.off_title } }
                </p>
                <div class="mock-card">
                    <p class="mock-label">{ m.browser_to }</p>
                    { chips(m.me) }
                </div>
            </div>
        },
        1 => {
            let cards = CONTACTS.iter().map(|&(name, fp, key, browser)| {
                html! {
                    <div class="mock-card">
                        <p class="mock-name">
                            { name }
                            if browser { <span class="mock-badge">{ m.badge }</span> }
                        </p>
                        <p class="mock-fp">{ fp }</p>
                        <p class="mock-key">{ key }</p>
                        <p class="mock-btns">
                            <span>{ m.copy }</span>
                            <span>{ m.change_key }</span>
                            <span>{ m.rename }</span>
                            <span>{ m.delete }</span>
                        </p>
                    </div>
                }
            });
            html! {
                <div class="mock-contacts">
                    <p class="mock-h">{ m.contacts_title }</p>
                    { for cards }
                    <span class="mock-accent">{ m.new_contact }</span>
                </div>
            }
        }
        _ => html! {
            <div class="mock-message">
                <p class="mock-h">{ m.msg_title }</p>
                <p class="mock-input">
                    <span class="mock-typed" style={format!("--n: {}", m.message.chars().count())}>
                        { m.message }
                    </span>
                </p>
                <div class="mock-to">
                    <span class="mock-label">{ m.to }</span>
                    { chips(m.me) }
                </div>
                <p class="mock-btns">
                    <span class="mock-accent">{ m.encrypt }</span>
                    <span>{ m.decrypt }</span>
                </p>
                <p class="mock-note">{ m.result_note }</p>
                <p class="mock-result">{ token(7) }</p>
            </div>
        },
    };

    html! {
        <section id="app" aria-labelledby="app-h" data-progress="">
            { crate::components::sections::head(5, "app-h", words(t.app_h), "words") }
            <p class="sec-intro" data-reveal="blur">{ t.app_intro }</p>
            <div class="app-tour">
                <div class="app-window" data-reveal="">
                    { app_bar() }
                    <div class="app-body">
                        <div class="app-tabs" role="tablist" aria-label={t.app_h}>
                            { for tabs }
                        </div>
                        // Une clé par onglet : le panneau est recréé, et ses
                        // animations d'entrée rejouent à chaque changement.
                        <div
                            class="app-panel"
                            id="app-panel"
                            role="tabpanel"
                            aria-labelledby={format!("app-tab-{}", *tab)}
                            key={tab.to_string()}
                        >
                            { panel }
                        </div>
                    </div>
                </div>
                <p class="app-caption" aria-live="polite" key={format!("legende-{}", *tab)}>
                    <span class="app-caption-n" aria-hidden="true">{ format!("{:02}", *tab + 1) }</span>
                    { raw(t.app_captions[*tab]) }
                </p>
            </div>
        </section>
    }
}
