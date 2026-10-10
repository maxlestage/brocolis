//! Maquettes de l'application de bureau, en HTML.
//!
//! Le site présente l'application plutôt que la ligne de commande : on y voit
//! la même fenêtre, les mêmes couleurs (fixes, quel que soit le thème du
//! site — c'est une image de l'application) et les mêmes mots.
//!
//! - [`app_bar`] et [`power_disc`] : la barre de titre et le grand
//!   interrupteur, repris par la section « Un clic ».
//! - [`section_app`] : la visite des trois onglets. Les onglets sont de vrais
//!   onglets (motif WAI-ARIA : flèches, Origine, Fin) ; l'interrupteur de
//!   l'onglet Protection se manipule. Le reste est un décor : des libellés,
//!   pas des boutons qui ne feraient rien.

use active::html::*;
use active::{Element, Node, State, fragment_dyn, use_state};
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::components::demo::token;
use crate::components::sections::head;
use crate::dom::document;
use crate::markup::{Translated, tr, words};
use crate::settings::t;

/// Barre de titre de la fenêtre : les trois pastilles et le nom.
pub fn app_bar() -> Node {
    div()
        .class("app-bar")
        .attr("aria-hidden", "true")
        .child(span().class("app-dots"))
        .child(span().class("app-name").text("Brocolis"))
        .into()
}

/// Le disque « marche/arrêt » et ses deux ondes, sans le bouton qui l'entoure.
pub fn power_disc() -> Node {
    Node::from([
        span().class("power-wave").attr("aria-hidden", "true"),
        span().class("power-wave").attr("aria-hidden", "true"),
        span()
            .class("power-disc")
            .attr("aria-hidden", "true")
            .child(
                svg()
                    .class("power-glyph")
                    .attr("viewBox", "0 0 48 48")
                    .child(path().attr("d", "M15.6 16 A13 13 0 1 0 32.4 16"))
                    .child(path().attr("d", "M24 8 V24")),
            ),
    ])
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
fn chips() -> Element {
    div()
        .class("mock-chips")
        .child(span().class("mock-chip").tr(|t| t.mock.me))
        .child(span().class("mock-chip on").text("✔ Alice"))
        .child(span().class("mock-chip").text("Bob"))
}

/// Choisit un onglet et lui donne le focus, comme une vraie barre d'onglets.
fn select(tab: State<usize>, i: usize) {
    tab.set(i);
    if let Some(el) = document()
        .get_element_by_id(&format!("app-tab-{i}"))
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = el.focus();
    }
}

pub fn section_app() -> Node {
    let tab = use_state(0usize);
    // Hors du panneau : l'interrupteur garde sa position d'un onglet à l'autre.
    let on = use_state(true);

    // Onglets au clavier : flèches gauche/droite (en boucle), Origine, Fin.
    let tabs = (0..3).map(|i| {
        let current = move || tab.get() == i;
        button()
            .attr("type", "button")
            .attr("role", "tab")
            .id(format!("app-tab-{i}"))
            .attr_dyn("aria-selected", move || current().to_string())
            .attr("aria-controls", "app-panel")
            .attr_dyn("tabindex", move || {
                if current() { "0" } else { "-1" }.to_owned()
            })
            .on_click(move |_| select(tab, i))
            .on("keydown", move |e| {
                let next = match e.key().as_str() {
                    "ArrowRight" => (i + 1) % 3,
                    "ArrowLeft" => (i + 2) % 3,
                    "Home" => 0,
                    "End" => 2,
                    _ => return,
                };
                e.prevent_default();
                select(tab, next);
            })
            .tr(move |t| t.app_tabs[i])
    });

    // Le panneau et la légende sont recréés à chaque changement d'onglet :
    // leurs animations d'entrée rejouent. Seules ces deux régions lisent
    // l'onglet courant.
    let panel = fragment_dyn(move || {
        let i = tab.get();
        vec![
            div()
                .class("app-panel")
                .id("app-panel")
                .attr("role", "tabpanel")
                .attr("aria-labelledby", format!("app-tab-{i}"))
                .child(panel_body(i, on))
                .into(),
        ]
    });
    let caption = fragment_dyn(move || {
        let i = tab.get();
        vec![
            p().class("app-caption")
                .attr("aria-live", "polite")
                .html_dyn(move || {
                    format!(
                        r#"<span class="app-caption-n" aria-hidden="true">{:02}</span>{}"#,
                        i + 1,
                        t().app_captions[i]
                    )
                })
                .into(),
        ]
    });

    section()
        .id("app")
        .attr("aria-labelledby", "app-h")
        .attr("data-progress", "")
        .child(head(5, "app-h", words(|t| t.app_h), "words"))
        .child(
            p().class("sec-intro")
                .attr("data-reveal", "blur")
                .tr(|t| t.app_intro),
        )
        .child(
            div()
                .class("app-tour")
                .child(
                    div()
                        .class("app-window")
                        .attr("data-reveal", "")
                        .child(app_bar())
                        .child(
                            div()
                                .class("app-body")
                                .child(
                                    div()
                                        .class("app-tabs")
                                        .attr("role", "tablist")
                                        .attr_tr("aria-label", |t| t.app_h)
                                        .children(tabs),
                                )
                                .child(panel),
                        ),
                )
                .child(caption),
        )
        .into()
}

/// Le contenu d'un onglet.
fn panel_body(tab: usize, on: State<bool>) -> Node {
    match tab {
        0 => div()
            .class("mock-protect")
            .child(
                button()
                    .attr("type", "button")
                    .class("power")
                    .bool_attr("data-on", move || on.get())
                    .attr_dyn("aria-pressed", move || on.get().to_string())
                    .on_click(move |_| on.update(|o| *o = !*o))
                    .child(power_disc())
                    .child(span().class("power-label").child(tr(move |t| {
                        if on.get() {
                            t.mock.deactivate
                        } else {
                            t.mock.activate
                        }
                    }))),
            )
            .child(
                p().class("mock-title")
                    .class_if("on", move || on.get())
                    .child(tr(move |t| {
                        if on.get() {
                            t.mock.on_title
                        } else {
                            t.mock.off_title
                        }
                    })),
            )
            .child(
                div()
                    .class("mock-card")
                    .child(p().class("mock-label").tr(|t| t.mock.browser_to))
                    .child(chips()),
            )
            .into(),
        1 => {
            let cards =
                CONTACTS.map(|(name, fp, key, browser)| {
                    div()
                        .class("mock-card")
                        .child(p().class("mock-name").text(name).child(
                            browser.then(|| span().class("mock-badge").tr(|t| t.mock.badge)),
                        ))
                        .child(p().class("mock-fp").text(fp))
                        .child(p().class("mock-key").text(key))
                        .child(
                            p().class("mock-btns")
                                .child(span().tr(|t| t.mock.copy))
                                .child(span().tr(|t| t.mock.change_key))
                                .child(span().tr(|t| t.mock.rename))
                                .child(span().tr(|t| t.mock.delete)),
                        )
                });
            div()
                .class("mock-contacts")
                .child(p().class("mock-h").tr(|t| t.mock.contacts_title))
                .children(cards)
                .child(span().class("mock-accent").tr(|t| t.mock.new_contact))
                .into()
        }
        _ => div()
            .class("mock-message")
            .child(p().class("mock-h").tr(|t| t.mock.msg_title))
            .child(
                p().class("mock-input").child(
                    span()
                        .class("mock-typed")
                        .attr_dyn("style", || {
                            format!("--n: {}", t().mock.message.chars().count())
                        })
                        .tr(|t| t.mock.message),
                ),
            )
            .child(
                div()
                    .class("mock-to")
                    .child(span().class("mock-label").tr(|t| t.mock.to))
                    .child(chips()),
            )
            .child(
                p().class("mock-btns")
                    .child(span().class("mock-accent").tr(|t| t.mock.encrypt))
                    .child(span().tr(|t| t.mock.decrypt)),
            )
            .child(p().class("mock-note").tr(|t| t.mock.result_note))
            .child(p().class("mock-result").text(token(7)))
            .into(),
    }
}
