import { useT } from "../i18n/LangContext";

export function SectionCli() {
  const t = useT();
  return (
    <section id="cli">
      <h2 dangerouslySetInnerHTML={{ __html: t("s4.h") }} />
      <p>{t("s4.intro")}</p>
      <pre>
        <code>{t("s4.code")}</code>
      </pre>
      <p dangerouslySetInnerHTML={{ __html: t("s4.outro") }} />
    </section>
  );
}
