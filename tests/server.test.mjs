// Tests de non-régression du serveur statique (server.js).
//
// Ils existent parce que trois requêtes triviales suffisaient à TUER le
// processus (« /% », « /%00 ») ou à consommer le quota d'API du jeton
// (« /dl/constructor »). Sans garde-fou automatisé, ce genre de défaut revient.
//
// Aucune dépendance : `node --test tests/` suffit. Un faux api.github.com est
// démarré ici même, donc aucun accès réseau n'est nécessaire.

import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const DIST = path.join(ROOT, "site", "dist");

/** Premier fichier de `site/dist` dont le nom porte une empreinte de contenu. */
function hashedFile(extension) {
  const found = fs
    .readdirSync(DIST)
    .find((name) => /-[0-9a-f]{16}(?:_bg)?\./.test(name) && name.endsWith(extension));
  assert.ok(found, `le build doit produire un fichier ${extension} empreinté`);
  return found;
}

const ASSET_NAME = "brocolis-macos-universal.tar.gz";
const BODY = Buffer.alloc(4096, 0x42);

let stub, storage, server, base;

/** Attend que le serveur réponde, ou échoue au bout de ~5 s. */
async function waitReady(url) {
  for (let i = 0; i < 50; i++) {
    try {
      const r = await fetch(url, { signal: AbortSignal.timeout(500) });
      if (r.ok) return;
    } catch {
      /* pas encore prêt */
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`serveur injoignable : ${url}`);
}

/** Émet une requête GET brute, sans normalisation du chemin par le client. */
function rawGet(rawPath) {
  return new Promise((resolve, reject) => {
    const { port } = new URL(base);
    const sock = net.connect(Number(port), "127.0.0.1", () => {
      sock.write(`GET ${rawPath} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n`);
    });
    let buf = "";
    sock.on("data", (d) => (buf += d));
    sock.on("end", () => {
      const m = buf.match(/^HTTP\/1\.\d (\d{3})/);
      m ? resolve(Number(m[1])) : reject(new Error(`réponse illisible : ${buf.slice(0, 80)}`));
    });
    sock.on("error", reject);
    sock.setTimeout(5000, () => sock.destroy(new Error("délai dépassé")));
  });
}

function listen(handler) {
  return new Promise((resolve) => {
    const s = http.createServer(handler);
    s.listen(0, () => resolve(s));
  });
}

before(async () => {
  storage = await listen((req, res) => {
    res.writeHead(200, { "Content-Length": String(BODY.length) });
    res.end(BODY);
  });
  const storagePort = storage.address().port;

  stub = await listen((req, res) => {
    if (/\/releases\/tags\//.test(req.url)) {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ assets: [{ id: 7, name: ASSET_NAME }] }));
      return;
    }
    if (/\/releases\/assets\/7$/.test(req.url)) {
      res.writeHead(302, { Location: `http://127.0.0.1:${storagePort}/blob` });
      res.end();
      return;
    }
    res.writeHead(404);
    res.end();
  });

  server = spawn("node", [path.join(ROOT, "server.js")], {
    env: {
      ...process.env,
      PORT: "0",
      GITHUB_TOKEN: "jeton-de-test",
      GITHUB_API_BASE: `http://127.0.0.1:${stub.address().port}`,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });

  // server.js annonce son port sur la sortie standard.
  const port = await new Promise((resolve, reject) => {
    let buf = "";
    server.stdout.on("data", (d) => {
      buf += d;
      const m = buf.match(/port (\d+)/);
      if (m) resolve(Number(m[1]));
    });
    server.on("exit", (c) => reject(new Error(`serveur sorti (code ${c})`)));
    setTimeout(() => reject(new Error("démarrage trop long")), 10000);
  });
  base = `http://127.0.0.1:${port}`;
  await waitReady(base + "/");
});

after(() => {
  server?.kill();
  stub?.close();
  storage?.close();
});

/** Le serveur doit être TOUJOURS debout après la requête testée. */
async function stillAlive() {
  const r = await fetch(base + "/", { signal: AbortSignal.timeout(3000) });
  assert.equal(r.status, 200, "le serveur doit avoir survécu à la requête");
}

test("un pourcentage mal formé renvoie 400 sans tuer le processus", async () => {
  for (const p of ["/%", "/%E0%A4%A", "/%zz"]) {
    const r = await fetch(base + p);
    assert.equal(r.status, 400, `${p} doit répondre 400`);
    await stillAlive();
  }
});

test("un octet nul renvoie 400 sans tuer le processus", async () => {
  const r = await fetch(base + "/%00");
  assert.equal(r.status, 400);
  await stillAlive();
});

test("la traversée de répertoire est refusée", async () => {
  // `fetch` normalise « /../ » AVANT d'émettre la requête : le serveur ne
  // verrait jamais le chemin qu'on veut tester. On écrit donc la requête HTTP
  // à la main, telle qu'un attaquant l'enverrait.
  for (const p of ["/../package.json", "/a/../../server.js"]) {
    const status = await rawGet(p);
    assert.equal(status, 403, `${p} doit répondre 403`);
  }
});

test("les clés héritées d'Object ne sont pas des téléchargements valides", async () => {
  for (const slug of ["constructor", "__proto__", "toString", "valueOf", "hasOwnProperty"]) {
    const r = await fetch(`${base}/dl/${slug}`);
    assert.equal(r.status, 404, `/dl/${slug} doit répondre 404`);
  }
});

test("un téléchargement connu est relayé avec le bon nom de fichier", async () => {
  const r = await fetch(base + "/dl/macos");
  assert.equal(r.status, 200);
  assert.match(r.headers.get("content-disposition"), new RegExp(ASSET_NAME));
  assert.equal(r.headers.get("cache-control"), "no-store");
  const buf = Buffer.from(await r.arrayBuffer());
  assert.equal(buf.length, BODY.length, "le corps doit arriver en entier");
});

test("les anciennes URL redirigent vers la route à dièse", async () => {
  for (const p of ["/telecharger", "/telecharger.html"]) {
    const r = await fetch(base + p, { redirect: "manual" });
    assert.equal(r.status, 302);
    assert.equal(r.headers.get("location"), "/#/telecharger");
  }
});

test("le module WebAssembly est servi en application/wasm", async () => {
  // `WebAssembly.instantiateStreaming` refuse tout autre type : un mauvais
  // MIME ici ne casse pas une ressource secondaire, il laisse la page vide.
  const r = await fetch(`${base}/${hashedFile(".wasm")}`);
  assert.equal(r.status, 200);
  assert.equal(r.headers.get("content-type"), "application/wasm");
});

test("les fichiers empreintés sont immuables, index.html revalidé", async () => {
  for (const ext of [".js", ".wasm", ".css"]) {
    const r = await fetch(`${base}/${hashedFile(ext)}`);
    assert.equal(r.status, 200, `${ext} doit être servi`);
    assert.match(
      r.headers.get("cache-control"),
      /immutable/,
      `${ext} empreinté doit être déclaré immuable`,
    );
  }
  const index = await fetch(base + "/");
  assert.equal(index.headers.get("cache-control"), "no-cache");
});

test("une route applicative retombe sur index.html, une ressource absente non", async () => {
  const spa = await fetch(base + "/une/route/inconnue");
  assert.equal(spa.status, 200);
  const missing = await fetch(base + "/assets/absent.js");
  assert.equal(missing.status, 404);
});

test("l'abandon du client ne perturbe pas le serveur", async () => {
  for (let i = 0; i < 3; i++) {
    const ctrl = new AbortController();
    const p = fetch(base + "/dl/macos", { signal: ctrl.signal }).then(
      (r) => r.body.getReader().read().then(() => ctrl.abort()),
      () => {},
    );
    await p.catch(() => {});
  }
  await stillAlive();
});
