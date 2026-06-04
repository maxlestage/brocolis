import { useT } from "../i18n/LangContext";

const LI_KEYS = ["s3.li1", "s3.li2", "s3.li3", "s3.li4"] as const;

export function SectionQuickstart() {
  const t = useT();
  return (
    <section id="quickstart">
      <h2>{t("s3.h")}</h2>
      <ol>
        {LI_KEYS.map((k) => (
          <li key={k} dangerouslySetInnerHTML={{ __html: t(k) }} />
        ))}
      </ol>
    </section>
  );
}
