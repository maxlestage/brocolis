use active::html::*;
use active::{Node, el};

use crate::markup::{Translated, letters, rolling};
use crate::route::Route;
use crate::settings::t;

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
pub fn hero() -> Node {
    let scroll = || t().hero_scroll.to_uppercase();

    let stage =
        div()
            .class("hero-stage")
            .attr_dyn("data-cursor", scroll)
            .child(span().class("hero-glow").attr("aria-hidden", "true"))
            .child(canvas().class("hero-dust").attr("aria-hidden", "true"))
            .child(
                p().class("hero-eyebrow")
                    .child(span().class("pulse").attr("aria-hidden", "true"))
                    .child(span().attr("data-scramble", "").tr(|t| t.hero_eyebrow)),
            )
            .child(
                div()
                    .class("hero-art")
                    // Image la plus visible de la page : chargée en priorité et
                    // dimensionnée en dur, pour qu'elle ne décale rien en arrivant.
                    .child(
                        img()
                            .attr("src", "./brocolisMan.png")
                            .attr_tr("alt", |t| t.hero_alt)
                            .attr("width", "440")
                            .attr("height", "440")
                            .attr("decoding", "async")
                            .attr("fetchpriority", "high"),
                    )
                    .child(span().class("hero-sweep").attr("aria-hidden", "true"))
                    .child(span().class("hero-orbit").attr("aria-hidden", "true")),
            )
            // Une lettre par `<span>` : chacune monte de derrière sa ligne avec un
            // léger décalage sur la précédente. Le titre reste un seul mot pour
            // les lecteurs d'écran (contenu textuel intact, aucune espace ajoutée).
            .child(h1().class("hero-title").children(letters("Brocolis")))
            .child(
                div()
                    .class("hero-badge")
                    .attr("aria-hidden", "true")
                    .child(
                        svg()
                            .attr("viewBox", "0 0 120 120")
                            .child(defs().child(
                                path().id("hero-badge-path").attr(
                                    "d",
                                    "M60,60 m-47,0 a47,47 0 1,1 94,0 a47,47 0 1,1 -94,0",
                                ),
                            ))
                            .child(
                                text_svg().child(
                                    el("textPath")
                                        .href("#hero-badge-path")
                                        .attr("textLength", "292")
                                        .attr("lengthAdjust", "spacing")
                                        .text_dyn(move || {
                                            let s = scroll();
                                            format!("{s} · {s} · {s} · ")
                                        }),
                                ),
                            ),
                    )
                    .child(span().class("hero-badge-arrow").text("↓")),
            );

    header()
        .id("top")
        .class("hero")
        .child(div().class("hero-pin").child(stage))
        .child(
            div()
                .class("container hero-intro")
                .child(
                    p().class("tagline")
                        .attr("data-reveal", "mask")
                        .raw(|t| t.hero_tagline),
                )
                .child(
                    p().class("lead")
                        .attr("data-reveal", "blur")
                        .raw(|t| t.hero_lead),
                )
                .child(
                    div()
                        .class("cta")
                        .attr("data-reveal", "")
                        .child(
                            a().class("btn primary")
                                .attr("data-magnetic", "")
                                .href(Route::Download.href())
                                .child(rolling(|t| t.hero_cta))
                                .child(
                                    span()
                                        .class("btn-arrow")
                                        .attr("aria-hidden", "true")
                                        .text("→"),
                                ),
                        )
                        .child(
                            span()
                                .class("cta-sub")
                                .attr("data-scramble", "")
                                .tr(|t| t.hero_cta_sub),
                        ),
                ),
        )
        .into()
}
