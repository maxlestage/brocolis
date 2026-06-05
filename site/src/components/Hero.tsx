import { useT } from "../i18n/LangContext";

export function Hero() {
  const t = useT();
  return (
    <header id="top" className="hero">
      <div className="container hero-stack">
        <div className="hero-art">
          <img
            src="./brocolisMan.png"
            alt={t("hero.alt")}
            width={220}
            height={220}
          />
        </div>
        <div className="hero-text">
          <h1>
            Brocolis{" "}
            <span className="emoji" aria-hidden="true">
              🥦
            </span>
          </h1>
          <p
            className="tagline"
            dangerouslySetInnerHTML={{ __html: t("hero.tagline") }}
          />
          <p
            className="lead"
            dangerouslySetInnerHTML={{ __html: t("hero.lead") }}
          />
          <div className="cta">
            <div className="cta-stack">
              <a
                className="btn primary"
                href="https://github.com/maxlestage/brocolis/releases"
              >
                {t("hero.cta")}
              </a>
              <span className="cta-sub">{t("hero.cta.sub")}</span>
            </div>
          </div>
        </div>
      </div>
    </header>
  );
}
