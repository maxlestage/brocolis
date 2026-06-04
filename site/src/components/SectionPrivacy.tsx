import { useT } from "../i18n/LangContext";

const LI_KEYS = ["s6.li1", "s6.li2", "s6.li3", "s6.li4"] as const;

export function SectionPrivacy() {
  const t = useT();
  return (
    <section id="privacy">
      <h2>{t("s6.h")}</h2>
      <ul className="checklist">
        {LI_KEYS.map((k) => (
          <li key={k} dangerouslySetInnerHTML={{ __html: t(k) }} />
        ))}
      </ul>
    </section>
  );
}
