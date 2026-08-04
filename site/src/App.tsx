import { HashRouter, Routes, Route } from "react-router";
import { LangProvider } from "./i18n/LangContext";
import { ThemeProvider } from "./theme/ThemeContext";
import { Landing } from "./components/Landing";
import { DownloadPage } from "./components/DownloadPage";

// SPA React : la navigation (landing ↔ téléchargement) est gérée côté client
// par react-router. HashRouter est choisi pour fonctionner à l'identique sur
// GitHub Pages (sous /brocolis/) et sur Heroku (à la racine), sans config
// serveur ni fichier 404 de repli.
export default function App() {
  return (
    <ThemeProvider>
      <LangProvider>
        <HashRouter>
          <Routes>
            <Route path="/" element={<Landing />} />
            <Route path="/telecharger" element={<DownloadPage />} />
            <Route path="*" element={<Landing />} />
          </Routes>
        </HashRouter>
      </LangProvider>
    </ThemeProvider>
  );
}
