import { useT } from "../i18n/LangContext";

const CARDS = [
  { t: "s2.c1.t", d: "s2.c1.d" },
  { t: "s2.c2.t", d: "s2.c2.d" },
  { t: "s2.c3.t", d: "s2.c3.d" },
  { t: "s2.c4.t", d: "s2.c4.d" },
] as const;

export function SectionFeatures() {
  const t = useT();
  return (
    <section id="features">
      <h2>{t("s2.h")}</h2>
      <div className="grid">
        {CARDS.map((c) => (
          <article className="card" key={c.t}>
            <h3>{t(c.t)}</h3>
            <p dangerouslySetInnerHTML={{ __html: t(c.d) }} />
          </article>
        ))}
      </div>
    </section>
  );
}
