use yew::prelude::*;

use crate::components::footer::Footer;
use crate::components::marquee::Marquee;
use crate::components::skip_link::SkipLink;
use crate::components::switchers::Switchers;
use crate::markup::{raw, rolling, words};
use crate::route::Route;
use crate::settings::use_t;

/// Page des versions GitHub, pour qui veut une version précise.
const RELEASES: &str = "https://github.com/maxlestage/brocolis/releases";

// Les binaires sont relayés par le serveur (routes `/dl/*` de server.js) : les
// assets de release vivent dans un dépôt privé, donc un lien GitHub direct
// renverrait 404 à tout visiteur. Le serveur, lui, est authentifié.
const DL_MACOS: &str = "/dl/macos";
const DL_LINUX_DEB: &str = "/dl/linux-deb";
const DL_LINUX_RPM: &str = "/dl/linux-rpm";
const DL_WINDOWS: &str = "/dl/windows";

#[function_component]
pub fn DownloadPage() -> Html {
    let t = use_t();

    html! {
        <>
            <SkipLink />
            // Pas de hamburger ici : juste la marque et les sélecteurs.
            <nav class="navbar dl-navbar" aria-label={t.nav_menu}>
                <div class="container nav-inner">
                    <a class="nav-brand" href={Route::Home.href()}>
                        <span class="nav-leaf" aria-hidden="true">{ "🥦" }</span>
                        <span data-scramble="">{ "Brocolis" }</span>
                    </a>
                    <div class="nav-switchers">
                        <Switchers />
                    </div>
                </div>
            </nav>

            <main id="contenu" class="container dl-main" tabindex="-1">
                <a class="dl-back" href={Route::Home.href()}>
                    <span aria-hidden="true">{ "← " }</span><span data-scramble="">{ t.dl_back }</span>
                </a>
                <h1 id="dl-h" class="dl-title" data-reveal="words">{ words(t.dl_h) }</h1>
                <p class="dl-intro" data-reveal="blur">{ raw(t.dl_intro) }</p>
                <p class="dl-version" data-reveal="blur">{ raw(t.dl_version) }</p>

                <section id="downloads" class="grid grid-3 dl-grid" aria-labelledby="dl-h">
                    <article class="card platform-card" data-reveal="card" data-tilt="" data-spot="">
                        <div class="platform-head">
                            <span class="platform-icon" aria-hidden="true">{ "🍎" }</span>
                            <h2>{ t.dl_macos_t }</h2>
                        </div>
                        <p>{ raw(t.dl_macos_d) }</p>
                        <a class="btn primary platform-cta" data-magnetic="" href={DL_MACOS}>
                            { rolling(t.dl_macos_btn) }
                        </a>
                        <p class="dl-note">{ raw(t.dl_macos_note) }</p>
                    </article>

                    <article class="card platform-card" data-reveal="card" data-tilt="" data-spot="">
                        <div class="platform-head">
                            <span class="platform-icon" aria-hidden="true">{ "🐧" }</span>
                            <h2>{ t.dl_linux_t }</h2>
                        </div>
                        <p>{ raw(t.dl_linux_d) }</p>
                        <div class="dl-btn-row">
                            <a class="btn primary platform-cta" data-magnetic="" href={DL_LINUX_DEB}>
                                { rolling(t.dl_linux_deb) }
                            </a>
                            <a class="btn platform-cta" data-magnetic="" href={DL_LINUX_RPM}>
                                { rolling(t.dl_linux_rpm) }
                            </a>
                        </div>
                        <p class="dl-note">{ raw(t.dl_linux_note) }</p>
                    </article>

                    <article class="card platform-card" data-reveal="card" data-tilt="" data-spot="">
                        <div class="platform-head">
                            <span class="platform-icon" aria-hidden="true">{ "🪟" }</span>
                            <h2>{ t.dl_windows_t }</h2>
                        </div>
                        <p>{ raw(t.dl_windows_d) }</p>
                        <a class="btn primary platform-cta" data-magnetic="" href={DL_WINDOWS}>
                            { rolling(t.dl_windows_btn) }
                        </a>
                        <p class="dl-note">{ raw(t.dl_windows_note) }</p>
                    </article>
                </section>

                <p class="dl-all" data-reveal="">
                    <a href={RELEASES}>{ t.dl_all }</a>
                </p>
            </main>

            <Marquee words={vec!["macOS", "Linux", "Windows", "Intel", "Apple Silicon", ".deb", ".rpm", "x86_64"]} />
            <Footer cta={false} />
        </>
    }
}
