import { useEffect, useState, type MouseEvent } from "react";
import { Link } from "react-router-dom";
import { useT } from "../i18n/LangContext";
import { Switchers } from "./Switchers";

const NAV_ITEMS: ReadonlyArray<{
  id: string;
  key:
    | "nav.platforms"
    | "nav.features"
    | "nav.quickstart"
    | "nav.cli"
    | "nav.privacy";
}> = [
  { id: "platforms", key: "nav.platforms" },
  { id: "features", key: "nav.features" },
  { id: "quickstart", key: "nav.quickstart" },
  { id: "cli", key: "nav.cli" },
  { id: "privacy", key: "nav.privacy" },
];

export function Navbar() {
  const t = useT();
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  const close = () => setOpen(false);

  // Défilement doux vers une section. HashRouter occupe déjà le hash de
  // l'URL, donc on gère le scroll en JS plutôt que par ancres #id.
  const scrollTo = (e: MouseEvent, id: string) => {
    e.preventDefault();
    close();
    if (id === "top") {
      window.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth" });
  };

  return (
    <nav className="navbar">
      <div className="container nav-inner">
        <a
          className="nav-brand"
          href="#top"
          onClick={(e) => scrollTo(e, "top")}
        >
          <span className="nav-leaf" aria-hidden="true">
            🥦
          </span>
          <span>Brocolis</span>
        </a>
        <button
          type="button"
          className={`nav-toggle ${open ? "open" : ""}`}
          aria-label={t("nav.menu")}
          aria-expanded={open}
          onClick={() => setOpen((v) => !v)}
        >
          <span />
          <span />
          <span />
        </button>
        <div className={`nav-drawer ${open ? "open" : ""}`}>
          <ul className="nav-links">
            {NAV_ITEMS.map((item) => (
              <li key={item.id}>
                <a href={`#${item.id}`} onClick={(e) => scrollTo(e, item.id)}>
                  {t(item.key)}
                </a>
              </li>
            ))}
            <li>
              <Link className="nav-download" to="/telecharger" onClick={close}>
                {t("nav.download")}
              </Link>
            </li>
          </ul>
          <div className="nav-switchers">
            <Switchers />
          </div>
        </div>
      </div>
    </nav>
  );
}
