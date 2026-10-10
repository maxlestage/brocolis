use yew::prelude::*;

use crate::components::app_mock::SectionApp;
use crate::components::demo::Demo;
use crate::components::footer::Footer;
use crate::components::hero::Hero;
use crate::components::manifesto::Manifesto;
use crate::components::marquee::Marquee;
use crate::components::navbar::Navbar;
use crate::components::sections::{
    SectionFeatures, SectionHowItWorks, SectionPlatforms, SectionPrivacy, SectionQuickstart,
    SectionSwitch,
};
use crate::components::skip_link::SkipLink;

/// Page d'accueil : l'assemblage des sections.
#[function_component]
pub fn Landing() -> Html {
    html! {
        <>
            <SkipLink />
            <Navbar />
            <Hero />
            <Marquee />
            // `tabindex="-1"` : non atteignable par tabulation, mais
            // focalisable par programme — c'est ce qui permet au lien d'évitement
            // de déplacer réellement le focus clavier ici.
            <main id="contenu" class="container" tabindex="-1">
                <Manifesto />
                <Demo />
                <SectionPlatforms />
                <SectionSwitch />
                <SectionFeatures />
                <SectionQuickstart />
                <SectionApp />
                <SectionHowItWorks />
                <SectionPrivacy />
            </main>
            <Footer />
            <Spy />
        </>
    }
}

/// Repère de section, en bas à gauche sur grand écran : le numéro de la
/// section courante défile comme un compteur mécanique (`--spy`, écrit par
/// `motion`). Décoratif — la barre de navigation porte la même information
/// (`data-current`) de façon accessible.
#[function_component]
fn Spy() -> Html {
    let digits = (1..=7).map(|n| html! { <span>{ format!("{n:02}") }</span> });
    html! {
        <div class="spy" aria-hidden="true">
            <span class="spy-window"><span class="spy-strip">{ for digits }</span></span>
            <span class="spy-total">{ "/ 07" }</span>
        </div>
    }
}
