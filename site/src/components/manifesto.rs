use yew::prelude::*;

use crate::settings::use_t;

/// Manifeste : une phrase dont les mots s'allument au fil du défilement.
///
/// Chaque mot porte son rang (`--i`) ; `motion` écrit l'avancement de lecture
/// (`--p`, `data-progress="read"`) sur le bloc, et la CSS en déduit l'opacité,
/// la hauteur et le flou de chaque mot. Sans mouvement, la phrase est
/// simplement affichée en entier.
#[function_component]
pub fn Manifesto() -> Html {
    let t = use_t();
    let words: Vec<&str> = t.manifesto.split_whitespace().collect();
    let n = words.len();
    let spans = words.into_iter().enumerate().map(|(i, w)| {
        html! {
            <>
                <span class="word" style={format!("--i: {i}")}>{ w }</span>
                { " " }
            </>
        }
    });

    html! {
        <div class="manifesto" data-progress="read">
            <p class="kicker" data-reveal="" data-scramble="">{ t.manifesto_label }</p>
            <p class="manifesto-text" style={format!("--n: {n}")}>{ for spans }</p>
        </div>
    }
}
