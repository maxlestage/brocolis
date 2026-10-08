use yew::prelude::*;

use crate::markup::raw;
use crate::route::Route;
use crate::settings::use_t;

#[function_component]
pub fn Hero() -> Html {
    let t = use_t();

    html! {
        <header id="top" class="hero">
            <div class="container hero-stack">
                <div class="hero-art">
                    // Image la plus visible de la page : chargée en priorité et
                    // dimensionnée en dur, pour qu'elle ne décale rien en
                    // arrivant.
                    <img
                        src="./brocolisMan.png"
                        alt={t.hero_alt}
                        width="220"
                        height="220"
                        decoding="async"
                        fetchpriority="high"
                    />
                </div>
                <div class="hero-text">
                    <h1>
                        { "Brocolis " }
                        <span class="emoji" aria-hidden="true">{ "🥦" }</span>
                    </h1>
                    <p class="tagline">{ raw(t.hero_tagline) }</p>
                    <p class="lead">{ raw(t.hero_lead) }</p>
                    <div class="cta">
                        <div class="cta-stack">
                            <a class="btn primary" href={Route::Download.href()}>
                                { t.hero_cta }
                            </a>
                            <span class="cta-sub">{ t.hero_cta_sub }</span>
                        </div>
                    </div>
                </div>
            </div>
        </header>
    }
}
