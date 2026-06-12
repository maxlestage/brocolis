import { useT } from "../i18n/LangContext";
import { Switchers } from "./Switchers";

const REL = "https://github.com/maxlestage/brocolis/releases";

// Liens de téléchargement direct vers les builds « rolling » (dernier master).
// Les noms d'artefacts sont stables (cf. workflows *-cli-release.yml).
const DL = {
  macos: `${REL}/download/cli-latest/brocolis-macos-universal.tar.gz`,
  linuxDeb: `${REL}/download/cli-latest-linux/brocolis-linux-amd64.deb`,
  linuxRpm: `${REL}/download/cli-latest-linux/brocolis-linux-x86_64.rpm`,
  windows: `${REL}/download/cli-latest-windows/brocolis-windows-x86_64.zip`,
} as const;

export function DownloadPage() {
  const t = useT();
  return (
    <>
      <nav className="navbar">
        <div className="container nav-inner">
          <a className="nav-brand" href="./">
            <span className="nav-leaf" aria-hidden="true">
              🥦
            </span>
            <span>Brocolis</span>
          </a>
          <div className="nav-switchers">
            <Switchers />
          </div>
        </div>
      </nav>

      <main className="container dl-main">
        <a className="dl-back" href="./">
          ← {t("dl.back")}
        </a>
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
