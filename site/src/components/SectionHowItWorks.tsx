import { useT } from "../i18n/LangContext";

export function SectionHowItWorks() {
  const t = useT();
  return (
    <section id="how">
      <h2>{t("s5.h")}</h2>
      <p>{t("s5.intro")}</p>
      <pre className="diagram">
        <code>{t("s5.diagram")}</code>
      </pre>
    </section>
  );
}
