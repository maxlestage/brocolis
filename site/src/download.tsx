import React from "react";
import ReactDOM from "react-dom/client";
import { LangProvider } from "./i18n/LangContext";
import { ThemeProvider } from "./theme/ThemeContext";
import { DownloadPage } from "./components/DownloadPage";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider>
      <LangProvider>
        <DownloadPage />
      </LangProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
