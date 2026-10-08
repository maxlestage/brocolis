//! Les six sections de l'accueil.
//!
//! Chaque `<section>` est nommée par son propre titre via `aria-labelledby` :
//! sans ça une section n'est pas exposée comme région et disparaît de la liste
//! des repères que proposent les lecteurs d'écran.

use yew::prelude::*;

use crate::markup::raw;
use crate::route::Route;
use crate::settings::use_t;

#[function_component]
pub fn SectionPlatforms() -> Html {
    let t = use_t();

    let cards = t.platform_cards().into_iter().map(|p| {
        html! {
            <article class="card platform-card">
                <div class="platform-head">
                    <span class="platform-icon" aria-hidden="true">{ p.icon }</span>
                    <h3>{ p.title }</h3>
                </div>
                <p>{ raw(p.desc) }</p>
                <a class="btn primary platform-cta" href={Route::Download.href()}>
                    { p.cta }
                </a>
            </article>
        }
    });

    html! {
        <section id="platforms" aria-labelledby="platforms-h">
            <h2 id="platforms-h">{ t.platforms_h }</h2>
            <p>{ raw(t.platforms_intro) }</p>
            <div class="grid grid-3">{ for cards }</div>
        </section>
    }
}

#[function_component]
pub fn SectionAuto() -> Html {
    let t = use_t();
    let items = t.s1_li.iter().map(|&li| html! { <li>{ raw(li) }</li> });

    html! {
        <section id="auto" aria-labelledby="auto-h">
            <h2 id="auto-h">{ t.s1_h }</h2>
            <pre><code>{ "sudo brocolis auto" }</code></pre>
            <ul class="checklist">{ for items }</ul>
            <p class="note">{ raw(t.s1_note) }</p>
        </section>
    }
}

#[function_component]
pub fn SectionFeatures() -> Html {
    let t = use_t();
    let cards = t.s2_cards.iter().map(|c| {
        html! {
            <article class="card">
                <h3>{ c.title }</h3>
                <p>{ raw(c.body) }</p>
            </article>
        }
    });

    html! {
        <section id="features" aria-labelledby="features-h">
            <h2 id="features-h">{ t.s2_h }</h2>
            <div class="grid">{ for cards }</div>
        </section>
    }
}

#[function_component]
pub fn SectionQuickstart() -> Html {
    let t = use_t();
    let items = t.s3_li.iter().map(|&li| html! { <li>{ raw(li) }</li> });

    html! {
        <section id="quickstart" aria-labelledby="quickstart-h">
            <h2 id="quickstart-h">{ t.s3_h }</h2>
            <ol>{ for items }</ol>
        </section>
    }
}

#[function_component]
pub fn SectionCli() -> Html {
    let t = use_t();

    html! {
        <section id="cli" aria-labelledby="cli-h">
            <h2 id="cli-h">{ raw(t.s4_h) }</h2>
            <p>{ t.s4_intro }</p>
            <pre tabindex="0"><code>{ t.s4_code }</code></pre>
            <p>{ raw(t.s4_outro) }</p>
        </section>
    }
}

#[function_component]
pub fn SectionHowItWorks() -> Html {
    let t = use_t();

    html! {
        <section id="how" aria-labelledby="how-h">
            <h2 id="how-h">{ t.s5_h }</h2>
            <p>{ t.s5_intro }</p>
            // Un schéma en art ASCII déborde forcément sur téléphone : il est
            // donc défilable, et `tabindex` le rend atteignable au clavier —
            // sans quoi ce qui dépasse serait hors d'atteinte sans souris.
            <pre class="diagram" tabindex="0" role="img" aria-label={t.s5_h}>
                <code>{ t.s5_diagram }</code>
            </pre>
        </section>
    }
}

#[function_component]
pub fn SectionPrivacy() -> Html {
    let t = use_t();
    let items = t.s6_li.iter().map(|&li| html! { <li>{ raw(li) }</li> });

    html! {
        <section id="privacy" aria-labelledby="privacy-h">
            <h2 id="privacy-h">{ t.s6_h }</h2>
            <ul class="checklist">{ for items }</ul>
        </section>
    }
}
