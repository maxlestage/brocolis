use yew::prelude::*;

use crate::markup::raw;
use crate::route::Route;
use crate::settings::use_t;

/// En-tête de l'accueil, en deux temps.
///
/// 1. Une scène épinglée (`.hero-pin`, plus haute que l'écran, dont l'intérieur
///    reste collé en haut) : le titre géant, et la mascotte qui se forme dans
///    la poussière à mesure qu'on défile, puis qu'un balayage de lumière
///    traverse. La poussière est dessinée par `motion` dans le canevas.
/// 2. L'accroche, le texte d'introduction et l'appel à télécharger, qui
///    apparaissent ensuite dans le flux normal.
///
/// Sans mouvement, la scène n'est pas épinglée et l'image est affichée
/// directement : le canevas reste vide et transparent.
#[function_component]
pub fn Hero() -> Html {
    let t = use_t();

    // Une lettre par `<span>` : chacune monte de derrière sa ligne avec un
    // léger décalage sur la précédente. Le titre reste un seul mot pour les
    // lecteurs d'écran (contenu textuel intact, aucune espace ajoutée).
    let letters = "Brocolis".chars().enumerate().map(|(i, c)| {
        html! { <span class="char" style={format!("--i: {i}")}>{ c }</span> }
    });

    html! {
        <header id="top" class="hero">
            <div class="hero-pin">
                <div class="hero-stage">
                    <canvas class="hero-dust" aria-hidden="true" />
                    <p class="hero-eyebrow">
                        <span class="pulse" aria-hidden="true" />
                        { t.hero_eyebrow }
                    </p>
                    <div class="hero-art">
                        // Image la plus visible de la page : chargée en
                        // priorité et dimensionnée en dur, pour qu'elle ne
                        // décale rien en arrivant.
                        <img
                            src="./brocolisMan.png"
                            alt={t.hero_alt}
                            width="440"
                            height="440"
                            decoding="async"
                            fetchpriority="high"
                        />
                        <span class="hero-sweep" aria-hidden="true" />
                    </div>
                    <h1 class="hero-title">{ for letters }</h1>
                    <p class="hero-scroll" aria-hidden="true">
                        <span>{ t.hero_scroll }</span>
                        <span class="hero-scroll-line" />
                    </p>
                </div>
            </div>
            <div class="container hero-intro">
                <p class="tagline" data-reveal="mask">{ raw(t.hero_tagline) }</p>
                <p class="lead" data-reveal="">{ raw(t.hero_lead) }</p>
                <div class="cta" data-reveal="">
                    <a class="btn primary" data-magnetic="" href={Route::Download.href()}>
                        <span>{ t.hero_cta }</span>
                        <span class="btn-arrow" aria-hidden="true">{ "→" }</span>
                    </a>
                    <span class="cta-sub">{ t.hero_cta_sub }</span>
                </div>
            </div>
        </header>
    }
}
