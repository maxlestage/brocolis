#!/usr/bin/env bash
# Test d'intégration Linux pour les scripts install.sh / uninstall.sh.
#
# Vérifie, sur une vraie machine Linux :
#   - l'ajout du CA au magasin de confiance système, puis son retrait ;
#   - la pose de la politique entreprise Firefox, puis son retrait ;
#   - la génération du brocolis.toml ;
#   - un vrai flux HTTPS au travers du proxy (MITM + certificat forgé) ;
#   - le round-trip du chiffrement sélectif.
#
# Usage (depuis la racine du dépôt) :
#   bash linux/test-linux.sh
#
# Compile la CLI, élève les privilèges (sudo) pour la partie système, exécute
# install.sh, lance le proxy, vérifie, puis uninstall.sh restaure l'état.
set -u

SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
LINUX_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$LINUX_DIR/.." && pwd)"
cd "$ROOT_DIR"

if [ "$(uname)" != "Linux" ]; then
  echo "Ce script doit être exécuté sur Linux." >&2
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
INSTALL="$LINUX_DIR/install.sh"
UNINSTALL="$LINUX_DIR/uninstall.sh"
if [ ! -x "$BIN" ]; then
  echo "Binaire absent : $BIN" >&2
  echo "Lancez ce script SANS sudo (il compilera puis s'élèvera seul)." >&2
  exit 2
fi
if [ ! -x "$INSTALL" ] || [ ! -x "$UNINSTALL" ]; then
  chmod +x "$INSTALL" "$UNINSTALL" 2>/dev/null
fi

PORT=8888
ECHO_PORT=8899
FF_POLICY="/etc/firefox/policies/policies.json"
CONFIG_FILE="/etc/brocolis/brocolis.toml"
STATE_FILE="/var/lib/brocolis/state.env"

PASS=0
FAIL=0
ok() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
ko() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

BROCOLIS_PID=""
ECHO_PID=""
WORK="$(mktemp -d /tmp/brocolis-linux-test.XXXXXX)"

# Détecte le magasin de confiance pour vérifier la présence du CA.
TRUST_FILE=""
if [ -d /usr/local/share/ca-certificates ]; then
  TRUST_FILE="/usr/local/share/ca-certificates/brocolis-ca.crt"
elif [ -d /etc/pki/ca-trust/source/anchors ]; then
  TRUST_FILE="/etc/pki/ca-trust/source/anchors/brocolis-ca.crt"
elif [ -d /etc/ca-certificates/trust-source/anchors ]; then
  TRUST_FILE="/etc/ca-certificates/trust-source/anchors/brocolis-ca.crt"
else
  echo "Magasin de confiance non détecté — test non applicable ici." >&2
  exit 2
fi

# État initial à restaurer/vérifier.
if [ -f "$TRUST_FILE" ]; then CA_PRE=1; else CA_PRE=0; fi
if [ -f "$FF_POLICY" ]; then FF_PRE=1; else FF_PRE=0; fi
if [ -f "$CONFIG_FILE" ]; then CFG_PRE=1; else CFG_PRE=0; fi

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

# Origine d'écho locale, si python3 dispo (pour le round-trip de chiffrement).
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

if [ -f "$TRUST_FILE" ]; then
  ok "CA présent dans le magasin de confiance ($TRUST_FILE)"
else
  ko "CA absent du magasin de confiance"
fi

if [ -f "$FF_POLICY" ] && grep -q "ImportEnterpriseRoots" "$FF_POLICY"; then
  ok "politique Firefox déposée ($FF_POLICY)"
else
  if [ "$FF_PRE" -eq 1 ]; then
    echo "  INFO: politique Firefox préexistait — install.sh n'a pas écrasé"
  else
    ko "politique Firefox manquante"
  fi
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
if grep -q "écoute" auto.log 2>/dev/null; then
  ok "proxy démarré"
else
  ko "proxy non démarré"
  echo "---- auto.log ----"; cat auto.log
  exit 1
fi

# Flux HTTPS réel via le proxy : CONNECT + certificat forgé + upstream.
HCODE="$(curl -s -o ex.html -w '%{http_code}' \
  -x "http://127.0.0.1:$PORT" --cacert /etc/brocolis/brocolis-ca.crt \
  https://example.com/ 2>/dev/null || true)"
if [ "$HCODE" = "200" ] && grep -qi "Example Domain" ex.html; then
  ok "MITM HTTPS réel (example.com → 200)"
else
  ko "MITM HTTPS réel échoué (code=$HCODE)"
fi

# Bonus : confiance « transparente » via le magasin système (selon le backend
# TLS de curl — OpenSSL/GnuTLS lisent /etc/ssl/certs ; NSS pas toujours).
TCODE="$(curl -s -o /dev/null -w '%{http_code}' \
  -x "http://127.0.0.1:$PORT" https://example.com/ 2>/dev/null || true)"
if [ "$TCODE" = "200" ]; then
  echo "  INFO: curl fait confiance au CA via le magasin système — OK"
else
  echo "  INFO: curl n'utilise pas le magasin système (backend TLS) — non bloquant (code=$TCODE)"
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

# Vérification de la restauration.
if [ "$CA_PRE" -eq 0 ]; then
  if [ -f "$TRUST_FILE" ]; then
    ko "CA non retiré du magasin de confiance"
  else
    ok "CA retiré du magasin (état initial restauré)"
  fi
else
  if [ -f "$TRUST_FILE" ]; then
    ok "CA préexistant conservé (comportement correct)"
  else
    ko "CA préexistant supprimé à tort"
  fi
fi

if [ "$FF_PRE" -eq 0 ]; then
  if [ -f "$FF_POLICY" ]; then
    ko "politique Firefox non retirée"
  else
    ok "politique Firefox retirée"
  fi
else
  if [ -f "$FF_POLICY" ]; then
    ok "politique Firefox préexistante conservée"
  else
    ko "politique Firefox préexistante supprimée à tort"
  fi
fi

if [ "$CFG_PRE" -eq 0 ]; then
  if [ -f "$CONFIG_FILE" ]; then
    ko "brocolis.toml non retiré"
  else
    ok "brocolis.toml retiré"
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
