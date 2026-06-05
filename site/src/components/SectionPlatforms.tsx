import { useT } from "../i18n/LangContext";

const RELEASE_BASE = "https://github.com/maxlestage/brocolis/releases/tag";

const PLATFORMS = [
  {
    title: "platforms.macos.t",
    desc: "platforms.macos.d",
    cta: "platforms.macos.cta",
    icon: "🍎",
    href: `${RELEASE_BASE}/cli-latest`,
  },
  {
    title: "platforms.linux.t",
    desc: "platforms.linux.d",
    cta: "platforms.linux.cta",
    icon: "🐧",
    href: `${RELEASE_BASE}/cli-latest-linux`,
  },
  {
    title: "platforms.windows.t",
    desc: "platforms.windows.d",
    cta: "platforms.windows.cta",
    icon: "🪟",
    href: `${RELEASE_BASE}/cli-latest-windows`,
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
