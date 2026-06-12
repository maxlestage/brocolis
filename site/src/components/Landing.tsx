import { Navbar } from "./Navbar";
import { Hero } from "./Hero";
import { SectionPlatforms } from "./SectionPlatforms";
import { SectionAuto } from "./SectionAuto";
import { SectionFeatures } from "./SectionFeatures";
import { SectionQuickstart } from "./SectionQuickstart";
import { SectionCli } from "./SectionCli";
import { SectionHowItWorks } from "./SectionHowItWorks";
import { SectionPrivacy } from "./SectionPrivacy";
import { Footer } from "./Footer";

// Page d'accueil : assemblée des sections. Rendue sur la route "/".
export function Landing() {
  return (
    <>
      <Navbar />
      <Hero />
      <main className="container">
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
  );
}
