use active::html::*;
use active::{Element, Node};

use crate::components::footer::site_footer;
use crate::components::marquee::marquee;
use crate::components::skip_link::skip_link;
use crate::components::switchers::switchers;
use crate::markup::{Pick, Translated, rolling, words};
use crate::route::Route;

/// Page des versions GitHub, pour qui veut une version précise.
const RELEASES: &str = "https://github.com/maxlestage/brocolis/releases";

// Les binaires sont relayés par le serveur (routes `/dl/*` de server.js) : les
// assets de release vivent dans un dépôt privé, donc un lien GitHub direct
// renverrait 404 à tout visiteur. Le serveur, lui, est authentifié.
const DL_MACOS: &str = "/dl/macos";
const DL_LINUX_DEB: &str = "/dl/linux-deb";
const DL_LINUX_RPM: &str = "/dl/linux-rpm";
const DL_WINDOWS: &str = "/dl/windows";

/// Bandeau de la page : les plateformes et formats proposés.
const MARQUEE: &[&str] = &[
    "macOS",
    "Linux",
    "Windows",
    "Intel",
    "Apple Silicon",
    ".deb",
    ".rpm",
    "x86_64",
];

/// Bouton de téléchargement.
fn download(primary: bool, href: &'static str, label: impl Pick + Copy) -> Element {
    a().class("btn platform-cta")
        .class(if primary { "primary" } else { "" })
        .attr("data-magnetic", "")
        .href(href)
        .child(rolling(label))
}

/// Carte d'une plateforme : titre, description, boutons, note.
fn platform(
    icon: &'static str,
    title: impl Pick,
    desc: impl Pick,
    buttons: Element,
    note: impl Pick,
) -> Element {
    article()
        .class("card platform-card")
        .attr("data-reveal", "card")
        .attr("data-tilt", "")
        .attr("data-spot", "")
        .child(
            div()
                .class("platform-head")
                .child(
                    span()
                        .class("platform-icon")
                        .attr("aria-hidden", "true")
                        .text(icon),
                )
                .child(h2().tr(title)),
        )
        .child(p().raw(desc))
        .child(buttons)
        .child(p().class("dl-note").raw(note))
}

pub fn download_page() -> Node {
    Node::from([
        skip_link(),
        // Pas de hamburger ici : juste la marque et les sélecteurs.
        nav()
            .class("navbar dl-navbar")
            .attr_tr("aria-label", |t| t.nav_menu)
            .child(
                div()
                    .class("container nav-inner")
                    .child(
                        a().class("nav-brand")
                            .href(Route::Home.href())
                            .child(
                                span()
                                    .class("nav-leaf")
                                    .attr("aria-hidden", "true")
                                    .text("🥦"),
                            )
                            .child(span().attr("data-scramble", "").text("Brocolis")),
                    )
                    .child(div().class("nav-switchers").child(switchers())),
            )
            .into(),
        main()
            .id("contenu")
            .class("container dl-main")
            .attr("tabindex", "-1")
            .child(
                a().class("dl-back")
                    .href(Route::Home.href())
                    .child(span().attr("aria-hidden", "true").text("← "))
                    .child(span().attr("data-scramble", "").tr(|t| t.dl_back)),
            )
            .child(
                h1().id("dl-h")
                    .class("dl-title")
                    .attr("data-reveal", "words")
                    .child(words(|t| t.dl_h)),
            )
            .child(
                p().class("dl-intro")
                    .attr("data-reveal", "blur")
                    .raw(|t| t.dl_intro),
            )
            .child(
                p().class("dl-version")
                    .attr("data-reveal", "blur")
                    .raw(|t| t.dl_version),
            )
            .child(
                section()
                    .id("downloads")
                    .class("grid grid-3 dl-grid")
                    .attr("aria-labelledby", "dl-h")
                    .child(platform(
                        "🍎",
                        |t| t.dl_macos_t,
                        |t| t.dl_macos_d,
                        download(true, DL_MACOS, |t| t.dl_macos_btn),
                        |t| t.dl_macos_note,
                    ))
                    .child(platform(
                        "🐧",
                        |t| t.dl_linux_t,
                        |t| t.dl_linux_d,
                        div()
                            .class("dl-btn-row")
                            .child(download(true, DL_LINUX_DEB, |t| t.dl_linux_deb))
                            .child(download(false, DL_LINUX_RPM, |t| t.dl_linux_rpm)),
                        |t| t.dl_linux_note,
                    ))
                    .child(platform(
                        "🪟",
                        |t| t.dl_windows_t,
                        |t| t.dl_windows_d,
                        download(true, DL_WINDOWS, |t| t.dl_windows_btn),
                        |t| t.dl_windows_note,
                    )),
            )
            .child(
                p().class("dl-all")
                    .attr("data-reveal", "")
                    .child(a().href(RELEASES).tr(|t| t.dl_all)),
            )
            .into(),
        marquee(false, false, Some(MARQUEE)),
        site_footer(false),
    ])
}
