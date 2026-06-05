#!/usr/bin/env bash
# Test d'intégration ciblé sur `brocolis auto` (chemin natif Linux).
#
# Couvre, sur une vraie machine Debian/Ubuntu, Fedora/RHEL ou Arch :
#   - le démarrage de auto et la pose des artefacts
#     (CA dans le magasin, politique Firefox, /etc/profile.d/...) ;
#   - la consolidation par l'outil distro
#     (update-ca-certificates / update-ca-trust / trust extract-compat) ;
#   - un vrai flux HTTPS via le proxy (CONNECT + cert forgé) ;
#   - la confiance transparente du CA via le magasin système ;
#   - la restauration à l'arrêt (Ctrl-C / SIGTERM).
#
# Complémentaire de linux/test-linux.sh, qui teste le chemin
# install.sh + brocolis run.
#
# Usage (depuis la racine du dépôt) :
#   bash linux/test-auto.sh
#
# Le script compile la CLI puis demande le mot de passe administrateur
# (une fois) pour la partie système. Il restaure l'état initial en sortant.
set -u

SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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
if [ ! -x "$BIN" ]; then
  echo "Binaire absent : $BIN" >&2
  echo "Lancez ce script SANS sudo (il compilera puis s'élèvera seul)." >&2
  exit 2
fi

PORT=8898
FF_POLICY="/etc/firefox/policies/policies.json"
FF_POLICY_DIR="/etc/firefox/policies"
ENV_PROXY="/etc/profile.d/brocolis-proxy.sh"

# Détection du magasin (les trois mêmes branches que src/system/linux.rs).
if   [ -d /usr/local/share/ca-certificates ];        then TRUST_FILE="/usr/local/share/ca-certificates/brocolis-ca.crt"
elif [ -d /etc/pki/ca-trust/source/anchors ];        then TRUST_FILE="/etc/pki/ca-trust/source/anchors/brocolis-ca.crt"
elif [ -d /etc/ca-certificates/trust-source/anchors ];then TRUST_FILE="/etc/ca-certificates/trust-source/anchors/brocolis-ca.crt"
else echo "Magasin de confiance non détecté." >&2; exit 2; fi

PASS=0; FAIL=0
ok() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
ko() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

BPID=""
ECHO_PID=""
WORK="$(mktemp -d /tmp/brocolis-auto-test.XXXXXX)"

# État initial à restaurer/vérifier.
[ -f "$TRUST_FILE" ] && CA_PRE=1 || CA_PRE=0
[ -f "$FF_POLICY" ]  && FF_PRE=1 || FF_PRE=0
[ -f "$ENV_PROXY" ]  && EP_PRE=1 || EP_PRE=0

cleanup() {
  if [ -n "$BPID" ] && kill -0 "$BPID" 2>/dev/null; then
    kill -INT "$BPID" 2>/dev/null
    for _ in $(seq 1 50); do
      kill -0 "$BPID" 2>/dev/null || break
      sleep 0.2
    done
    kill -KILL "$BPID" 2>/dev/null || true
  fi
  [ -n "$ECHO_PID" ] && kill "$ECHO_PID" 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

cd "$WORK"

KEY="$("$BIN" keygen | head -1)"
"$BIN" ca --ca-cert brocolis-ca.crt --ca-key brocolis-ca.key >/dev/null
cat > brocolis.toml <<EOF
listen = "127.0.0.1:$PORT"
ca_cert = "$WORK/brocolis-ca.crt"
ca_key  = "$WORK/brocolis-ca.key"
key = "$KEY"
encrypt_hosts = []
EOF

# Origine d'écho locale (pour le round-trip de chiffrement), si python3 dispo.
ECHO_PORT=8897
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

echo "==> Démarrage de « brocolis auto --config $WORK/brocolis.toml »…"
RUST_LOG=info "$BIN" auto --config brocolis.toml >auto.log 2>&1 &
BPID=$!

for _ in $(seq 1 80); do
  grep -q "écoute" auto.log 2>/dev/null && break
  kill -0 "$BPID" 2>/dev/null || break
  sleep 0.1
done
if grep -q "écoute" auto.log; then
  ok "proxy démarré"
else
  ko "proxy non démarré"
  echo "---- auto.log ----"; cat auto.log
  exit 1
fi

echo "== Pendant l'exécution =="

[ -f "$TRUST_FILE" ] \
  && ok "CA dans le magasin ($TRUST_FILE)" \
  || ko "CA absent du magasin"
grep -q "BEGIN CERTIFICATE" "$TRUST_FILE" 2>/dev/null \
  && ok "CA est un PEM valide" \
  || ko "CA n'est pas un PEM"

if [ -f "$FF_POLICY" ] && grep -q "ImportEnterpriseRoots" "$FF_POLICY"; then
  ok "politique Firefox déposée"
else
  if [ "$FF_PRE" -eq 1 ]; then
    echo "  INFO: politique Firefox préexistante — non écrasée"
  else
    ko "politique Firefox manquante/invalide"
  fi
fi

if [ -f "$ENV_PROXY" ] && grep -q "127.0.0.1:$PORT" "$ENV_PROXY"; then
  ok "env-proxy posé sur le bon port"
else
  if [ "$EP_PRE" -eq 1 ]; then
    echo "  INFO: env-proxy préexistant — non écrasé"
  else
    ko "env-proxy manquant ou mauvais port"
  fi
fi

# Le bundle consolidé contient-il bien Brocolis CA ? `grep` direct ne marche
# pas (PEM = base64), il faut décoder via openssl pour atteindre le CN.
BUNDLE=""
for cand in /etc/ssl/certs/ca-certificates.crt /etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem /etc/ssl/cert.pem; do
  [ -f "$cand" ] && { BUNDLE="$cand"; break; }
done
ca_in_bundle() {
  [ -n "$BUNDLE" ] || return 1
  awk -v RS='-----BEGIN CERTIFICATE-----' 'NR>1 {print "-----BEGIN CERTIFICATE-----"$0}' "$BUNDLE" \
    | openssl crl2pkcs7 -nocrl -certfile /dev/stdin 2>/dev/null \
    | openssl pkcs7 -print_certs -noout 2>/dev/null \
    | grep -q "Brocolis Root CA"
}
if [ -n "$BUNDLE" ]; then
  if ca_in_bundle; then
    ok "CA présent dans le bundle système consolidé ($BUNDLE)"
  else
    ko "CA pas consolidé dans le bundle système ($BUNDLE)"
  fi
else
  echo "  INFO: aucun bundle système connu trouvé — vérif consolidation ignorée"
fi

# Flux HTTPS via le proxy avec --cacert explicite (test du MITM).
HCODE="$(curl -s -o ex.html -w '%{http_code}' \
  -x "http://127.0.0.1:$PORT" --cacert "$WORK/brocolis-ca.crt" \
  https://example.com/ 2>/dev/null || true)"
if [ "$HCODE" = "200" ] && grep -qi "Example Domain" ex.html; then
  ok "MITM HTTPS réel (example.com → 200)"
else
  ko "MITM HTTPS réel échoué (code=$HCODE)"
fi

# Confiance transparente via le magasin système (selon le backend TLS de curl).
TCODE="$(curl -s -o /dev/null -w '%{http_code}' \
  -x "http://127.0.0.1:$PORT" https://example.com/ 2>/dev/null || true)"
if [ "$TCODE" = "200" ]; then
  ok "curl fait confiance au CA via le magasin système (transparent)"
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

echo "== Arrêt et vérification de la restauration =="
kill -INT "$BPID" 2>/dev/null
for _ in $(seq 1 60); do
  kill -0 "$BPID" 2>/dev/null || break
  sleep 0.2
done
if kill -0 "$BPID" 2>/dev/null; then
  ko "le processus ne s'est pas arrêté proprement"
  kill -KILL "$BPID" 2>/dev/null
fi
BPID=""

if [ "$CA_PRE" -eq 0 ]; then
  [ ! -f "$TRUST_FILE" ] \
    && ok "CA retiré du magasin (état initial restauré)" \
    || ko "CA non retiré du magasin"
else
  [ -f "$TRUST_FILE" ] \
    && ok "CA préexistant conservé (comportement correct)" \
    || ko "CA préexistant supprimé à tort"
fi

if [ "$FF_PRE" -eq 0 ]; then
  [ ! -f "$FF_POLICY" ] \
    && ok "politique Firefox retirée" \
    || ko "politique Firefox non retirée"
  [ ! -d "$FF_POLICY_DIR" ] \
    && ok "dossier /etc/firefox/policies retiré (vide)" \
    || echo "  INFO: $FF_POLICY_DIR subsiste (probablement non vide)"
else
  [ -f "$FF_POLICY" ] \
    && ok "politique Firefox préexistante conservée" \
    || ko "politique Firefox préexistante supprimée à tort"
fi

if [ "$EP_PRE" -eq 0 ]; then
  [ ! -f "$ENV_PROXY" ] \
    && ok "env-proxy retiré" \
    || ko "env-proxy non retiré"
else
  [ -f "$ENV_PROXY" ] \
    && ok "env-proxy préexistant conservé" \
    || ko "env-proxy préexistant supprimé à tort"
fi

# Le bundle système ne doit plus contenir Brocolis CA après revert
# (même décodage openssl que pour la vérif d'engage).
if [ -n "$BUNDLE" ] && [ "$CA_PRE" -eq 0 ]; then
  if ! ca_in_bundle; then
    ok "bundle système nettoyé"
  else
    ko "Brocolis Root CA subsiste dans le bundle système"
  fi
fi

echo
echo "=================================================="
echo "  Résultat : $PASS réussi(s), $FAIL échoué(s)"
echo "=================================================="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
