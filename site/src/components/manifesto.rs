use yew::prelude::*;

use crate::settings::use_t;

/// Manifeste : une phrase dont les mots s'allument au fil du défilement.
///
/// Chaque mot porte son rang (`--i`) ; `motion` écrit l'avancement (`--p`) sur
/// le bloc, et la CSS en déduit l'opacité de chaque mot. Sans mouvement, la
/// phrase est simplement affichée en entier.
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
        <div class="manifesto">
            <p class="kicker" data-reveal="">{ t.manifesto_label }</p>
            <p class="manifesto-text" style={format!("--n: {n}")}>{ for spans }</p>
        </div>
    }
}
