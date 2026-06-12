import { useT } from "../i18n/LangContext";

// Les CTA mènent à la page dédiée de téléchargement (liens directs par OS).
const DOWNLOAD_PAGE = "./telecharger.html";

const PLATFORMS = [
  {
    title: "platforms.macos.t",
    desc: "platforms.macos.d",
    cta: "platforms.macos.cta",
    icon: "🍎",
    href: DOWNLOAD_PAGE,
  },
  {
    title: "platforms.linux.t",
    desc: "platforms.linux.d",
    cta: "platforms.linux.cta",
    icon: "🐧",
    href: DOWNLOAD_PAGE,
  },
  {
    title: "platforms.windows.t",
    desc: "platforms.windows.d",
    cta: "platforms.windows.cta",
    icon: "🪟",
    href: DOWNLOAD_PAGE,
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
            <a className="btn primary platform-cta" href={p.href}>
              {t(p.cta)}
            </a>
          </article>
        ))}
      </div>
    </section>
  );
}
