use yew::prelude::*;

use crate::settings::use_t;

#[derive(Properties, PartialEq)]
pub struct MarqueeProps {
    /// Défile dans l'autre sens.
    #[prop_or(false)]
    pub reverse: bool,
    /// Lettres évidées (contour seul) plutôt que pleines.
    #[prop_or(false)]
    pub outline: bool,
    /// Mots affichés ; par défaut, les mots-clés de la langue courante.
    #[prop_or_default]
    pub words: Option<Vec<&'static str>>,
}

/// Bandeau défilant.
///
/// La piste est doublée pour boucler sans couture : `motion` la translate
/// modulo la largeur d'une copie. Sa vitesse suit celle du défilement de la
/// page, et son sens s'inverse quand on remonte ; les lettres s'inclinent avec
/// la vitesse (`--velocity`). Décoratif — chaque mot est repris ailleurs —,
/// donc caché aux lecteurs d'écran, qui sinon le liraient deux fois.
#[function_component]
pub fn Marquee(props: &MarqueeProps) -> Html {
    let t = use_t();
    let list: Vec<&'static str> = props.words.clone().unwrap_or_else(|| t.marquee.to_vec());
    let run = || {
        list.iter().map(|&word| {
            html! {
                <>
                    <span class="marquee-word">{ word }</span>
                    <span class="marquee-sep">{ "✳" }</span>
                </>
            }
        })
    };

    html! {
        <div
            class={classes!("marquee", props.outline.then_some("outline"))}
            data-marquee={if props.reverse { "-1" } else { "1" }}
            aria-hidden="true"
        >
            <div class="marquee-track">
                <div class="marquee-run">{ for run() }</div>
                <div class="marquee-run">{ for run() }</div>
            </div>
        </div>
    }
}
