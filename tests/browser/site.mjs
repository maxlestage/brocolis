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
 * Plusieurs rendus suivent un événement ou une image, et des propriétés sont animées : un
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
  const expected = ["platforms", "switch", "features", "quickstart", "app", "how", "ratchet", "privacy"];
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
  lang === "en" && title.includes("Encrypt what you mark")
    ? ok(`langue → en (titre : « ${title} »)`)
    : bad(`langue : html[lang]=${lang}, titre=${title}`);
  const desc = await page.getAttribute('meta[name="description"]', "content");
  desc?.startsWith("Brocolis is an app") ? ok("meta description traduite") : bad(`meta description : ${desc?.slice(0, 40)}`);

  // Changement de thème. `body` porte une transition de 0,2s sur
  // `background-color` : lire la couleur après un délai fixe attrape une
  // valeur intermédiaire dès que la machine est un peu lente (rgb(14,21,17)
  // au lieu de rgb(13,20,16) sur un runner d'intégration continue). On attend
  // que la peinture converge au lieu de deviner un délai — l'assertion porte
  // bien sur l'état final, elle ne le suppose plus atteint.
  const FOND_SOMBRE = "rgb(11, 13, 12)";
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

// --- 2 bis. Page « Protocole v3 » --------------------------------------------
{
  const { ctx, page, errors } = await newPage({ width: 390, height: 844 });
  await load(page, BASE + "/");
  // L'accueil y mène depuis sa section « Double cliquet ».
  await page.$eval('#ratchet a[href="#/protocole"]', (a) => a.click());
  await page.waitForSelector("#v3-sim", { timeout: 10000 });
  ok("la section « Double cliquet » mène à #/protocole");
  const ids = await page.$$eval(".v3-main section[id]", (els) => els.map((e) => e.id));
  const attendu = ["v4", "v3-change", "v3-sim", "v3-steps", "v3-token", "v3-threats", "v3-where", "v3-faq"];
  JSON.stringify(ids) === JSON.stringify(attendu)
    ? ok("protocole : les huit parties (v4 en tête), dans l'ordre")
    : bad(`protocole : ${JSON.stringify(ids)}`);
  const lignes = await page.$$eval(".v3-table.cmp:not(.v4) tbody tr", (r) => r.length);
  lignes === 7 ? ok("comparatif : 7 lignes") : bad(`comparatif : ${lignes} lignes`);
  const v4 = await page.$$eval(".v3-table.v4 tbody tr", (r) => r.length);
  const v4Cols = await page.$$eval(".v3-table.v4 thead th", (r) => r.length);
  v4 === 6 && v4Cols === 4
    ? ok("v4 : comparatif Signal / v3 / v4, 6 lignes")
    : bad(`v4 : ${v4} lignes, ${v4Cols} colonnes`);
  // Le simulateur : vous, vous, l'espion, vous, Alice, vous.
  const [ecrire, repondre, espion] = await page.$$(".sim-btn");
  const espionInactif = await espion.evaluate((b) => b.disabled);
  for (const b of [ecrire, ecrire]) await b.click();
  await espion.click();
  for (const b of [ecrire, repondre, ecrire]) await b.click();
  await until(page, () => document.querySelectorAll(".sim-msg").length === 5);
  const bilan = await page.evaluate(() => ({
    msgs: document.querySelectorAll(".sim-msg").length,
    tours: document.querySelectorAll(".sim-turn").length,
    lus: document.querySelectorAll(".sim-badge.bad").length,
    score: document.querySelector(".sim-score").textContent,
    espion: document.querySelectorAll(".sim-btn")[2].disabled,
  }));
  espionInactif &&
  bilan.msgs === 5 &&
  bilan.tours === 2 &&
  bilan.lus === 2 &&
  bilan.score === "L'espion lit 2 message(s) sur 5." &&
  bilan.espion
    ? ok(`simulateur : ${bilan.score} (avant la copie : hors de portée ; après la guérison : aussi)`)
    : bad(`simulateur : ${JSON.stringify(bilan)}`);
  await page.click(".sim-reset");
  await until(page, () => document.querySelector(".sim-empty") !== null);
  (await page.$(".sim-empty")) ? ok("simulateur : « Recommencer » vide le fil") : bad("simulateur : pas remis à zéro");
  // Une question s'ouvre.
  await page.click(".v3-faq summary");
  (await page.$eval(".v3-faq", (d) => d.open)) ? ok("FAQ : une question s'ouvre") : bad("FAQ fermée");
  const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  over <= 0 ? ok("protocole : aucun débordement horizontal (390px)") : bad(`protocole : débordement de ${over}px`);
  // Accès direct, et titre de l'onglet.
  await page.goto(BASE + "/#/protocole", { waitUntil: "networkidle" });
  await page.waitForSelector("#v3-title", { timeout: 10000 });
  const titre = await page.title();
  titre.startsWith("Protocole v3") ? ok(`accès direct à #/protocole (« ${titre} »)`) : bad(`titre : ${titre}`);
  errors.length === 0 ? ok("aucune erreur console (protocole)") : bad(`console : ${errors.join(" | ")}`);
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

// --- 7. Mouvement : rideau, poussière, apparitions, curseur ------------------
// Les animations sont pilotées par le WebAssembly et écrites en variables CSS
// ou en attributs : on lit ces valeurs, pas des captures.
{
  const { ctx, page, errors } = await newPage({ width: 1280, height: 800 });
  await load(page, BASE + "/");

  const motion = await page.evaluate(() => document.documentElement.classList.contains("motion"));
  motion ? ok("classe `motion` posée (pas de préférence de réduction)") : bad("classe `motion` absente");

  // Le rideau couvre l'arrivée, ne bloque aucun clic, puis disparaît.
  const curtainPE = await page.$eval(".curtain", (e) => getComputedStyle(e).pointerEvents).catch(() => null);
  curtainPE === "none" ? ok("rideau d'ouverture présent, transparent aux clics") : bad(`rideau : pointer-events=${curtainPE}`);
  await until(page, () => !document.querySelector(".curtain") && document.documentElement.classList.contains("ready"));
  const gone = await page.evaluate(() => [!document.querySelector(".curtain"), document.documentElement.classList.contains("ready")]);
  gone[0] && gone[1] ? ok("le rideau se lève puis disparaît (html.ready)") : bad(`rideau : retiré=${gone[0]}, ready=${gone[1]}`);

  // Une fois par session : un rechargement ne le rejoue pas.
  await page.reload({ waitUntil: "networkidle" });
  await page.waitForSelector("nav.navbar");
  const again = await page.$(".curtain");
  !again ? ok("rideau non rejoué au rechargement (une fois par session)") : bad("le rideau revient à chaque rechargement");

  // Scène épinglée : plus haute que l'écran, intérieur collant.
  const pin = await page.evaluate(() => ({
    pin: document.querySelector(".hero-pin").getBoundingClientRect().height,
    stage: getComputedStyle(document.querySelector(".hero-stage")).position,
  }));
  pin.pin > 800 * 2 && pin.stage === "sticky"
    ? ok(`scène du héros épinglée (${Math.round(pin.pin)}px de course, sticky)`)
    : bad(`scène : hauteur=${pin.pin}, position=${pin.stage}`);

  // La poussière : la progression suit le défilement, et le canevas peint.
  const at = async (y) => {
    await page.evaluate((v) => window.scrollTo(0, v), y);
    await until(page, (v) => Math.abs(window.scrollY - v) < 2, y);
    await page.waitForFunction(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
    return page.evaluate(() => {
      const c = document.querySelector("canvas.hero-dust");
      const d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
      let painted = 0;
      for (let i = 3; i < d.length; i += 16) if (d[i] > 0) painted++;
      return { hero: parseFloat(document.querySelector(".hero-pin").style.getPropertyValue("--hero")), painted, w: c.width };
    });
  };
  const debut = await at(0);
  const milieu = await at(700);
  const fin = await at(1700);
  debut.hero === 0 && milieu.hero > 0.3 && milieu.hero < 0.8 && fin.hero === 1
    ? ok(`progression de la scène : ${debut.hero} → ${milieu.hero.toFixed(2)} → ${fin.hero}`)
    : bad(`progression : ${debut.hero} / ${milieu.hero} / ${fin.hero}`);
  debut.w > 0 && debut.painted > 100 && milieu.painted > 100
    ? ok(`la poussière est dessinée (${milieu.painted} pixels échantillonnés peints)`)
    : bad(`canevas : largeur=${debut.w}, peints=${debut.painted}/${milieu.painted}`);
  const photo = await page.$eval(".hero-art img", (e) => getComputedStyle(e).opacity);
  photo === "1" ? ok("la photo recouvre la mosaïque une fois formée") : bad(`opacité de la photo en fin de scène : ${photo}`);

  // Apparitions : caché avant, révélé en entrant dans la vue.
  // Les mots du titre sont sous leur ligne, puis montent.
  const mot = () => page.$eval("#privacy-h .w > span", (e) => getComputedStyle(e).transform);
  const avant = [await page.$eval("#privacy-h", (e) => e.hasAttribute("data-in")), await mot()];
  await page.evaluate(() => document.querySelector("#privacy").scrollIntoView());
  await until(page, () => document.querySelector("#privacy-h").hasAttribute("data-in"));
  await until(page, () => getComputedStyle(document.querySelector("#privacy-h .w > span")).transform === "none");
  const apres = [await page.$eval("#privacy-h", (e) => e.hasAttribute("data-in")), await mot()];
  const titre = await page.$eval("#privacy-h", (e) => e.textContent);
  !avant[0] && avant[1] !== "none" && apres[0] && apres[1] === "none"
    ? ok("un titre se dévoile mot à mot en entrant dans la vue")
    : bad(`apparition : avant=${JSON.stringify(avant)}, après=${JSON.stringify(apres)}`);
  titre === "Vie privée et sécurité"
    ? ok(`le découpage en mots garde le texte intact (« ${titre} »)`)
    : bad(`texte du titre altéré : ${JSON.stringify(titre)}`);

  // Barre de navigation : elle s'efface en descendant, revient en remontant.
  await page.evaluate(() => window.scrollBy(0, 400));
  await until(page, () => document.documentElement.classList.contains("nav-hidden"));
  const cachee = await page.evaluate(() => document.documentElement.classList.contains("nav-hidden"));
  await page.evaluate(() => window.scrollBy(0, -200));
  await until(page, () => !document.documentElement.classList.contains("nav-hidden"));
  const revenue = await page.evaluate(() => !document.documentElement.classList.contains("nav-hidden"));
  cachee && revenue ? ok("la barre s'efface en descendant et revient en remontant") : bad(`barre : cachée=${cachee}, revenue=${revenue}`);

  // Manifeste : l'avancement est écrit sur le bloc.
  await page.evaluate(() => document.querySelector(".manifesto").scrollIntoView({ block: "center" }));
  await until(page, () => parseFloat(document.querySelector(".manifesto").style.getPropertyValue("--p")) > 0);
  const p = await page.$eval(".manifesto", (e) => parseFloat(e.style.getPropertyValue("--p")));
  p > 0 && p <= 1 ? ok(`le manifeste s'allume au défilement (--p=${p.toFixed(2)})`) : bad(`manifeste : --p=${p}`);

  // Curseur anneau et aimantation (souris).
  const btn = await page.$(".footer .btn.primary");
  await btn.scrollIntoViewIfNeeded();
  const box = await btn.boundingBox();
  await page.mouse.move(box.x + box.width / 2 + 20, box.y + box.height / 2 + 10);
  await page.mouse.move(box.x + box.width / 2 + 24, box.y + box.height / 2 + 12);
  await until(page, () => document.querySelector(".cursor").hasAttribute("data-hover"));
  const cur = await page.$eval(".cursor", (e) => [getComputedStyle(e).display, e.hasAttribute("data-on"), e.hasAttribute("data-hover")]);
  cur[0] === "block" && cur[1] && cur[2] ? ok("curseur anneau affiché, grossi sur un bouton") : bad(`curseur : ${JSON.stringify(cur)}`);
  const mx = await btn.evaluate((e) => parseFloat(e.style.getPropertyValue("--mx")));
  mx > 0 ? ok(`le bouton est attiré par le pointeur (--mx=${mx}px)`) : bad(`aimantation : --mx=${mx}`);

  // Volet de transition au changement de page.
  await page.click('.footer a.btn.primary');
  await page.waitForSelector(".dl-title");
  const wipe = await page.$(".wipe");
  wipe ? ok("volet de transition joué au changement de page") : bad("pas de volet au changement de page");

  errors.length === 0 ? ok("aucune erreur console (mouvement)") : bad(`console : ${errors.join(" | ")}`);
  await ctx.close();
}

// --- 7 bis. Mouvement, deuxième passe -------------------------------------
{
  const { ctx, page, errors } = await newPage({ width: 1280, height: 800 });
  await load(page, BASE + "/");
  await until(page, () => document.documentElement.classList.contains("ready"));

  // Texte « déchiffré » : il défile puis revient exactement à l'original.
  const eyebrow = "[data-scramble]";
  await until(page, (s) => document.querySelector(".hero-eyebrow " + s).textContent === "Application · Double cliquet · Rust", eyebrow);
  const net = await page.$eval(".hero-eyebrow [data-scramble]", (e) => e.textContent);
  net === "Application · Double cliquet · Rust" ? ok("le texte déchiffré revient intact") : bad(`surtitre : ${net}`);

  // Pendant un déchiffrage, la langue change : c'est la nouvelle qui gagne.
  const box = await page.$eval(".hero-eyebrow [data-scramble]", (e) => { const r = e.getBoundingClientRect(); return [r.x + 20, r.y + r.height / 2]; });
  await page.mouse.move(box[0], box[1]);
  await until(page, () => document.querySelector(".hero-eyebrow [data-scramble]").textContent !== "Application · Double cliquet · Rust");
  const pendant = await page.$eval(".hero-eyebrow [data-scramble]", (e) => e.textContent);
  await page.click('.switcher button:has-text("EN")');
  await until(page, () => document.querySelector(".hero-eyebrow [data-scramble]").textContent === "App · Double ratchet · Rust");
  const apres = await page.$eval(".hero-eyebrow [data-scramble]", (e) => e.textContent);
  apres === "App · Double ratchet · Rust" && pendant !== "Application · Double cliquet · Rust"
    ? ok(`changement de langue en plein déchiffrage : texte final anglais (pendant : « ${pendant} »)`)
    : bad(`après changement de langue : « ${apres} » (pendant : « ${pendant} »)`);
  await page.click('.switcher button:has-text("FR")');
  await until(page, () => document.documentElement.lang === "fr");

  // Démonstration : le secret se scelle en jeton, puis se rouvre chez les proches.
  await page.evaluate(() => document.querySelector(".demo-board").scrollIntoView({ block: "center" }));
  await until(page, () => document.querySelector(".demo-row:nth-child(2) .demo-token")?.textContent.startsWith("[[BROCOLIS:v3:"));
  const jeton = await page.$eval(".demo-row:nth-child(2) .demo-token", (e) => e.textContent);
  await page.waitForFunction(() => document.querySelector(".demo-row:nth-child(3) .demo-token.clear")?.textContent === "demain à 18h", null, { timeout: 12000 }).catch(() => {});
  const clair = await page.$eval(".demo-row:nth-child(3) .demo-token", (e) => e.textContent);
  jeton.startsWith("[[BROCOLIS:v3:") && clair === "demain à 18h"
    ? ok(`démonstration : « ${jeton} » côté serveur, « ${clair} » chez les proches`)
    : bad(`démonstration : serveur=${jeton}, proches=${clair}`);

  // Bandeau : il avance tout seul.
  await page.evaluate(() => document.querySelector(".marquee").scrollIntoView({ block: "center" }));
  // Le défilement est doux : on attend que le bandeau soit réellement à
  // l'écran (il ne s'anime qu'une fois visible) avant de mesurer.
  await until(page, () => {
    const r = document.querySelector(".marquee").getBoundingClientRect();
    return r.top >= 0 && r.bottom <= innerHeight;
  });
  const pos = () => page.$eval(".marquee .marquee-track", (e) => new DOMMatrix(getComputedStyle(e).transform).m41);
  const x0 = await pos();
  await page.waitForTimeout(400);
  const x1 = await pos();
  Math.abs(x1 - x0) > 5 ? ok(`le bandeau défile (${Math.round(x0)} → ${Math.round(x1)}px)`) : bad(`bandeau immobile (${x0} → ${x1})`);

  // Carte : inclinée sous le pointeur, redressée en la quittant.
  const card = await page.$("#platforms .card");
  await card.scrollIntoViewIfNeeded();
  await until(page, () => document.querySelector("#platforms .card").hasAttribute("data-in"));
  const cb = await card.boundingBox();
  await page.mouse.move(cb.x + cb.width * 0.85, cb.y + cb.height * 0.2);
  await page.mouse.move(cb.x + cb.width * 0.9, cb.y + cb.height * 0.15);
  const incl = await card.evaluate((e) => [e.style.getPropertyValue("--rx"), e.style.getPropertyValue("--ry")]);
  await page.mouse.move(5, 400);
  await page.mouse.move(6, 401);
  const repos = await card.evaluate((e) => e.style.getPropertyValue("--ry"));
  parseFloat(incl[1]) > 0 && parseFloat(incl[0]) > 0 && repos === ""
    ? ok(`carte inclinée sous le pointeur (${incl.join(", ")}), redressée ensuite`)
    : bad(`inclinaison : ${JSON.stringify(incl)}, au repos : ${JSON.stringify(repos)}`);

  // Section courante : rang sur <html>, lien marqué, repère affiché.
  await page.evaluate(() => document.querySelector("#features").scrollIntoView());
  await until(page, () => document.querySelector('.nav-links a[href="#features"]').hasAttribute("data-current"));
  const spy = await page.evaluate(() => [
    document.documentElement.style.getPropertyValue("--spy"),
    document.querySelector('.nav-links a[href="#features"]').hasAttribute("data-current"),
    document.querySelectorAll(".nav-links a[data-current]").length,
    document.documentElement.classList.contains("spy-on"),
  ]);
  spy[0] === "2" && spy[1] && spy[2] === 1 && spy[3]
    ? ok("section courante suivie (--spy=2, lien « Fonctionnalités » marqué)")
    : bad(`section courante : ${JSON.stringify(spy)}`);

  // « Un clic » : l'interrupteur de la maquette s'allume en entrant dans la vue.
  await page.evaluate(() => document.querySelector("#switch .switch-mock").scrollIntoView({ block: "center" }));
  await until(page, () => getComputedStyle(document.querySelector("#switch .when-on")).opacity === "1");
  const allume = await page.evaluate(() => ({
    on: getComputedStyle(document.querySelector("#switch .when-on")).opacity,
    off: getComputedStyle(document.querySelector("#switch .when-off")).opacity,
    disque: getComputedStyle(document.querySelector("#switch .power-disc")).backgroundColor,
  }));
  allume.on === "1" && allume.off === "0" && allume.disque === "rgb(200, 242, 92)"
    ? ok("l'interrupteur de la maquette s'allume (« Protection active », disque citron vert)")
    : bad(`interrupteur : ${JSON.stringify(allume)}`);

  // Visite de l'application : de vrais onglets, à la souris et au clavier.
  await page.evaluate(() => document.querySelector("#app .app-tour").scrollIntoView({ block: "center" }));
  const onglet = () =>
    page.evaluate(() => ({
      choisi: document.querySelector('#app [role="tab"][aria-selected="true"]')?.id,
      focus: document.activeElement?.id,
      panneau: document.querySelector("#app-panel")?.getAttribute("aria-labelledby"),
      texte: document.querySelector("#app-panel")?.textContent ?? "",
      legende: document.querySelector(".app-caption")?.textContent ?? "",
    }));
  await page.click("#app-tab-1");
  const contacts = await onglet();
  contacts.choisi === "app-tab-1" && contacts.panneau === "app-tab-1" && contacts.texte.includes("Alice") && contacts.texte.includes("Changer de clé") && contacts.legende.startsWith("02")
    ? ok("visite : l'onglet Contacts montre Alice et « Changer de clé »")
    : bad(`onglet Contacts : ${JSON.stringify(contacts)}`);
  await page.keyboard.press("ArrowRight");
  const message = await onglet();
  message.choisi === "app-tab-2" && message.focus === "app-tab-2" && message.texte.includes("Message entier chiffré pour Alice")
    ? ok("visite : flèche droite → onglet Message, focus compris")
    : bad(`onglet Message au clavier : ${JSON.stringify(message)}`);
  await page.keyboard.press("Home");
  await page.click("#app-panel .power");
  const coupe = await page.evaluate(() => [
    document.querySelector("#app-panel .power").getAttribute("aria-pressed"),
    document.querySelector("#app-panel .mock-title").textContent,
  ]);
  coupe[0] === "false" && coupe[1] === "Protection coupée"
    ? ok("visite : l'interrupteur de l'onglet Protection se coupe au clic")
    : bad(`interrupteur de la visite : ${JSON.stringify(coupe)}`);

  errors.length === 0 ? ok("aucune erreur console (deuxième passe)") : bad(`console : ${errors.join(" | ")}`);
  await ctx.close();
}

// --- 8. Mouvement réduit : rien de caché, rien d'épinglé --------------------
{
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, locale: "fr-FR", reducedMotion: "reduce" });
  const page = await ctx.newPage();
  await load(page, BASE + "/");
  const etat = await page.evaluate(() => ({
    motion: document.documentElement.classList.contains("motion"),
    rideau: !!document.querySelector(".curtain"),
    pin: document.querySelector(".hero-pin").getBoundingClientRect().height,
    stage: document.querySelector(".hero-stage").getBoundingClientRect().height,
    photo: getComputedStyle(document.querySelector(".hero-art img")).opacity,
    caches: [...document.querySelectorAll("[data-reveal]")].filter((e) => getComputedStyle(e).opacity !== "1").length,
    lettres: [...document.querySelectorAll(".hero-title .char")].filter((e) => getComputedStyle(e).transform !== "none").length,
    demo: document.querySelector(".demo-row:nth-child(3) .demo-token")?.textContent,
    interrupteur: [
      getComputedStyle(document.querySelector("#switch .when-on")).opacity,
      getComputedStyle(document.querySelector("#switch .when-off")).visibility,
    ],
    message: getComputedStyle(document.querySelector("#app .app-panel")).opacity,
  }));
  !etat.motion && !etat.rideau ? ok("mouvement réduit : ni classe `motion`, ni rideau") : bad(`réduit : motion=${etat.motion}, rideau=${etat.rideau}`);
  Math.abs(etat.pin - etat.stage) < 1 && etat.photo === "1"
    ? ok("mouvement réduit : héros non épinglé, mascotte affichée d'emblée")
    : bad(`réduit : scène ${etat.pin}px pour ${etat.stage}px, photo ${etat.photo}`);
  etat.caches === 0 && etat.lettres === 0
    ? ok("mouvement réduit : aucun contenu caché en attente d'animation")
    : bad(`réduit : ${etat.caches} bloc(s) et ${etat.lettres} lettre(s) masqués`);
  etat.demo === "demain à 18h"
    ? ok("mouvement réduit : démonstration figée sur son état final, lisible")
    : bad(`réduit : démonstration « ${etat.demo} »`);
  etat.interrupteur[0] === "1" && etat.interrupteur[1] === "hidden" && etat.message === "1"
    ? ok("mouvement réduit : maquettes affichées allumées, sans attendre d'animation")
    : bad(`réduit : maquettes ${JSON.stringify(etat)}`);
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
