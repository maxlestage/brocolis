use yew::prelude::*;

use crate::markup::{letters, raw, rolling};
use crate::route::Route;
use crate::settings::use_t;

/// En-tête de l'accueil, en deux temps.
///
/// 1. Une scène épinglée (`.hero-pin`, plus haute que l'écran, dont l'intérieur
///    reste collé en haut) : le titre géant, et la mascotte qui se forme dans
///    la poussière à mesure qu'on défile, puis qu'un balayage de lumière
///    traverse. La poussière est dessinée par `motion` dans le canevas ; les
///    lettres du titre se dispersent pendant la course, la mascotte suit le
///    pointeur, un badge circulaire tourne en invitant à défiler.
/// 2. L'accroche, le texte d'introduction et l'appel à télécharger, qui
///    apparaissent ensuite dans le flux normal.
///
/// Sans mouvement, la scène n'est pas épinglée et l'image est affichée
/// directement : le canevas reste vide et transparent.
#[function_component]
pub fn Hero() -> Html {
    let t = use_t();
    let scroll = t.hero_scroll.to_uppercase();
    let badge = format!("{scroll} · {scroll} · {scroll} · ");

    html! {
        <header id="top" class="hero">
            <div class="hero-pin">
                <div class="hero-stage" data-cursor={scroll.clone()}>
                    <span class="hero-glow" aria-hidden="true" />
                    <canvas class="hero-dust" aria-hidden="true" />
                    <p class="hero-eyebrow">
                        <span class="pulse" aria-hidden="true" />
                        <span data-scramble="">{ t.hero_eyebrow }</span>
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
                        <span class="hero-orbit" aria-hidden="true" />
                    </div>
                    // Une lettre par `<span>` : chacune monte de derrière sa
                    // ligne avec un léger décalage sur la précédente. Le titre
                    // reste un seul mot pour les lecteurs d'écran (contenu
                    // textuel intact, aucune espace ajoutée).
                    <h1 class="hero-title">{ letters("Brocolis") }</h1>
                    <div class="hero-badge" aria-hidden="true">
                        <svg viewBox="0 0 120 120">
                            <defs>
                                <path
                                    id="hero-badge-path"
                                    d="M60,60 m-47,0 a47,47 0 1,1 94,0 a47,47 0 1,1 -94,0"
                                />
                            </defs>
                            <text>
                                <textPath href="#hero-badge-path" textLength="292" lengthAdjust="spacing">
                                    { badge }
                                </textPath>
                            </text>
                        </svg>
                        <span class="hero-badge-arrow">{ "↓" }</span>
                    </div>
                </div>
            </div>
            <div class="container hero-intro">
                <p class="tagline" data-reveal="mask">{ raw(t.hero_tagline) }</p>
                <p class="lead" data-reveal="blur">{ raw(t.hero_lead) }</p>
                <div class="cta" data-reveal="">
                    <a class="btn primary" data-magnetic="" href={Route::Download.href()}>
                        { rolling(t.hero_cta) }
                        <span class="btn-arrow" aria-hidden="true">{ "→" }</span>
                    </a>
                    <span class="cta-sub" data-scramble="">{ t.hero_cta_sub }</span>
                </div>
            </div>
        </header>
    }
}
