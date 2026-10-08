// Vérification du site (application WebAssembly) dans un vrai navigateur.
//
// Ce harnais existe parce que trois corrections successives de la page de
// téléchargement ont été jugées « à l'œil » sur des captures d'écran, et que
// les trois fois le défaut était ailleurs : un `padding` en raccourci qui
// remettait la gouttière à zéro, un sélecteur battu en spécificité, un palier
// de media query déclenché trop tôt. Rien de tout ça ne se voit sur une image
// — ça se mesure. Les assertions portent donc sur des valeurs calculées par le
// navigateur, pas sur une apparence.
//
// Usage : npm test (démarre server.js tout seul)
//         node site.mjs http://hôte:port (contre un serveur déjà lancé)

import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const ROOT = path.dirname(path.dirname(path.dirname(fileURLToPath(import.meta.url))));

const problems = [];
const note = (m) => console.log(`   ${m}`);
let checks = 0;
const ok = (m) => {
  checks += 1;
  console.log(`✓  ${m}`);
};
const bad = (m) => {
  problems.push(m);
  console.log(`✗  ${m}`);
};

/** Démarre server.js sur un port libre et renvoie son URL. */
function startServer() {
  const proc = spawn("node", [path.join(ROOT, "server.js")], {
    env: { ...process.env, PORT: "0" },
    stdio: ["ignore", "pipe", "inherit"],
  });
  const url = new Promise((resolve, reject) => {
    let buf = "";
    proc.stdout.on("data", (d) => {
      buf += d;
      const m = buf.match(/port (\d+)/);
      if (m) resolve(`http://127.0.0.1:${m[1]}`);
    });
    proc.on("exit", (c) => reject(new Error(`server.js est sorti (code ${c})`)));
    setTimeout(() => reject(new Error("server.js n'a pas démarré à temps")), 10000);
  });
  return { proc, url };
}

const serveur = process.argv[2] ? null : startServer();
const BASE = process.argv[2] || (await serveur.url);

// En intégration continue, `playwright install chromium` dépose le navigateur
// là où playwright-core sait le trouver seul : on ne force un chemin que s'il
// est fourni (image de conteneur avec un Chromium préinstallé).
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH || undefined,
  args: ["--no-sandbox"],
});

async function newPage(viewport, locale = "fr-FR") {
  // Sans préférence stockée, le site suit `navigator.language` : la locale du
  // contexte doit donc être explicite, sinon le test dépend de la valeur par
  // défaut du navigateur (en-US dans Chromium headless).
  const ctx = await browser.newContext({ viewport, locale, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  const errors = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
  page.on("requestfailed", (r) => errors.push(`requestfailed: ${r.url()} ${r.failure()?.errorText}`));
  return { ctx, page, errors };
}

/**
 * Attend qu'une condition devienne vraie côté page, sans lever si elle ne
 * vient pas : l'assertion qui suit doit pouvoir rapporter la valeur réelle.
 *
 * Toutes les attentes du harnais passent par là plutôt que par un délai fixe.
 * Yew rend de façon asynchrone et plusieurs propriétés sont animées : un
 * `waitForTimeout` calibré sur cette machine devient une source d'échecs
 * intermittents sur une machine plus lente — c'est exactement ce qui est
 * arrivé à l'assertion sur le fond sombre.
 */
async function until(page, fn, arg) {
  await page.waitForFunction(fn, arg, { timeout: 5000 }).catch(() => {});
}

async function load(page, url) {
  await page.goto(url, { waitUntil: "networkidle" });
  // L'application est rendue par WebAssembly : rien n'est dans le HTML initial.
  await page.waitForSelector("nav.navbar", { timeout: 15000 });
}

// --- 1. Accueil, mobile -----------------------------------------------------
{
  const { ctx, page, errors } = await newPage({ width: 390, height: 844 });
  await load(page, BASE + "/");

  const h1 = (await page.textContent("h1")) || "";
  h1.includes("Brocolis") ? ok(`accueil : h1 « ${h1.trim()} »`) : bad(`h1 inattendu : ${h1}`);

  const sections = await page.$$eval("main section[id]", (els) => els.map((e) => e.id));
  const expected = ["platforms", "auto", "features", "quickstart", "cli", "how", "privacy"];
  JSON.stringify(sections) === JSON.stringify(expected)
    ? ok(`7 sections dans l'ordre : ${sections.join(", ")}`)
    : bad(`sections : attendu ${expected.join(",")} — obtenu ${sections.join(",")}`);

  // Le balisage des traductions doit avoir été interprété, pas affiché.
  const hasCode = await page.$eval(".lead", (el) => !!el.querySelector("code.inv"));
  hasCode ? ok("le balisage des traductions est interprété (<code class=inv>)") : bad("<code> rendu en texte");
  const leaked = (await page.textContent("body")) || "";
  leaked.includes("<strong>") || leaked.includes("<code>")
    ? bad("du balisage apparaît en texte brut")
    : ok("aucune balise visible en texte brut");

  // Débordement horizontal : le bug historique du site.
  const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  over <= 0 ? ok("aucun débordement horizontal (390px)") : bad(`débordement horizontal de ${over}px`);

  // Tiroir de navigation.
  const drawerHiddenBefore = await page.$eval("#nav-drawer", (e) => getComputedStyle(e).display);
  await page.click(".nav-toggle");
  await until(page, () => document.querySelector(".nav-toggle")?.getAttribute("aria-expanded") === "true");
  const drawerShown = await page.$eval("#nav-drawer", (e) => getComputedStyle(e).display);
  const expanded = await page.getAttribute(".nav-toggle", "aria-expanded");
  drawerHiddenBefore === "none" && drawerShown === "flex" && expanded === "true"
    ? ok("le tiroir s'ouvre au clic (aria-expanded=true)")
    : bad(`tiroir : ${drawerHiddenBefore} → ${drawerShown}, aria-expanded=${expanded}`);

  await page.keyboard.press("Escape");
  await until(page, () => getComputedStyle(document.querySelector("#nav-drawer")).display === "none");
  const afterEsc = await page.$eval("#nav-drawer", (e) => getComputedStyle(e).display);
  afterEsc === "none" ? ok("Échap referme le tiroir") : bad(`Échap : display=${afterEsc}`);

  // Un lien de section ne doit PAS changer la route.
  await page.click(".nav-toggle");
  await until(page, () => getComputedStyle(document.querySelector("#nav-drawer")).display === "flex");
  await page.click('.nav-links a[href="#features"]');
  await until(page, () => window.scrollY > 100);
  const hash = await page.evaluate(() => location.hash);
  const navbarStill = await page.$("nav.navbar");
  hash === "" && navbarStill
    ? ok("un lien de section défile sans toucher au fragment d'URL")
    : bad(`après clic sur #features : location.hash = ${JSON.stringify(hash)}`);
  const scrolled = await page.evaluate(() => window.scrollY);
  scrolled > 100 ? ok(`la page a défilé (scrollY=${Math.round(scrolled)})`) : bad(`pas de défilement (scrollY=${scrolled})`);

  // Changement de langue.
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.click(".nav-toggle");
  await until(page, () => getComputedStyle(document.querySelector("#nav-drawer")).display === "flex");
  await page.click('.switcher button:has-text("EN")');
  await until(page, () => document.documentElement.lang === "en");
  const lang = await page.getAttribute("html", "lang");
  const title = await page.title();
  lang === "en" && title.includes("Rust MITM proxy")
    ? ok(`langue → en (titre : « ${title} »)`)
    : bad(`langue : html[lang]=${lang}, titre=${title}`);
  const desc = await page.getAttribute('meta[name="description"]', "content");
  desc?.startsWith("A Rust MITM proxy") ? ok("meta description traduite") : bad(`meta description : ${desc?.slice(0, 40)}`);

  // Changement de thème. `body` porte une transition de 0,2s sur
  // `background-color` : lire la couleur après un délai fixe attrape une
  // valeur intermédiaire dès que la machine est un peu lente (rgb(14,21,17)
  // au lieu de rgb(13,20,16) sur un runner d'intégration continue). On attend
  // que la peinture converge au lieu de deviner un délai — l'assertion porte
  // bien sur l'état final, elle ne le suppose plus atteint.
  const FOND_SOMBRE = "rgb(13, 20, 16)";
  await page.click('.switcher button[title="Dark"]');
  await until(
    page,
    (attendu) => getComputedStyle(document.body).backgroundColor === attendu,
    FOND_SOMBRE,
  );
  const theme = await page.getAttribute("html", "data-theme");
  const bg = await page.$eval("body", (e) => getComputedStyle(e).backgroundColor);
  theme === "dark" && bg === FOND_SOMBRE
    ? ok(`thème → dark (fond ${bg})`)
    : bad(`thème : data-theme=${theme}, fond=${bg}`);

  // Persistance.
  const stored = await page.evaluate(() => [localStorage.getItem("brocolis.lang"), localStorage.getItem("brocolis.theme")]);
  JSON.stringify(stored) === '["en","dark"]' ? ok("préférences persistées") : bad(`localStorage : ${JSON.stringify(stored)}`);

  await page.reload({ waitUntil: "networkidle" });
  await page.waitForSelector("nav.navbar");
  const afterReload = await page.evaluate(() => [document.documentElement.lang, document.documentElement.dataset.theme]);
  JSON.stringify(afterReload) === '["en","dark"]'
    ? ok("préférences restaurées au rechargement")
    : bad(`après rechargement : ${JSON.stringify(afterReload)}`);

  // Lien d'évitement.
  await page.keyboard.press("Tab");
  const focused = await page.evaluate(() => document.activeElement?.className);
  focused === "skip-link" ? ok("premier Tab = lien d'évitement") : bad(`premier Tab : ${focused}`);
  await page.keyboard.press("Enter");
  await until(page, () => document.activeElement?.id === "contenu");
  const focusedAfter = await page.evaluate(() => document.activeElement?.id);
  focusedAfter === "contenu" ? ok("le lien d'évitement déplace le focus sur <main>") : bad(`focus après Entrée : ${focusedAfter}`);
  const hashAfterSkip = await page.evaluate(() => location.hash);
  hashAfterSkip === "" ? ok("le lien d'évitement ne touche pas au fragment") : bad(`fragment après évitement : ${hashAfterSkip}`);

  errors.length === 0 ? ok("aucune erreur console (accueil)") : bad(`console : ${errors.join(" | ")}`);
  await ctx.close();
}

// --- 2. Page de téléchargement ---------------------------------------------
{
  const { ctx, page, errors } = await newPage({ width: 390, height: 844 });
  await load(page, BASE + "/");
  await page.evaluate(() => window.scrollTo(0, 2000));
  await page.click('.hero a.btn.primary, .platform-card a.btn.primary');
  await page.waitForSelector(".dl-title", { timeout: 10000 });
  await until(page, () => window.scrollY < 10);

  const top = await page.evaluate(() => window.scrollY);
  top < 10 ? ok(`arrivée en haut de la page de téléchargement (scrollY=${Math.round(top)})`) : bad(`scrollY=${Math.round(top)} à l'arrivée`);

  const t = await page.title();
  t === "Télécharger Brocolis — Brocolis 🥦" ? ok(`titre d'onglet : « ${t} »`) : bad(`titre : ${t}`);

  const hrefs = await page.$$eval("main a[href^='/dl/']", (els) => els.map((e) => e.getAttribute("href")));
  const want = ["/dl/macos", "/dl/linux-deb", "/dl/linux-rpm", "/dl/windows"];
  JSON.stringify(hrefs) === JSON.stringify(want) ? ok("4 liens /dl/* dans l'ordre") : bad(`liens : ${hrefs.join(",")}`);

  // Parité de gabarit avec l'accueil : c'est la régression qu'on a corrigée
  // trois fois à la main. On mesure, on ne regarde pas.
  const metrics = await page.evaluate(() => {
    const main = document.querySelector("main.dl-main");
    const card = document.querySelector(".dl-grid .card");
    const cs = getComputedStyle(main);
    return {
      padLeft: cs.paddingLeft,
      padRight: cs.paddingRight,
      cardLeft: Math.round(card.getBoundingClientRect().left),
      cardWidth: Math.round(card.getBoundingClientRect().width),
      titleSize: getComputedStyle(document.querySelector(".dl-title")).fontSize,
      overflow: document.documentElement.scrollWidth - window.innerWidth,
    };
  });
  note(`gabarit : padding ${metrics.padLeft}/${metrics.padRight}, carte à ${metrics.cardLeft}px large de ${metrics.cardWidth}px, titre ${metrics.titleSize}`);
  metrics.padLeft === "24px" && metrics.cardLeft === 24
    ? ok("les cartes respectent la gouttière de 24px (comme l'accueil)")
    : bad(`gouttière : padding=${metrics.padLeft}, carte à ${metrics.cardLeft}px`);
  metrics.overflow <= 0 ? ok("aucun débordement horizontal (téléchargement)") : bad(`débordement de ${metrics.overflow}px`);

  // Les deux CTA Linux doivent avoir la même hauteur (bug corrigé jadis).
  const h = await page.$$eval(".dl-btn-row .btn", (els) => els.map((e) => Math.round(e.getBoundingClientRect().height)));
  h.length === 2 && h[0] === h[1] ? ok(`les deux CTA Linux font ${h[0]}px`) : bad(`hauteurs des CTA Linux : ${h.join(" / ")}`);

  // Retour à l'accueil.
  await page.click(".dl-back");
  await page.waitForSelector(".hero", { timeout: 10000 });
  ok("le lien « Accueil » ramène au héros");

  // Accès direct par URL.
  await page.goto(BASE + "/#/telecharger", { waitUntil: "networkidle" });
  await page.waitForSelector(".dl-title", { timeout: 10000 });
  ok("accès direct à #/telecharger");

  // Ancienne URL → redirection.
  const r = await page.goto(BASE + "/telecharger", { waitUntil: "networkidle" });
  await page.waitForSelector(".dl-title", { timeout: 10000 });
  ok(`ancienne URL /telecharger redirigée (${r.status()}) et servie`);

  errors.length === 0 ? ok("aucune erreur console (téléchargement)") : bad(`console : ${errors.join(" | ")}`);
  await ctx.close();
}

// --- 3. Bureau : menu horizontal -------------------------------------------
{
  const { ctx, page, errors } = await newPage({ width: 1280, height: 900 });
  await load(page, BASE + "/");
  const toggle = await page.$eval(".nav-toggle", (e) => getComputedStyle(e).display);
  const drawer = await page.$eval("#nav-drawer", (e) => getComputedStyle(e).flexDirection);
  toggle === "none" && drawer === "row"
    ? ok("1280px : menu horizontal, pas de hamburger")
    : bad(`1280px : hamburger=${toggle}, tiroir=${drawer}`);
  const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  over <= 0 ? ok("aucun débordement horizontal (1280px)") : bad(`débordement de ${over}px`);
  errors.length === 0 ? ok("aucune erreur console (bureau)") : bad(`console : ${errors.join(" | ")}`);
  await ctx.close();
}

// --- 4. Le palier critique 761–960px (ancien débordement) -------------------
for (const w of [601, 760, 905, 959, 960]) {
  const { ctx, page } = await newPage({ width: w, height: 900 });
  await load(page, BASE + "/");
  // Tiroir ouvert : c'est dans cet état que la barre débordait.
  const hamburger = await page.$eval(".nav-toggle", (e) => getComputedStyle(e).display);
  if (hamburger !== "none") {
    await page.click(".nav-toggle");
    await until(page, () => getComputedStyle(document.querySelector("#nav-drawer")).display === "flex");
  }
  const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  over <= 0 ? ok(`${w}px : pas de débordement, menu ouvert`) : bad(`${w}px : débordement de ${over}px`);
  await ctx.close();
}

// --- 5. Les trois langues × les trois thèmes rendent sans erreur ------------
for (const lang of ["fr", "en", "es"]) {
  for (const theme of ["green", "light", "dark"]) {
    const { ctx, page, errors } = await newPage({ width: 390, height: 844 });
    await page.addInitScript(
      ([l, t]) => {
        localStorage.setItem("brocolis.lang", l);
        localStorage.setItem("brocolis.theme", t);
      },
      [lang, theme],
    );
    await load(page, BASE + "/");
    const seen = await page.evaluate(() => [document.documentElement.lang, document.documentElement.dataset.theme]);
    const footer = (await page.textContent("footer")) || "";
    const attributed = footer.includes("Maxime Nathan Lestage");
    const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    seen[0] === lang && seen[1] === theme && attributed && over <= 0 && errors.length === 0
      ? ok(`${lang}/${theme} : rendu, attribué, sans débordement`)
      : bad(`${lang}/${theme} : vu=${JSON.stringify(seen)}, attribution=${attributed}, débordement=${over}, erreurs=${errors.join("|")}`);
    await ctx.close();
  }
}

// --- 6. Détection de la langue du navigateur (sans préférence stockée) ------
for (const [locale, expected, marker] of [
  ["fr-FR", "fr", "Plateformes supportées"],
  ["en-GB", "en", "Supported platforms"],
  ["es-419", "es", "Plataformas soportadas"],
  ["de-DE", "fr", "Plateformes supportées"],
]) {
  const { ctx, page } = await newPage({ width: 390, height: 844 }, locale);
  await load(page, BASE + "/");
  const seen = await page.getAttribute("html", "lang");
  const heading = (await page.textContent("#platforms-h")) || "";
  seen === expected && heading.trim() === marker
    ? ok(`${locale} → ${expected} (« ${heading.trim()} »)`)
    : bad(`${locale} : attendu ${expected}/${marker}, obtenu ${seen}/${heading.trim()}`);
  await ctx.close();
}

await browser.close();
serveur?.proc.kill();

console.log("");
if (problems.length === 0) {
  console.log(`TOUT EST VERT (${checks} vérifications)`);
} else {
  console.log(`${problems.length} PROBLÈME(S) :`);
  for (const p of problems) console.log(`  - ${p}`);
  process.exitCode = 1;
}
