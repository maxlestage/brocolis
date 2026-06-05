import { useEffect, useState } from "react";
import { useT } from "../i18n/LangContext";
import { Switchers } from "./Switchers";

const NAV_ITEMS: ReadonlyArray<{
  href: string;
  key:
    | "nav.platforms"
    | "nav.features"
    | "nav.quickstart"
    | "nav.cli"
    | "nav.privacy";
}> = [
  { href: "#platforms", key: "nav.platforms" },
  { href: "#features", key: "nav.features" },
  { href: "#quickstart", key: "nav.quickstart" },
  { href: "#cli", key: "nav.cli" },
  { href: "#privacy", key: "nav.privacy" },
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

  return (
    <nav className="navbar">
      <div className="container nav-inner">
        <a className="nav-brand" href="#top" onClick={close}>
          <span className="nav-leaf" aria-hidden="true">🥦</span>
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
              <li key={item.href}>
                <a href={item.href} onClick={close}>
                  {t(item.key)}
                </a>
              </li>
            ))}
          </ul>
          <div className="nav-switchers">
            <Switchers />
          </div>
        </div>
      </div>
    </nav>
  );
}
