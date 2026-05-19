#!/usr/bin/env bash
# Test d'intégration macOS pour `brocolis auto`.
#
# Vérifie, sur une vraie machine macOS :
#   - l'ajout du CA au trousseau Système, puis son retrait à l'arrêt ;
#   - le réglage du proxy web/HTTPS système, puis sa restauration ;
#   - la pose/retrait de la politique Firefox et du profil .mobileconfig ;
#   - un vrai flux HTTPS au travers du proxy (MITM + certificat forgé) ;
#   - le round-trip du chiffrement sélectif.
#
# Usage (depuis la racine du dépôt) :
#   bash scripts/test-macos.sh
#
# Le script compile la CLI puis demande le mot de passe administrateur
# (une fois) pour la partie système. Il restaure l'état initial en sortant.
set -u

SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [ "$(uname)" != "Darwin" ]; then
  echo "Ce script doit être exécuté sur macOS." >&2
  exit 2
fi

# Phase 1 (utilisateur) : compilation, puis élévation des privilèges.
if [ "$(id -u)" -ne 0 ]; then
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo introuvable — installez Rust (https://rustup.rs)." >&2
    exit 2
  fi
  echo "==> Compilation (cargo build --release --bin brocolis)…"
  cargo build --release --bin brocolis || exit 1
  echo "==> Élévation des privilèges (sudo) pour la partie système…"
  exec sudo "$SELF" "$@"
fi

# Phase 2 (root).
BIN="$ROOT_DIR/target/release/brocolis"
if [ ! -x "$BIN" ]; then
  echo "Binaire absent : $BIN" >&2
  echo "Lancez ce script SANS sudo (il compilera puis s'élèvera seul)." >&2
  exit 2
fi

PORT=8888
ECHO_PORT=8899
KEYCHAIN="/Library/Keychains/System.keychain"
CA_CN="Brocolis Root CA"
FF_POLICY="/Applications/Firefox.app/Contents/Resources/distribution/policies.json"

PASS=0
FAIL=0
ok() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
ko() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

BROCOLIS_PID=""
ECHO_PID=""
WORK="$(mktemp -d /tmp/brocolis-macos-test.XXXXXX)"

primary_service() {
  networksetup -listallnetworkservices | tail -n +2 | grep -v '^\*' | head -1
}
SVC="$(primary_service)"
if [ -z "$SVC" ]; then
  echo "Aucun service réseau actif détecté." >&2
  exit 2
fi
echo "Service réseau testé : « $SVC »"

# État initial à restaurer/vérifier.
PRE_WEB="$(networksetup -getwebproxy "$SVC")"
PRE_SEC="$(networksetup -getsecurewebproxy "$SVC")"
if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
  CA_PRE=1
else
  CA_PRE=0
fi
if [ -f "$FF_POLICY" ]; then FF_PRE=1; else FF_PRE=0; fi

cleanup() {
  # `brocolis auto` restaure sur SIGINT ; on lui laisse le temps de le faire.
  if [ -n "$BROCOLIS_PID" ] && kill -0 "$BROCOLIS_PID" 2>/dev/null; then
    kill -INT "$BROCOLIS_PID" 2>/dev/null
    for _ in $(seq 1 50); do
      kill -0 "$BROCOLIS_PID" 2>/dev/null || break
      sleep 0.2
    done
  fi
  [ -n "$ECHO_PID" ] && kill "$ECHO_PID" 2>/dev/null
  rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

cd "$WORK"

KEY="$("$BIN" keygen | head -1)"
cat > brocolis.toml <<EOF
listen = "127.0.0.1:$PORT"
ca_cert = "brocolis-ca.crt"
ca_key = "brocolis-ca.key"
key = "$KEY"
encrypt_hosts = []
EOF
"$BIN" ca --ca-cert brocolis-ca.crt --ca-key brocolis-ca.key >/dev/null

# Origine d'écho locale (pour le round-trip de chiffrement), si python3 dispo.
HAVE_PY=0
if command -v python3 >/dev/null 2>&1; then
  HAVE_PY=1
  cat > echo.py <<'PY'
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer
PORT = int(sys.argv[1])
class E(BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers.get('Content-Length', 0))
        b = self.rfile.read(n)
        sys.stderr.write("ORIGIN:" + b.decode('utf-8', 'replace') + "\n")
        sys.stderr.flush()
        self.send_response(200)
        self.send_header('Content-Type', 'text/plain')
        self.send_header('Content-Length', str(len(b)))
        self.end_headers()
        self.wfile.write(b)
    def log_message(self, *a):
        pass
HTTPServer(('127.0.0.1', PORT), E).serve_forever()
PY
  python3 echo.py "$ECHO_PORT" 2>echo.log &
  ECHO_PID=$!
fi

echo "==> Démarrage de « brocolis auto »…"
RUST_LOG=info "$BIN" auto --config brocolis.toml >auto.log 2>&1 &
BROCOLIS_PID=$!

for _ in $(seq 1 60); do
  grep -q "écoute" auto.log 2>/dev/null && break
  kill -0 "$BROCOLIS_PID" 2>/dev/null || break
  sleep 0.2
done
if grep -q "écoute" auto.log 2>/dev/null; then
  ok "proxy démarré"
else
  ko "proxy non démarré"
  echo "---- auto.log ----"; cat auto.log
  exit 1
fi

echo "== Pendant l'exécution =="

if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
  ok "CA présent dans le trousseau Système"
else
  ko "CA absent du trousseau Système"
fi

WEB="$(networksetup -getwebproxy "$SVC")"
if echo "$WEB" | grep -qi "Enabled: Yes" \
  && echo "$WEB" | grep -q "127.0.0.1" \
  && echo "$WEB" | grep -q "$PORT"; then
  ok "proxy web système réglé sur 127.0.0.1:$PORT"
else
  ko "proxy web système non réglé"; echo "$WEB"
fi

SEC="$(networksetup -getsecurewebproxy "$SVC")"
if echo "$SEC" | grep -qi "Enabled: Yes" && echo "$SEC" | grep -q "$PORT"; then
  ok "proxy HTTPS système réglé sur 127.0.0.1:$PORT"
else
  ko "proxy HTTPS système non réglé"; echo "$SEC"
fi

if [ -f brocolis-firefox.mobileconfig ]; then
  ok "profil .mobileconfig généré"
else
  ko "profil .mobileconfig manquant"
fi

if [ -d /Applications/Firefox.app ]; then
  if [ "$FF_PRE" -eq 1 ]; then
    echo "  INFO: une politique Firefox préexistait — non écrasée (vérif ignorée)"
  elif [ -f "$FF_POLICY" ] && grep -q "ImportEnterpriseRoots" "$FF_POLICY"; then
    ok "politique Firefox déposée"
  else
    ko "politique Firefox manquante"
  fi
else
  echo "  INFO: Firefox non installé — étape Firefox ignorée"
fi

# Flux HTTPS réel via le proxy : CONNECT + certificat forgé + upstream.
HCODE="$(curl -s -o ex.html -w '%{http_code}' \
  -x "http://127.0.0.1:$PORT" --cacert brocolis-ca.crt \
  https://example.com/ 2>/dev/null || true)"
if [ "$HCODE" = "200" ] && grep -qi "Example Domain" ex.html; then
  ok "MITM HTTPS réel (example.com → 200)"
else
  ko "MITM HTTPS réel échoué (code=$HCODE)"
fi

# Bonus informatif : confiance « transparente » (dépend du backend TLS de curl).
TCODE="$(curl -s -o /dev/null -w '%{http_code}' \
  -x "http://127.0.0.1:$PORT" https://example.com/ 2>/dev/null || true)"
if [ "$TCODE" = "200" ]; then
  echo "  INFO: curl fait confiance au CA du trousseau (transparent) — OK"
else
  echo "  INFO: curl n'utilise pas le trousseau système (selon son backend TLS) — non bloquant"
fi

if [ "$HAVE_PY" -eq 1 ]; then
  OUT="$(curl -s -x "http://127.0.0.1:$PORT" "http://127.0.0.1:$ECHO_PORT/" \
    --data-binary 'a ((secret)) b' -H 'Content-Type: text/plain' 2>/dev/null || true)"
  ORIG="$(tail -1 echo.log 2>/dev/null || true)"
  if [ "$OUT" = "a secret b" ] && echo "$ORIG" | grep -q "BROCOLIS:v1:"; then
    ok "chiffrement sélectif round-trip (origine ne voit que le chiffré)"
  else
    ko "chiffrement sélectif round-trip"
    echo "  reçu='$OUT' origine='$ORIG'"
  fi
else
  echo "  INFO: python3 absent — round-trip de chiffrement ignoré"
fi

echo "== Arrêt et vérification de la restauration =="
kill -INT "$BROCOLIS_PID" 2>/dev/null
for _ in $(seq 1 60); do
  kill -0 "$BROCOLIS_PID" 2>/dev/null || break
  sleep 0.2
done
if kill -0 "$BROCOLIS_PID" 2>/dev/null; then
  ko "le processus ne s'est pas arrêté proprement"
  kill -KILL "$BROCOLIS_PID" 2>/dev/null
fi
BROCOLIS_PID=""

if [ "$CA_PRE" -eq 0 ]; then
  if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
    ko "CA non retiré du trousseau"
  else
    ok "CA retiré du trousseau (état initial restauré)"
  fi
else
  if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
    ok "CA préexistant conservé (comportement correct)"
  else
    ko "CA préexistant supprimé à tort"
  fi
fi

POST_WEB="$(networksetup -getwebproxy "$SVC")"
POST_SEC="$(networksetup -getsecurewebproxy "$SVC")"
if [ "$POST_WEB" = "$PRE_WEB" ]; then
  ok "proxy web restauré à l'identique"
else
  ko "proxy web non restauré"
  echo "  avant : $PRE_WEB"
  echo "  après : $POST_WEB"
fi
if [ "$POST_SEC" = "$PRE_SEC" ]; then
  ok "proxy HTTPS restauré à l'identique"
else
  ko "proxy HTTPS non restauré"
  echo "  avant : $PRE_SEC"
  echo "  après : $POST_SEC"
fi

if [ ! -f brocolis-firefox.mobileconfig ]; then
  ok "profil .mobileconfig retiré"
else
  ko "profil .mobileconfig non retiré"
fi

if [ -d /Applications/Firefox.app ] && [ "$FF_PRE" -eq 0 ]; then
  if [ ! -f "$FF_POLICY" ]; then
    ok "politique Firefox retirée"
  else
    ko "politique Firefox non retirée"
  fi
fi

echo
echo "=================================================="
echo "  Résultat : $PASS réussi(s), $FAIL échoué(s)"
echo "=================================================="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
