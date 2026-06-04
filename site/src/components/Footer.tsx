import { useT } from "../i18n/LangContext";

export function Footer() {
  const t = useT();
  return (
    <footer>
      <div className="container">
        <p>{t("footer")}</p>
      </div>
    </footer>
  );
}
