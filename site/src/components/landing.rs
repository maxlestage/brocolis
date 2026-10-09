use yew::prelude::*;

use crate::components::footer::Footer;
use crate::components::hero::Hero;
use crate::components::manifesto::Manifesto;
use crate::components::marquee::Marquee;
use crate::components::navbar::Navbar;
use crate::components::sections::{
    SectionAuto, SectionCli, SectionFeatures, SectionHowItWorks, SectionPlatforms, SectionPrivacy,
    SectionQuickstart,
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
                <SectionPlatforms />
                <SectionAuto />
                <SectionFeatures />
                <SectionQuickstart />
                <SectionCli />
                <SectionHowItWorks />
                <SectionPrivacy />
            </main>
            <Footer />
        </>
    }
}
