import { useT } from "../i18n/LangContext";

const LI_KEYS = ["s1.li1", "s1.li2", "s1.li3", "s1.li4", "s1.li5", "s1.li6"] as const;

export function SectionAuto() {
  const t = useT();
  return (
    <section id="auto">
      <h2>{t("s1.h")}</h2>
      <pre>
        <code>sudo brocolis auto</code>
      </pre>
      <ul className="checklist">
        {LI_KEYS.map((k) => (
          <li key={k} dangerouslySetInnerHTML={{ __html: t(k) }} />
        ))}
      </ul>
      <p className="note" dangerouslySetInnerHTML={{ __html: t("s1.note") }} />
    </section>
  );
}
