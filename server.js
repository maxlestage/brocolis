// Serveur statique minimal (sans dépendance) pour servir le site vitrine
// construit dans site/dist sur le port fourni par Heroku ($PORT).
//
// Pourquoi maison plutôt qu'un paquet (serve/express) : zéro dépendance =
// pas d'installation supplémentaire au build, et un comportement déterministe
// (liaison sur $PORT, repli SPA, types MIME explicites).

const http = require("http");
const fs = require("fs");
const path = require("path");
const { Readable } = require("node:stream");

const DIST = path.join(__dirname, "site", "dist");
const PORT = process.env.PORT || 5000;

// ---------------------------------------------------------------------------
// Téléchargements (/dl/*)
//
// Les binaires sont publiés comme assets de release sur le dépôt GitHub, qui
// est privé : une requête non authentifiée sur l'URL GitHub reçoit un 404. On
// relaie donc le fichier ici avec le jeton du serveur — le dépôt reste privé,
// les téléchargements deviennent publics.
//
// Le relais est fait à la volée plutôt que copié dans le slug au moment du
// build : les releases sont « rolling » (reconstruites à chaque push sur
// master), donc une copie au build servirait des binaires périmés entre deux
// déploiements, alors que la page annonce « les dernières builds ».
const GITHUB_API = process.env.GITHUB_API_BASE || "https://api.github.com";
const GITHUB_REPO = process.env.GITHUB_REPO || "maxlestage/brocolis";
const GITHUB_TOKEN = process.env.GITHUB_TOKEN || process.env.GH_TOKEN || "";

// Noms d'artefacts stables, cf. les workflows *-cli-release.yml.
const DOWNLOADS = {
  macos: { tag: "cli-latest", asset: "brocolis-macos-universal.tar.gz" },
  "linux-deb": { tag: "cli-latest-linux", asset: "brocolis-linux-amd64.deb" },
  "linux-rpm": { tag: "cli-latest-linux", asset: "brocolis-linux-x86_64.rpm" },
  windows: { tag: "cli-latest-windows", asset: "brocolis-windows-x86_64.zip" },
};

// L'identifiant d'un asset ne bouge qu'au réassemblage de la release : inutile
// de refaire un appel API à chaque téléchargement.
const ASSET_TTL_MS = 5 * 60 * 1000;
const assetCache = new Map();

function apiHeaders(accept) {
  const headers = {
    Accept: accept || "application/vnd.github+json",
    "User-Agent": "brocolis-site",
    "X-GitHub-Api-Version": "2022-11-28",
  };
  if (GITHUB_TOKEN) headers.Authorization = `Bearer ${GITHUB_TOKEN}`;
  return headers;
}

async function resolveAssetId(tag, assetName) {
  const key = `${tag}/${assetName}`;
  const cached = assetCache.get(key);
  if (cached && cached.expires > Date.now()) return cached.id;

  const res = await fetch(
    `${GITHUB_API}/repos/${GITHUB_REPO}/releases/tags/${tag}`,
    { headers: apiHeaders() },
  );
  if (!res.ok) throw new Error(`release ${tag} : HTTP ${res.status}`);
  const release = await res.json();
  const asset = (release.assets || []).find((a) => a.name === assetName);
  if (!asset) throw new Error(`asset ${assetName} absent de ${tag}`);

  assetCache.set(key, { id: asset.id, expires: Date.now() + ASSET_TTL_MS });
  return asset.id;
}

async function serveDownload(res, slug) {
  const entry = DOWNLOADS[slug];
  if (!entry) {
    res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    res.end("Not found");
    return;
  }
  if (!GITHUB_TOKEN) {
    res.writeHead(503, { "Content-Type": "text/plain; charset=utf-8" });
    res.end("Téléchargements indisponibles : GITHUB_TOKEN n'est pas configuré.");
    return;
  }

  try {
    const id = await resolveAssetId(entry.tag, entry.asset);
    // L'API répond par une redirection signée vers le stockage. On la suit
    // nous-mêmes pour ne surtout pas réémettre l'en-tête Authorization vers
    // un autre domaine.
    const first = await fetch(
      `${GITHUB_API}/repos/${GITHUB_REPO}/releases/assets/${id}`,
      { headers: apiHeaders("application/octet-stream"), redirect: "manual" },
    );

    let upstream = first;
    if (first.status >= 300 && first.status < 400) {
      const location = first.headers.get("location");
      if (!location) throw new Error("redirection sans en-tête Location");
      upstream = await fetch(location, {
        headers: { "User-Agent": "brocolis-site" },
      });
    }
    if (!upstream.ok || !upstream.body) {
      throw new Error(`asset ${entry.asset} : HTTP ${upstream.status}`);
    }

    const headers = {
      "Content-Type": "application/octet-stream",
      "Content-Disposition": `attachment; filename="${entry.asset}"`,
      // Build « rolling » : le contenu change sans que l'URL change.
      "Cache-Control": "no-store",
    };
    const length = upstream.headers.get("content-length");
    if (length) headers["Content-Length"] = length;

    res.writeHead(200, headers);
    Readable.fromWeb(upstream.body).pipe(res);
  } catch (err) {
    console.error(`[dl] ${slug} : ${err.message}`);
    if (res.headersSent) {
      res.destroy();
      return;
    }
    res.writeHead(502, { "Content-Type": "text/plain; charset=utf-8" });
    res.end("Téléchargement temporairement indisponible");
  }
}

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".txt": "text/plain; charset=utf-8",
};

function sendFile(res, filePath, status) {
  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
      res.end("Not found");
      return;
    }
    const ext = path.extname(filePath).toLowerCase();
    // Cache : les assets Vite sont fingerprintés (hash dans le nom) donc
    // immuables ; le HTML doit être revalidé à chaque visite, sinon un
    // navigateur garde un index.html périmé qui pointe vers des assets
    // supprimés au déploiement suivant (page blanche).
    const isHashedAsset = filePath.includes(`${path.sep}assets${path.sep}`);
    const cacheControl =
      ext === ".html"
        ? "no-cache"
        : isHashedAsset
          ? "public, max-age=31536000, immutable"
          : "public, max-age=3600";
    res.writeHead(status, {
      "Content-Type": MIME[ext] || "application/octet-stream",
      "Cache-Control": cacheControl,
    });
    res.end(data);
  });
}

const server = http.createServer((req, res) => {
  let urlPath = decodeURIComponent((req.url || "/").split("?")[0]);

  // Anciennes URLs (site multi-pages) et routes applicatives : on redirige
  // vers la route HashRouter correspondante pour ne casser aucun lien ou
  // favori existant.
  if (urlPath === "/telecharger.html" || urlPath === "/telecharger") {
    res.writeHead(302, {
      Location: "/#/telecharger",
      "Cache-Control": "no-cache",
    });
    res.end();
    return;
  }

  // Relais des binaires de release (dépôt privé) — voir DOWNLOADS plus haut.
  // Le segment n'est jamais utilisé comme chemin de fichier : il sert
  // uniquement de clé dans DOWNLOADS, donc pas de traversée possible.
  if (urlPath.startsWith("/dl/")) {
    serveDownload(res, urlPath.slice("/dl/".length));
    return;
  }

  if (urlPath.endsWith("/")) urlPath += "index.html";

  const filePath = path.normalize(path.join(DIST, urlPath));
  // Empêche la traversée de répertoire hors de DIST.
  if (filePath !== DIST && !filePath.startsWith(DIST + path.sep)) {
    res.writeHead(403, { "Content-Type": "text/plain; charset=utf-8" });
    res.end("Forbidden");
    return;
  }

  fs.stat(filePath, (err, stat) => {
    if (!err && stat.isFile()) {
      sendFile(res, filePath, 200);
      return;
    }
    // SPA React (HashRouter) : toute route applicative passe par index.html.
    // Une route sans extension retombe donc sur index.html ; une ressource
    // avec extension manquante reste un 404 franc.
    if (path.extname(filePath) === "") {
      sendFile(res, path.join(DIST, "index.html"), 200);
    } else {
      res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
      res.end("Not found");
    }
  });
});

server.listen(PORT, () => {
  console.log(`Site Brocolis servi sur le port ${PORT} (depuis ${DIST})`);
});
