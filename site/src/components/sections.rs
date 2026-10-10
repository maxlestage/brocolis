//! Les sections de l'accueil (la visite de l'application est dans
//! `app_mock.rs`).
//!
//! Chaque `<section>` est nommée par son propre titre via `aria-labelledby` :
//! sans ça une section n'est pas exposée comme région et disparaît de la liste
//! des repères que proposent les lecteurs d'écran.
//!
//! Attributs lus par `motion` et la feuille de style (voir `motion/mod.rs`) :
//! - `data-reveal` : apparaît en entrant dans la vue — `mask` se dévoile de bas
//!   en haut, `words` fait monter les mots un à un, `blur` sort du flou,
//!   `left` glisse depuis la gauche, `card` bascule en 3D ;
//! - `data-progress` sur la section : son avancement (`--p`) trace le filet du
//!   haut et fait glisser le numéro fantôme ;
//! - `data-scramble` : les petits libellés se déchiffrent ;
//! - `data-tilt`, `data-spot` : cartes inclinées et éclairées sous le pointeur ;
//! - `data-magnetic` : boutons attirés par le pointeur.

use active::html::*;
use active::{Element, Node, fragment_dyn};

use crate::components::app_mock::{app_bar, power_disc};
use crate::i18n::FR;
use crate::markup::{Pick, Translated, rolling, words};
use crate::route::Route;
use crate::settings::t;

/// En-tête de section : numéro, titre, et numéro fantôme géant en arrière-plan.
/// Les numéros sont décoratifs — l'ordre des sections est déjà celui du
/// document.
pub(crate) fn head(index: u8, id: &'static str, title: Node, reveal: &'static str) -> Node {
    Node::from([
        span()
            .class("sec-ghost")
            .attr("aria-hidden", "true")
            .text(format!("{index:02}")),
        div()
            .class("sec-head")
            .child(
                span()
                    .class("sec-index")
                    .attr("aria-hidden", "true")
                    .attr("data-scramble", "")
                    .text(format!("{index:02}")),
            )
            .child(h2().id(id).attr("data-reveal", reveal).child(title)),
    ])
}

/// Une section de l'accueil, nommée par son titre `{id}-h`.
fn section_shell(id: &'static str) -> Element {
    section()
        .id(id)
        .attr("aria-labelledby", format!("{id}-h"))
        .attr("data-progress", "")
}

/// Coche tracée au trait quand l'élément de liste apparaît. Écrite en
/// balisage : elle précède le texte traduit dans le même `<li>`.
const TICK: &str = r#"<svg class="tick" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5"></path></svg>"#;

/// Élément de liste qui glisse depuis la gauche, coché ou non.
fn item(ticked: bool, pick: impl Pick) -> Element {
    li().attr("data-reveal", "left").html_dyn(move || {
        let text = pick(t());
        if ticked {
            format!("{TICK}{text}")
        } else {
            text.to_owned()
        }
    })
}

/// Bloc de terminal dont les lignes apparaissent l'une après l'autre. Les
/// sauts de ligne restent du texte entre les lignes : le contenu copié est
/// identique au texte d'origine.
fn terminal_lines(pick: impl Pick) -> Node {
    fragment_dyn(move || {
        let lines: Vec<&str> = pick(t()).split('\n').collect();
        let last = lines.len().saturating_sub(1);
        let mut nodes = Vec::with_capacity(lines.len() * 2);
        for (i, line) in lines.into_iter().enumerate() {
            nodes.push(
                span()
                    .class("line")
                    .style(format!("--i: {i}"))
                    .text(line)
                    .into(),
            );
            if i < last {
                nodes.push("\n".into());
            }
        }
        nodes
    })
}

pub fn section_platforms() -> Node {
    // Le pictogramme ne dépend pas de la langue : lu une fois, dans
    // n'importe laquelle.
    let cards = FR
        .platform_cards()
        .into_iter()
        .enumerate()
        .map(|(i, card)| {
            article()
                .class("card platform-card")
                .attr("data-reveal", "card")
                .attr("data-tilt", "")
                .attr("data-spot", "")
                .child(
                    div()
                        .class("platform-head")
                        .child(
                            span()
                                .class("platform-icon")
                                .attr("aria-hidden", "true")
                                .text(card.icon),
                        )
                        .child(h3().tr(move |t| t.platform_cards()[i].title)),
                )
                .child(p().raw(move |t| t.platform_cards()[i].desc))
                .child(
                    a().class("btn primary platform-cta")
                        .attr("data-magnetic", "")
                        .href(Route::Download.href())
                        .child(rolling(move |t| t.platform_cards()[i].cta))
                        .child(
                            span()
                                .class("btn-arrow")
                                .attr("aria-hidden", "true")
                                .text("→"),
                        ),
                )
        });

    section_shell("platforms")
        .child(head(1, "platforms-h", words(|t| t.platforms_h), "words"))
        .child(
            p().class("sec-intro")
                .attr("data-reveal", "blur")
                .raw(|t| t.platforms_intro),
        )
        .child(div().class("grid grid-3").children(cards))
        .into()
}

/// « Un clic » : l'interrupteur de l'application, qui s'allume quand la
/// section entre dans la vue, et ce que ce clic fait sur le système.
pub fn section_switch() -> Node {
    let items = (0..6).map(|i| item(true, move |t| t.s1_li[i]));

    section_shell("switch")
        .child(head(2, "switch-h", words(|t| t.s1_h), "words"))
        .child(
            div()
                .class("switch-layout")
                // Une image, pour les lecteurs d'écran : la maquette n'est pas
                // un bouton, elle montre celui de l'application.
                .child(
                    div()
                        .class("app-window switch-mock")
                        .attr("role", "img")
                        .attr_tr("aria-label", |t| t.switch_sr)
                        .attr("data-reveal", "")
                        .child(app_bar())
                        .child(
                            div()
                                .class("app-body switch-stage")
                                .attr("aria-hidden", "true")
                                .child(
                                    span()
                                        .class("power")
                                        .attr("data-on", "")
                                        .child(power_disc())
                                        .child(
                                            span().class("power-label").tr(|t| t.mock.deactivate),
                                        ),
                                )
                                .child(
                                    p().class("power-state")
                                        .child(span().class("when-off").tr(|t| t.mock.off_title))
                                        .child(span().class("when-on").tr(|t| t.mock.on_title)),
                                ),
                        ),
                )
                .child(ul().class("checklist").children(items)),
        )
        .child(
            p().class("note")
                .attr("data-reveal", "blur")
                .raw(|t| t.s1_note),
        )
        .into()
}

pub fn section_features() -> Node {
    let cards = (0..4).map(|i| {
        article()
            .class("card feature-card")
            .attr("data-reveal", "card")
            .attr("data-tilt", "")
            .attr("data-spot", "")
            .child(
                span()
                    .class("card-index")
                    .attr("aria-hidden", "true")
                    .attr("data-scramble", "")
                    .text(format!("{:02}", i + 1)),
            )
            .child(h3().tr(move |t| t.s2_cards[i].title))
            .child(p().raw(move |t| t.s2_cards[i].body))
    });

    section_shell("features")
        .child(head(3, "features-h", words(|t| t.s2_h), "words"))
        .child(div().class("grid").children(cards))
        .into()
}

pub fn section_quickstart() -> Node {
    let items = (0..4).map(|i| item(false, move |t| t.s3_li[i]));

    section_shell("quickstart")
        .child(head(4, "quickstart-h", words(|t| t.s3_h), "words"))
        .child(ol().class("steps").children(items))
        .into()
}

pub fn section_how_it_works() -> Node {
    section_shell("how")
        .child(head(6, "how-h", words(|t| t.s5_h), "words"))
        .child(p().attr("data-reveal", "blur").tr(|t| t.s5_intro))
        // Un schéma en art ASCII déborde forcément sur téléphone : il est donc
        // défilable, et `tabindex` le rend atteignable au clavier — sans quoi
        // ce qui dépasse serait hors d'atteinte sans souris.
        .child(
            pre()
                .class("terminal diagram lines")
                .attr("tabindex", "0")
                .attr("role", "img")
                .attr_tr("aria-label", |t| t.s5_h)
                .attr("data-reveal", "")
                .child(code().child(terminal_lines(|t| t.s5_diagram))),
        )
        .into()
}

pub fn section_privacy() -> Node {
    let items = (0..5).map(|i| item(true, move |t| t.s6_li[i]));

    section_shell("privacy")
        .child(head(7, "privacy-h", words(|t| t.s6_h), "words"))
        .child(ul().class("checklist").children(items))
        .into()
}
