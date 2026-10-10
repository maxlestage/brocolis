use active::html::*;
use active::{Node, fragment_dyn};

use crate::markup::Translated;
use crate::settings::t;

/// Manifeste : une phrase dont les mots s'allument au fil du défilement.
///
/// Chaque mot porte son rang (`--i`) ; `motion` écrit l'avancement de lecture
/// (`--p`, `data-progress="read"`) sur le bloc, et la CSS en déduit l'opacité,
/// la hauteur et le flou de chaque mot. Sans mouvement, la phrase est
/// simplement affichée en entier.
pub fn manifesto() -> Node {
    let words = fragment_dyn(|| {
        t().manifesto
            .split_whitespace()
            .enumerate()
            .flat_map(|(i, w)| {
                [
                    span()
                        .class("word")
                        .style(format!("--i: {i}"))
                        .text(w)
                        .into(),
                    " ".into(),
                ]
            })
            .collect()
    });

    div()
        .class("manifesto")
        .attr("data-progress", "read")
        .child(
            p().class("kicker")
                .attr("data-reveal", "")
                .attr("data-scramble", "")
                .tr(|t| t.manifesto_label),
        )
        .child(
            p().class("manifesto-text")
                .attr_dyn("style", || {
                    format!("--n: {}", t().manifesto.split_whitespace().count())
                })
                .child(words),
        )
        .into()
}
