//! Les sept sections de l'accueil.
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

use yew::prelude::*;

use crate::markup::{raw, rolling, words};
use crate::route::Route;
use crate::settings::use_t;

/// En-tête de section : numéro, titre, et numéro fantôme géant en arrière-plan.
/// Les numéros sont décoratifs — l'ordre des sections est déjà celui du
/// document.
fn head(index: u8, id: &'static str, title: Html, reveal: &'static str) -> Html {
    html! {
        <>
            <span class="sec-ghost" aria-hidden="true">{ format!("{index:02}") }</span>
            <div class="sec-head">
                <span class="sec-index" aria-hidden="true" data-scramble="">{ format!("{index:02}") }</span>
                <h2 {id} data-reveal={reveal}>{ title }</h2>
            </div>
        </>
    }
}

/// Coche tracée au trait quand l'élément de liste apparaît.
fn tick() -> Html {
    html! {
        <svg class="tick" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 12.5l4.5 4.5L19 7.5" />
        </svg>
    }
}

/// Bloc de terminal dont les lignes apparaissent l'une après l'autre. Les
/// sauts de ligne restent du texte entre les lignes : le contenu copié est
/// identique au texte d'origine.
fn terminal_lines(code: &str) -> Html {
    let lines: Vec<&str> = code.split('\n').collect();
    let last = lines.len().saturating_sub(1);
    lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            html! {
                <>
                    <span class="line" style={format!("--i: {i}")}>{ line.to_owned() }</span>
                    if i < last { { "\n" } }
                </>
            }
        })
        .collect()
}

#[function_component]
pub fn SectionPlatforms() -> Html {
    let t = use_t();

    let cards = t.platform_cards().into_iter().map(|p| {
        html! {
            <article class="card platform-card" data-reveal="card" data-tilt="" data-spot="">
                <div class="platform-head">
                    <span class="platform-icon" aria-hidden="true">{ p.icon }</span>
                    <h3>{ p.title }</h3>
                </div>
                <p>{ raw(p.desc) }</p>
                <a class="btn primary platform-cta" data-magnetic="" href={Route::Download.href()}>
                    { rolling(p.cta) }
                    <span class="btn-arrow" aria-hidden="true">{ "→" }</span>
                </a>
            </article>
        }
    });

    html! {
        <section id="platforms" aria-labelledby="platforms-h" data-progress="">
            { head(1, "platforms-h", words(t.platforms_h), "words") }
            <p class="sec-intro" data-reveal="blur">{ raw(t.platforms_intro) }</p>
            <div class="grid grid-3">{ for cards }</div>
        </section>
    }
}

#[function_component]
pub fn SectionAuto() -> Html {
    let t = use_t();
    let items = t
        .s1_li
        .iter()
        .map(|&li| html! { <li data-reveal="left">{ tick() }{ raw(li) }</li> });
    const CMD: &str = "sudo brocolis auto";

    html! {
        <section id="auto" aria-labelledby="auto-h" data-progress="">
            { head(2, "auto-h", words(t.s1_h), "words") }
            <pre class="terminal" data-reveal="">
                <code>
                    <span class="typed" style={format!("--n: {}", CMD.len())}>{ CMD }</span>
                    <span class="caret" aria-hidden="true" />
                </code>
            </pre>
            <ul class="checklist">{ for items }</ul>
            <p class="note" data-reveal="blur">{ raw(t.s1_note) }</p>
        </section>
    }
}

#[function_component]
pub fn SectionFeatures() -> Html {
    let t = use_t();
    let cards = t.s2_cards.iter().enumerate().map(|(i, c)| {
        html! {
            <article class="card feature-card" data-reveal="card" data-tilt="" data-spot="">
                <span class="card-index" aria-hidden="true" data-scramble="">{ format!("{:02}", i + 1) }</span>
                <h3>{ c.title }</h3>
                <p>{ raw(c.body) }</p>
            </article>
        }
    });

    html! {
        <section id="features" aria-labelledby="features-h" data-progress="">
            { head(3, "features-h", words(t.s2_h), "words") }
            <div class="grid">{ for cards }</div>
        </section>
    }
}

#[function_component]
pub fn SectionQuickstart() -> Html {
    let t = use_t();
    let items = t
        .s3_li
        .iter()
        .map(|&li| html! { <li data-reveal="left">{ raw(li) }</li> });

    html! {
        <section id="quickstart" aria-labelledby="quickstart-h" data-progress="">
            { head(4, "quickstart-h", words(t.s3_h), "words") }
            <ol class="steps">{ for items }</ol>
        </section>
    }
}

#[function_component]
pub fn SectionCli() -> Html {
    let t = use_t();

    html! {
        <section id="cli" aria-labelledby="cli-h" data-progress="">
            // Ce titre contient du balisage (`<code>`) : il se dévoile d'un
            // bloc plutôt que mot à mot.
            { head(5, "cli-h", raw(t.s4_h), "mask") }
            <p data-reveal="blur">{ t.s4_intro }</p>
            <pre class="terminal lines" tabindex="0" data-reveal="">
                <code>{ terminal_lines(t.s4_code) }</code>
            </pre>
            <p data-reveal="blur">{ raw(t.s4_outro) }</p>
        </section>
    }
}

#[function_component]
pub fn SectionHowItWorks() -> Html {
    let t = use_t();

    html! {
        <section id="how" aria-labelledby="how-h" data-progress="">
            { head(6, "how-h", words(t.s5_h), "words") }
            <p data-reveal="blur">{ t.s5_intro }</p>
            // Un schéma en art ASCII déborde forcément sur téléphone : il est
            // donc défilable, et `tabindex` le rend atteignable au clavier —
            // sans quoi ce qui dépasse serait hors d'atteinte sans souris.
            <pre class="terminal diagram lines" tabindex="0" role="img" aria-label={t.s5_h} data-reveal="">
                <code>{ terminal_lines(t.s5_diagram) }</code>
            </pre>
        </section>
    }
}

#[function_component]
pub fn SectionPrivacy() -> Html {
    let t = use_t();
    let items = t
        .s6_li
        .iter()
        .map(|&li| html! { <li data-reveal="left">{ tick() }{ raw(li) }</li> });

    html! {
        <section id="privacy" aria-labelledby="privacy-h" data-progress="">
            { head(7, "privacy-h", words(t.s6_h), "words") }
            <ul class="checklist">{ for items }</ul>
        </section>
    }
}
