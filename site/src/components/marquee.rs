use yew::prelude::*;

use crate::settings::use_t;

/// Bandeau défilant entre le héros et le contenu.
///
/// La piste est doublée pour boucler sans couture : l'animation la translate
/// de -50 %, soit exactement une copie. Décoratif (chaque mot est repris
/// ailleurs dans la page), donc caché aux lecteurs d'écran — qui sinon le
/// liraient deux fois.
#[function_component]
pub fn Marquee() -> Html {
    let t = use_t();
    let run = || {
        t.marquee.iter().map(|&word| {
            html! {
                <>
                    <span class="marquee-word">{ word }</span>
                    <span class="marquee-sep">{ "✳" }</span>
                </>
            }
        })
    };

    html! {
        <div class="marquee" aria-hidden="true">
            <div class="marquee-track">
                <div class="marquee-run">{ for run() }</div>
                <div class="marquee-run">{ for run() }</div>
            </div>
        </div>
    }
}
