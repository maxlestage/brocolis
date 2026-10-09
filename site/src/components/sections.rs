//! Les sept sections de l'accueil.
//!
//! Chaque `<section>` est nommée par son propre titre via `aria-labelledby` :
//! sans ça une section n'est pas exposée comme région et disparaît de la liste
//! des repères que proposent les lecteurs d'écran.
//!
//! Attributs lus par `motion` et la feuille de style :
//! - `data-reveal` : apparaît en entrant dans la vue (`="mask"` : se dévoile
//!   de bas en haut, pour les titres) ;
//! - `data-spot` : projecteur qui suit le pointeur sur les cartes ;
//! - `data-magnetic` : bouton attiré par le pointeur.

use yew::prelude::*;

use crate::markup::raw;
use crate::route::Route;
use crate::settings::use_t;

/// En-tête de section : numéro, puis titre. Le numéro est décoratif — l'ordre
/// des sections est déjà celui du document.
fn head(index: u8, id: &'static str, title: Html) -> Html {
    html! {
        <div class="sec-head">
            <span class="sec-index" aria-hidden="true">{ format!("{index:02}") }</span>
            <h2 {id} data-reveal="mask">{ title }</h2>
        </div>
    }
}

#[function_component]
pub fn SectionPlatforms() -> Html {
    let t = use_t();

    let cards = t.platform_cards().into_iter().map(|p| {
        html! {
            <article class="card platform-card" data-reveal="" data-spot="">
                <div class="platform-head">
                    <span class="platform-icon" aria-hidden="true">{ p.icon }</span>
                    <h3>{ p.title }</h3>
                </div>
                <p>{ raw(p.desc) }</p>
                <a class="btn primary platform-cta" data-magnetic="" href={Route::Download.href()}>
                    <span>{ p.cta }</span>
                    <span class="btn-arrow" aria-hidden="true">{ "→" }</span>
                </a>
            </article>
        }
    });

    html! {
        <section id="platforms" aria-labelledby="platforms-h">
            { head(1, "platforms-h", html! { t.platforms_h }) }
            <p class="sec-intro" data-reveal="">{ raw(t.platforms_intro) }</p>
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
        .map(|&li| html! { <li data-reveal="">{ raw(li) }</li> });

    html! {
        <section id="auto" aria-labelledby="auto-h">
            { head(2, "auto-h", html! { t.s1_h }) }
            <pre class="terminal" data-reveal=""><code>{ "sudo brocolis auto" }</code></pre>
            <ul class="checklist">{ for items }</ul>
            <p class="note" data-reveal="">{ raw(t.s1_note) }</p>
        </section>
    }
}

#[function_component]
pub fn SectionFeatures() -> Html {
    let t = use_t();
    let cards = t.s2_cards.iter().enumerate().map(|(i, c)| {
        html! {
            <article class="card feature-card" data-reveal="" data-spot="">
                <span class="card-index" aria-hidden="true">{ format!("{:02}", i + 1) }</span>
                <h3>{ c.title }</h3>
                <p>{ raw(c.body) }</p>
            </article>
        }
    });

    html! {
        <section id="features" aria-labelledby="features-h">
            { head(3, "features-h", html! { t.s2_h }) }
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
        .map(|&li| html! { <li data-reveal="">{ raw(li) }</li> });

    html! {
        <section id="quickstart" aria-labelledby="quickstart-h">
            { head(4, "quickstart-h", html! { t.s3_h }) }
            <ol class="steps">{ for items }</ol>
        </section>
    }
}

#[function_component]
pub fn SectionCli() -> Html {
    let t = use_t();

    html! {
        <section id="cli" aria-labelledby="cli-h">
            { head(5, "cli-h", raw(t.s4_h)) }
            <p data-reveal="">{ t.s4_intro }</p>
            <pre class="terminal" tabindex="0" data-reveal=""><code>{ t.s4_code }</code></pre>
            <p data-reveal="">{ raw(t.s4_outro) }</p>
        </section>
    }
}

#[function_component]
pub fn SectionHowItWorks() -> Html {
    let t = use_t();

    html! {
        <section id="how" aria-labelledby="how-h">
            { head(6, "how-h", html! { t.s5_h }) }
            <p data-reveal="">{ t.s5_intro }</p>
            // Un schéma en art ASCII déborde forcément sur téléphone : il est
            // donc défilable, et `tabindex` le rend atteignable au clavier —
            // sans quoi ce qui dépasse serait hors d'atteinte sans souris.
            <pre class="terminal diagram" tabindex="0" role="img" aria-label={t.s5_h} data-reveal="">
                <code>{ t.s5_diagram }</code>
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
        .map(|&li| html! { <li data-reveal="">{ raw(li) }</li> });

    html! {
        <section id="privacy" aria-labelledby="privacy-h">
            { head(7, "privacy-h", html! { t.s6_h }) }
            <ul class="checklist">{ for items }</ul>
        </section>
    }
}
