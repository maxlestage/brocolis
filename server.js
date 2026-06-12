// Serveur statique minimal (sans dépendance) pour servir le site vitrine
// construit dans site/dist sur le port fourni par Heroku ($PORT).
//
// Pourquoi maison plutôt qu'un paquet (serve/express) : zéro dépendance =
// pas d'installation supplémentaire au build, et un comportement déterministe
// (liaison sur $PORT, repli SPA, types MIME explicites).

const http = require("http");
const fs = require("fs");
const path = require("path");

const DIST = path.join(__dirname, "site", "dist");
const PORT = process.env.PORT || 5000;

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
