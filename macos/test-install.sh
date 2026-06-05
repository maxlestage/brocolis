#!/usr/bin/env bash
# Test d'intégration macOS pour le chemin install.sh / uninstall.sh.
#
# Couvre, sur une vraie machine macOS :
#   - install.sh pose CA dans le trousseau Système ;
#   - politique entreprise Firefox pour chaque variante détectée ;
#   - profil .mobileconfig généré ;
#   - brocolis.toml généré ;
#   - vrai flux HTTPS via le proxy (CONNECT + cert forgé) ;
#   - chiffrement sélectif round-trip ;
#   - uninstall.sh restaure tout (CA, politiques, .mobileconfig).
#
# Complémentaire de scripts/test-macos.sh, qui teste le chemin natif
# `brocolis auto`.
#
# Usage (depuis la racine du dépôt) :
#   bash macos/test-install.sh
#
# Le script compile la CLI puis demande le mot de passe administrateur
# (une fois) pour la partie système. Il restaure l'état initial en sortant.
set -u

SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
MACOS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$MACOS_DIR/.." && pwd)"
cd "$ROOT_DIR"

if [ "$(uname)" != "Darwin" ]; then
  echo "Ce script doit être exécuté sur macOS." >&2
  exit 2
fi

# Phase 1 (utilisateur) : compilation, puis élévation.
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
INSTALL="$MACOS_DIR/install.sh"
UNINSTALL="$MACOS_DIR/uninstall.sh"
if [ ! -x "$BIN" ]; then
  echo "Binaire absent : $BIN" >&2
  echo "Lancez ce script SANS sudo (il compilera puis s'élèvera seul)." >&2
  exit 2
fi
if [ ! -x "$INSTALL" ] || [ ! -x "$UNINSTALL" ]; then
  chmod +x "$INSTALL" "$UNINSTALL" 2>/dev/null
fi

PORT=8896
ECHO_PORT=8895
KEYCHAIN="/Library/Keychains/System.keychain"
CA_CN="Brocolis Root CA"
CONFIG_FILE="/etc/brocolis/brocolis.toml"
CA_CRT="/etc/brocolis/brocolis-ca.crt"
MOBILECONFIG="/etc/brocolis/brocolis-firefox.mobileconfig"
STATE_FILE="/var/lib/brocolis/state.env"
FIREFOX_APPS=(
  "/Applications/Firefox.app"
  "/Applications/Firefox Developer Edition.app"
  "/Applications/Firefox Nightly.app"
  "/Applications/Firefox ESR.app"
)

PASS=0
FAIL=0
ok() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
ko() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

BROCOLIS_PID=""
ECHO_PID=""
WORK="$(mktemp -d /tmp/brocolis-macos-install-test.XXXXXX)"

# État initial à restaurer/vérifier.
if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
  CA_PRE=1
else
  CA_PRE=0
fi
declare -A FF_PRE
for app in "${FIREFOX_APPS[@]}"; do
  if [ -f "$app/Contents/Resources/distribution/policies.json" ]; then
    FF_PRE["$app"]=1
  else
    FF_PRE["$app"]=0
  fi
done
[ -f "$MOBILECONFIG" ] && MC_PRE=1 || MC_PRE=0
[ -f "$CONFIG_FILE" ]  && CFG_PRE=1 || CFG_PRE=0

cleanup() {
  if [ -n "$BROCOLIS_PID" ] && kill -0 "$BROCOLIS_PID" 2>/dev/null; then
    kill -INT "$BROCOLIS_PID" 2>/dev/null
    for _ in $(seq 1 50); do
      kill -0 "$BROCOLIS_PID" 2>/dev/null || break
      sleep 0.2
    done
    kill -KILL "$BROCOLIS_PID" 2>/dev/null || true
  fi
  [ -n "$ECHO_PID" ] && kill "$ECHO_PID" 2>/dev/null || true
  # Garantit la restauration même si on a planté entre install et uninstall.
  if [ -f "$STATE_FILE" ]; then "$UNINSTALL" >/dev/null 2>&1 || true; fi
  rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

cd "$WORK"

# Origine d'écho locale, si python3 dispo.
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

echo "==> Exécution de install.sh…"
BROCOLIS_PORT=$PORT "$INSTALL" >install.log 2>&1
INSTALL_RC=$?
if [ "$INSTALL_RC" -ne 0 ]; then
  ko "install.sh a échoué (rc=$INSTALL_RC)"
  cat install.log
  exit 1
fi

echo "== Pendant l'exécution =="

if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
  ok "CA présent dans le trousseau Système"
else
  ko "CA absent du trousseau Système"
fi

# Vérif des politiques Firefox déposées dans les variantes détectées.
FF_INSTALLED=0
for app in "${FIREFOX_APPS[@]}"; do
  [ -d "$app" ] || continue
  policy="$app/Contents/Resources/distribution/policies.json"
  if [ -f "$policy" ] && grep -q "ImportEnterpriseRoots" "$policy"; then
    ok "politique Firefox déposée ($policy)"
    FF_INSTALLED=$((FF_INSTALLED + 1))
  elif [ "${FF_PRE[$app]:-0}" -eq 1 ]; then
    echo "  INFO: politique préexistait dans $app — non écrasée"
  else
    ko "politique Firefox manquante dans $app"
  fi
done
if [ "$FF_INSTALLED" -eq 0 ]; then
  echo "  INFO: aucune politique Firefox posée (aucun Firefox détecté ou tout préexistait)"
fi

if [ -f "$MOBILECONFIG" ] && grep -q "ImportEnterpriseRoots" "$MOBILECONFIG"; then
  ok "profil .mobileconfig généré"
else
  ko ".mobileconfig manquant ou invalide"
fi

if [ -f "$CONFIG_FILE" ]; then
  ok "brocolis.toml présent ($CONFIG_FILE)"
else
  ko "brocolis.toml absent"
fi

echo "==> Démarrage de « brocolis run --config $CONFIG_FILE »…"
RUST_LOG=info "$BIN" run --config "$CONFIG_FILE" >auto.log 2>&1 &
BROCOLIS_PID=$!
for _ in $(seq 1 60); do
  grep -q "écoute" auto.log 2>/dev/null && break
  kill -0 "$BROCOLIS_PID" 2>/dev/null || break
  sleep 0.2
done
if grep -q "écoute" auto.log; then
  ok "proxy démarré"
else
  ko "proxy non démarré"
  echo "---- auto.log ----"; cat auto.log
  exit 1
fi

# Flux HTTPS réel via le proxy.
HCODE="$(curl -s -o ex.html -w '%{http_code}' \
  -x "http://127.0.0.1:$PORT" --cacert "$CA_CRT" \
  https://example.com/ 2>/dev/null || true)"
if [ "$HCODE" = "200" ] && grep -qi "Example Domain" ex.html; then
  ok "MITM HTTPS réel (example.com → 200)"
else
  ko "MITM HTTPS réel échoué (code=$HCODE)"
fi

# Confiance transparente via trousseau (selon backend TLS de curl).
TCODE="$(curl -s -o /dev/null -w '%{http_code}' \
  -x "http://127.0.0.1:$PORT" https://example.com/ 2>/dev/null || true)"
if [ "$TCODE" = "200" ]; then
  ok "curl fait confiance au CA via le trousseau (transparent)"
else
  echo "  INFO: curl n'utilise pas le trousseau (backend TLS) — non bloquant (code=$TCODE)"
fi

if [ "$HAVE_PY" -eq 1 ]; then
  OUT="$(curl -s -x "http://127.0.0.1:$PORT" "http://127.0.0.1:$ECHO_PORT/" \
    --data-binary 'a ((secret)) b' -H 'Content-Type: text/plain' 2>/dev/null || true)"
  ORIG="$(tail -1 echo.log 2>/dev/null || true)"
  if [ "$OUT" = "a secret b" ] && echo "$ORIG" | grep -q "BROCOLIS:v1:"; then
    ok "chiffrement sélectif round-trip"
  else
    ko "chiffrement sélectif round-trip"
    echo "  reçu='$OUT' origine='$ORIG'"
  fi
fi

echo "== Arrêt du proxy puis désinstallation =="
kill -INT "$BROCOLIS_PID" 2>/dev/null
for _ in $(seq 1 60); do
  kill -0 "$BROCOLIS_PID" 2>/dev/null || break
  sleep 0.2
done
if kill -0 "$BROCOLIS_PID" 2>/dev/null; then
  ko "le proxy ne s'est pas arrêté proprement"
  kill -KILL "$BROCOLIS_PID" 2>/dev/null
fi
BROCOLIS_PID=""

"$UNINSTALL" >uninstall.log 2>&1
UNINSTALL_RC=$?
if [ "$UNINSTALL_RC" -ne 0 ]; then
  ko "uninstall.sh a échoué (rc=$UNINSTALL_RC)"
  cat uninstall.log
fi

# Vérif de la restauration.
if [ "$CA_PRE" -eq 0 ]; then
  if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
    ko "CA non retiré du trousseau"
  else
    ok "CA retiré du trousseau (état initial restauré)"
  fi
else
  if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
    ok "CA préexistant conservé"
  else
    ko "CA préexistant supprimé à tort"
  fi
fi

for app in "${FIREFOX_APPS[@]}"; do
  [ -d "$app" ] || continue
  policy="$app/Contents/Resources/distribution/policies.json"
  if [ "${FF_PRE[$app]:-0}" -eq 0 ]; then
    if [ ! -f "$policy" ]; then
      ok "politique Firefox retirée ($app)"
    else
      ko "politique Firefox non retirée ($app)"
    fi
  else
    if [ -f "$policy" ]; then
      ok "politique Firefox préexistante conservée ($app)"
    else
      ko "politique Firefox préexistante supprimée à tort ($app)"
    fi
  fi
done

if [ "$MC_PRE" -eq 0 ]; then
  if [ ! -f "$MOBILECONFIG" ]; then
    ok ".mobileconfig retiré"
  else
    ko ".mobileconfig non retiré"
  fi
fi

if [ "$CFG_PRE" -eq 0 ]; then
  if [ ! -f "$CONFIG_FILE" ]; then
    ok "brocolis.toml retiré"
  else
    ko "brocolis.toml non retiré"
  fi
fi

if [ -f "$STATE_FILE" ]; then
  ko "state.env non nettoyé"
else
  ok "state.env nettoyé"
fi

echo
echo "=================================================="
echo "  Résultat : $PASS réussi(s), $FAIL échoué(s)"
echo "=================================================="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
