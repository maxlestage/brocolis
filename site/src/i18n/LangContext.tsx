import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { DICT, LANGS, type I18nKey, type Lang } from "./dict";

type Ctx = {
  lang: Lang;
  setLang: (l: Lang) => void;
  t: (key: I18nKey) => string;
};

const LangCtx = createContext<Ctx | null>(null);

function detectLang(): Lang {
  try {
    const s = localStorage.getItem("brocolis.lang");
    if (s && (LANGS as readonly string[]).includes(s)) return s as Lang;
  } catch {
    // ignore
  }
  const nav = (navigator.language || "fr").slice(0, 2).toLowerCase();
  return (LANGS as readonly string[]).includes(nav) ? (nav as Lang) : "fr";
}

export function LangProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(() => detectLang());

  const setLang = (l: Lang) => {
    setLangState(l);
    try {
      localStorage.setItem("brocolis.lang", l);
    } catch {
      // ignore
    }
  };

  useEffect(() => {
    document.documentElement.lang = lang;
    const dict = DICT[lang];
    document.title = dict["meta.title"];
    const setMeta = (selector: string, value: string) => {
      const el = document.querySelector(selector);
      if (el) el.setAttribute("content", value);
    };
    setMeta('meta[name="description"]', dict["meta.desc"]);
    setMeta('meta[property="og:title"]', dict["meta.ogTitle"]);
    setMeta('meta[property="og:description"]', dict["meta.ogDesc"]);
  }, [lang]);

  const value = useMemo<Ctx>(
    () => ({ lang, setLang, t: (k) => DICT[lang][k] }),
    [lang],
  );

  return <LangCtx.Provider value={value}>{children}</LangCtx.Provider>;
}

export function useLang() {
  const v = useContext(LangCtx);
  if (!v) throw new Error("useLang must be used within LangProvider");
  return v;
}

export function useT() {
  return useLang().t;
}
