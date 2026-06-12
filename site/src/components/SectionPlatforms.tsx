import { Link } from "react-router-dom";
import { useT } from "../i18n/LangContext";

// Les CTA mènent à la page dédiée de téléchargement (route React).
const PLATFORMS = [
  {
    title: "platforms.macos.t",
    desc: "platforms.macos.d",
    cta: "platforms.macos.cta",
    icon: "🍎",
  },
  {
    title: "platforms.linux.t",
    desc: "platforms.linux.d",
    cta: "platforms.linux.cta",
    icon: "🐧",
  },
  {
    title: "platforms.windows.t",
    desc: "platforms.windows.d",
    cta: "platforms.windows.cta",
    icon: "🪟",
  },
] as const;

export function SectionPlatforms() {
  const t = useT();
  return (
    <section id="platforms">
      <h2>{t("platforms.h")}</h2>
      <p dangerouslySetInnerHTML={{ __html: t("platforms.intro") }} />
      <div className="grid grid-3">
        {PLATFORMS.map((p) => (
          <article className="card platform-card" key={p.title}>
            <div className="platform-head">
              <span className="platform-icon" aria-hidden="true">
                {p.icon}
              </span>
              <h3>{t(p.title)}</h3>
            </div>
            <p dangerouslySetInnerHTML={{ __html: t(p.desc) }} />
            <Link className="btn primary platform-cta" to="/telecharger">
              {t(p.cta)}
            </Link>
          </article>
        ))}
      </div>
    </section>
  );
}
