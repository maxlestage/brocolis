// Garde-fou de déploiement : vérifie que le site compilé présent dans
// site/dist est complet AVANT que le dyno ne commence à le servir.
//
// Pourquoi c'est nécessaire ici : le site est une application WebAssembly
// construite par Trunk (chaîne Rust), alors que Heroku construit ce dépôt avec
// le buildpack Node. Le build ne peut donc pas être refait sur le dyno, et
// site/dist est versionné. Si quelqu'un oublie de reconstruire après avoir
// touché au front, on veut un échec de déploiement explicite plutôt qu'une
// page blanche en production.
//
// Lancé par `npm run heroku-postbuild` (donc par Heroku) et par la CI.

import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const DIST = path.join(ROOT, "site", "dist");
const INDEX = path.join(DIST, "index.html");

function die(message) {
  console.error(`✗ ${message}`);
  console.error("");
  console.error("  Reconstruisez le site puis committez site/dist :");
  console.error("      cd site && trunk build --release");
  process.exit(1);
}

if (!existsSync(INDEX)) {
  die(`site/dist/index.html est absent (attendu : ${INDEX})`);
}

const html = readFileSync(INDEX, "utf8");

// Toutes les URL locales référencées par la page : `./x-<empreinte>.js`,
// `./styles-<empreinte>.css`, `./brocolisMan.png`… Les URL absolues
// (http://, //) sont ignorées, elles ne sont pas servies par ce serveur.
//
// La valeur d'attribut peut être entre guillemets doubles, simples, ou SANS
// guillemets : Trunk minifie le HTML en production et les retire (`href=./x.js`).
const ATTRIBUT = /(?:src|href)=(?:"([^"]*)"|'([^']*)'|([^\s"'>]+))/g;
const MODULE_WASM = /module_or_path:\s*['"]([^'"]+)['"]/g;

const referenced = new Set(
  [...html.matchAll(ATTRIBUT)]
    .map((m) => m[1] ?? m[2] ?? m[3])
    .concat([...html.matchAll(MODULE_WASM)].map((m) => m[1]))
    .filter((u) => u && !/^(?:https?:|\/\/|data:|#)/.test(u))
    .map((u) => u.replace(/^\.?\//, "").split(/[?#]/)[0])
    .filter(Boolean),
);

// Vérifié AVANT l'existence des fichiers : une extraction qui ne trouve plus
// rien passerait sinon pour un succès. C'est exactement ce qui est arrivé en
// activant la minification — les guillemets disparus, le motif ne reconnaissait
// plus que le `.wasm`, et le garde-fou ne gardait plus rien.
for (const extension of [".wasm", ".js", ".css", ".png"]) {
  if (![...referenced].some((rel) => rel.endsWith(extension))) {
    die(
      `site/dist/index.html ne référence aucun fichier ${extension} ` +
        `(trouvés : ${[...referenced].join(", ") || "aucun"})`,
    );
  }
}

const missing = [...referenced].filter(
  (rel) => !existsSync(path.join(DIST, rel)),
);
if (missing.length > 0) {
  die(
    `site/dist/index.html référence des fichiers absents : ${missing.join(", ")}`,
  );
}

console.log(
  `✓ site/dist prêt (${referenced.size} fichiers référencés, tous présents)`,
);
