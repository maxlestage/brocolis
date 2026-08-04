import { useEffect } from "react";
import { Link } from "react-router";
import { useT } from "../i18n/LangContext";
import { Switchers } from "./Switchers";

const REL = "https://github.com/maxlestage/brocolis/releases";

// Les binaires sont relayés par le serveur (voir /dl/* dans server.js) : les
// assets de release vivent dans un dépôt privé, donc un lien GitHub direct
// renverrait 404 à tout visiteur. Le serveur, lui, est authentifié.
const DL = {
  macos: "/dl/macos",
  linuxDeb: "/dl/linux-deb",
  linuxRpm: "/dl/linux-rpm",
  windows: "/dl/windows",
} as const;

export function DownloadPage() {
  const t = useT();

  // En SPA, le titre et le scroll ne changent pas tout seuls au changement de
  // route : on remonte en haut et on met à jour le titre de l'onglet.
  useEffect(() => {
    window.scrollTo(0, 0);
    document.title = `${t("dl.h")} — Brocolis 🥦`;
  }, [t]);

  return (
    <>
      <nav className="navbar dl-navbar">
        <div className="container nav-inner">
          <Link className="nav-brand" to="/">
            <span className="nav-leaf" aria-hidden="true">
              🥦
            </span>
            <span>Brocolis</span>
          </Link>
          <div className="nav-switchers">
            <Switchers />
          </div>
        </div>
      </nav>

      <main className="container dl-main">
        <Link className="dl-back" to="/">
          ← {t("dl.back")}
        </Link>
        <h1 className="dl-title">{t("dl.h")}</h1>
        <p className="dl-intro" dangerouslySetInnerHTML={{ __html: t("dl.intro") }} />
        <p className="dl-version" dangerouslySetInnerHTML={{ __html: t("dl.version") }} />

        <section id="downloads" className="grid grid-3 dl-grid">
          <article className="card platform-card">
            <div className="platform-head">
              <span className="platform-icon" aria-hidden="true">
                🍎
              </span>
              <h3>{t("dl.macos.t")}</h3>
            </div>
            <p dangerouslySetInnerHTML={{ __html: t("dl.macos.d") }} />
            <a className="btn primary platform-cta" href={DL.macos}>
              {t("dl.macos.btn")}
            </a>
            <p className="dl-note" dangerouslySetInnerHTML={{ __html: t("dl.macos.note") }} />
          </article>

          <article className="card platform-card">
            <div className="platform-head">
              <span className="platform-icon" aria-hidden="true">
                🐧
              </span>
              <h3>{t("dl.linux.t")}</h3>
            </div>
            <p dangerouslySetInnerHTML={{ __html: t("dl.linux.d") }} />
            <div className="dl-btn-row">
              <a className="btn primary platform-cta" href={DL.linuxDeb}>
                {t("dl.linux.deb")}
              </a>
              <a className="btn platform-cta" href={DL.linuxRpm}>
                {t("dl.linux.rpm")}
              </a>
            </div>
            <p className="dl-note" dangerouslySetInnerHTML={{ __html: t("dl.linux.note") }} />
          </article>

          <article className="card platform-card">
            <div className="platform-head">
              <span className="platform-icon" aria-hidden="true">
                🪟
              </span>
              <h3>{t("dl.windows.t")}</h3>
            </div>
            <p dangerouslySetInnerHTML={{ __html: t("dl.windows.d") }} />
            <a className="btn primary platform-cta" href={DL.windows}>
              {t("dl.windows.btn")}
            </a>
            <p className="dl-note" dangerouslySetInnerHTML={{ __html: t("dl.windows.note") }} />
          </article>
        </section>

        <p className="dl-all">
          <a href={REL}>{t("dl.all")}</a>
        </p>
      </main>
    </>
  );
}
