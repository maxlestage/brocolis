use active::Node;
use active::html::*;

use crate::components::app_mock::section_app;
use crate::components::demo::demo;
use crate::components::footer::site_footer;
use crate::components::hero::hero;
use crate::components::manifesto::manifesto;
use crate::components::marquee::marquee;
use crate::components::navbar::navbar;
use crate::components::sections::{
    section_features, section_how_it_works, section_platforms, section_privacy, section_quickstart,
    section_ratchet, section_switch,
};
use crate::components::skip_link::skip_link;

/// Page d'accueil : l'assemblage des sections.
pub fn landing() -> Node {
    Node::from([
        skip_link(),
        navbar(),
        hero(),
        marquee(false, false, None),
        // `tabindex="-1"` : non atteignable par tabulation, mais
        // focalisable par programme — c'est ce qui permet au lien d'évitement
        // de déplacer réellement le focus clavier ici.
        main()
            .id("contenu")
            .class("container")
            .attr("tabindex", "-1")
            .child(manifesto())
            .child(demo())
            .child(section_platforms())
            .child(section_switch())
            .child(section_features())
            .child(section_quickstart())
            .child(section_app())
            .child(section_how_it_works())
            .child(section_ratchet())
            .child(section_privacy())
            .into(),
        site_footer(true),
        spy(),
    ])
}

/// Repère de section, en bas à gauche sur grand écran : le numéro de la
/// section courante défile comme un compteur mécanique (`--spy`, écrit par
/// `motion`). Décoratif — la barre de navigation porte la même information
/// (`data-current`) de façon accessible.
fn spy() -> Node {
    let digits = (1..=8).map(|n| span().text(format!("{n:02}")));
    div()
        .class("spy")
        .attr("aria-hidden", "true")
        .child(
            span()
                .class("spy-window")
                .child(span().class("spy-strip").children(digits)),
        )
        .child(span().class("spy-total").text("/ 08"))
        .into()
}
