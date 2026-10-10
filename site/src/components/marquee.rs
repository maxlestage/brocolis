use active::Node;
use active::html::*;

use crate::markup::tr;

/// Bandeau défilant.
///
/// - `reverse` : défile dans l'autre sens ;
/// - `outline` : lettres évidées (contour seul) plutôt que pleines ;
/// - `words` : mots affichés ; par défaut, les mots-clés de la langue courante.
///
/// La piste est doublée pour boucler sans couture : `motion` la translate
/// modulo la largeur d'une copie. Sa vitesse suit celle du défilement de la
/// page, et son sens s'inverse quand on remonte ; les lettres s'inclinent avec
/// la vitesse (`--velocity`). Décoratif — chaque mot est repris ailleurs —,
/// donc caché aux lecteurs d'écran, qui sinon le liraient deux fois.
pub fn marquee(reverse: bool, outline: bool, words: Option<&'static [&'static str]>) -> Node {
    let run = || {
        let words: Vec<Node> = match words {
            Some(list) => list.iter().map(|&w| Node::from(w)).collect(),
            None => (0..6).map(|i| tr(move |t| t.marquee[i])).collect(),
        };
        div()
            .class("marquee-run")
            .children(words.into_iter().flat_map(|word| {
                [
                    span().class("marquee-word").child(word),
                    span().class("marquee-sep").text("✳"),
                ]
            }))
    };

    div()
        .class("marquee")
        .class(if outline { "outline" } else { "" })
        .attr("data-marquee", if reverse { "-1" } else { "1" })
        .attr("aria-hidden", "true")
        .child(div().class("marquee-track").child(run()).child(run()))
        .into()
}
