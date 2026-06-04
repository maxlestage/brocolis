import { LangProvider } from "./i18n/LangContext";
import { ThemeProvider } from "./theme/ThemeContext";
import { Navbar } from "./components/Navbar";
import { Hero } from "./components/Hero";
import { SectionAuto } from "./components/SectionAuto";
import { SectionFeatures } from "./components/SectionFeatures";
import { SectionQuickstart } from "./components/SectionQuickstart";
import { SectionCli } from "./components/SectionCli";
import { SectionHowItWorks } from "./components/SectionHowItWorks";
import { SectionPrivacy } from "./components/SectionPrivacy";
import { Footer } from "./components/Footer";

export default function App() {
  return (
    <ThemeProvider>
      <LangProvider>
        <Navbar />
        <Hero />
        <main className="container">
          <SectionAuto />
          <SectionFeatures />
          <SectionQuickstart />
          <SectionCli />
          <SectionHowItWorks />
          <SectionPrivacy />
        </main>
        <Footer />
      </LangProvider>
    </ThemeProvider>
  );
}
