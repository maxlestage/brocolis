#!/usr/bin/env bash
# Brocolis — installation système sur Linux.
#
# Équivalent de `brocolis auto` (macOS) pour la partie setup :
#   1. génère (ou recharge) le CA dans /etc/brocolis/ ;
#   2. l'ajoute au magasin de confiance système (Debian/Ubuntu, Fedora/RHEL,
#      Arch détectés via leurs outils respectifs) ;
#   3. dépose une politique entreprise Firefox dans /etc/firefox/policies/ pour
#      que Firefox accepte le CA (ne touche pas au bundle Firefox) ;
#   4. génère un brocolis.toml prêt à l'emploi avec une clé fraîche ;
#   5. enregistre l'état modifié dans /var/lib/brocolis/state.env pour que
#      uninstall.sh restaure proprement.
#
# Le proxy système (variables http_proxy/https_proxy, GSettings, paramètres
# navigateur) reste à configurer côté utilisateur — la diversité des DE et des
# packagings Linux rend l'automatisation universelle illusoire. Une fois
# l'install effectuée, configurez votre navigateur sur 127.0.0.1:8080 (HTTP +
# HTTPS) et lancez `brocolis run --config /etc/brocolis/brocolis.toml`.
#
# Usage :
#   sudo linux/install.sh
set -u

ETC_DIR="/etc/brocolis"
STATE_DIR="/var/lib/brocolis"
STATE_FILE="$STATE_DIR/state.env"
CA_CRT="$ETC_DIR/brocolis-ca.crt"
CA_KEY="$ETC_DIR/brocolis-ca.key"
CONFIG_FILE="$ETC_DIR/brocolis.toml"
FF_POLICY_DIR="/etc/firefox/policies"
FF_POLICY="$FF_POLICY_DIR/policies.json"
LISTEN_PORT="${BROCOLIS_PORT:-8080}"

if [ "$(uname)" != "Linux" ]; then
  echo "Ce script doit être exécuté sur Linux (uname=$(uname))." >&2
  exit 2
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "==> Élévation des privilèges (sudo) pour l'installation système…"
  exec sudo --preserve-env=BROCOLIS_PORT "$0" "$@"
fi

# 1. Localise le binaire brocolis : PATH d'abord, sinon target/release relatif.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
if command -v brocolis >/dev/null 2>&1; then
  BIN="$(command -v brocolis)"
elif [ -x "$ROOT_DIR/target/release/brocolis" ]; then
  BIN="$ROOT_DIR/target/release/brocolis"
else
  echo "brocolis introuvable (ni dans PATH, ni dans $ROOT_DIR/target/release/)." >&2
  echo "Compilez d'abord : cargo build --release --bin brocolis" >&2
  exit 2
fi
echo "Binaire utilisé : $BIN"

# 2. Détecte le magasin de confiance système. Trois familles supportées :
#    - Debian / Ubuntu et dérivés (update-ca-certificates)
#    - Fedora / RHEL et dérivés  (update-ca-trust)
#    - Arch et dérivés            (trust extract-compat)
DISTRO_FAMILY=""
TRUST_DIR=""
TRUST_FILE=""
TRUST_UPDATE=""
if [ -d /usr/local/share/ca-certificates ] && command -v update-ca-certificates >/dev/null 2>&1; then
  DISTRO_FAMILY="debian"
  TRUST_DIR="/usr/local/share/ca-certificates"
  TRUST_FILE="$TRUST_DIR/brocolis-ca.crt"
  TRUST_UPDATE="update-ca-certificates"
elif [ -d /etc/pki/ca-trust/source/anchors ] && command -v update-ca-trust >/dev/null 2>&1; then
  DISTRO_FAMILY="rhel"
  TRUST_DIR="/etc/pki/ca-trust/source/anchors"
  TRUST_FILE="$TRUST_DIR/brocolis-ca.crt"
  TRUST_UPDATE="update-ca-trust"
elif [ -d /etc/ca-certificates/trust-source/anchors ] && command -v trust >/dev/null 2>&1; then
  DISTRO_FAMILY="arch"
  TRUST_DIR="/etc/ca-certificates/trust-source/anchors"
  TRUST_FILE="$TRUST_DIR/brocolis-ca.crt"
  TRUST_UPDATE="trust extract-compat"
else
  echo "Magasin de confiance non détecté (essayé Debian/Ubuntu, Fedora/RHEL, Arch)." >&2
  echo "Installez 'ca-certificates' ou équivalent et relancez." >&2
  exit 2
fi
echo "Distribution détectée : $DISTRO_FAMILY"

# 3. Crée les dossiers de travail.
mkdir -p "$ETC_DIR" "$STATE_DIR"
chmod 755 "$ETC_DIR" "$STATE_DIR"

# 4. Génère (ou recharge) le CA dans $ETC_DIR.
if [ ! -f "$CA_CRT" ] || [ ! -f "$CA_KEY" ]; then
  echo "==> Génération du CA dans $ETC_DIR…"
  "$BIN" ca --ca-cert "$CA_CRT" --ca-key "$CA_KEY" >/dev/null
  chmod 600 "$CA_KEY"
else
  echo "CA déjà présent dans $ETC_DIR : conservé."
fi

# 5. Ajoute le CA au magasin de confiance s'il n'y est pas déjà.
INSTALLED_CA=0
if [ -f "$TRUST_FILE" ]; then
  echo "CA déjà présent dans $TRUST_FILE : conservé à l'arrêt."
else
  cp "$CA_CRT" "$TRUST_FILE"
  chmod 644 "$TRUST_FILE"
  if ! eval "$TRUST_UPDATE" >/dev/null; then
    echo "Mise à jour du magasin de confiance échouée." >&2
    rm -f "$TRUST_FILE"
    exit 1
  fi
  INSTALLED_CA=1
  echo "==> CA ajouté au magasin de confiance ($TRUST_FILE)"
fi

# 6. Politique entreprise Firefox (ne touche pas au bundle).
INSTALLED_FF_POLICY=0
if [ -f "$FF_POLICY" ]; then
  echo "Politique Firefox déjà présente ($FF_POLICY) : laissée intacte."
  echo "  Activez-y \"Certificates.ImportEnterpriseRoots\": true si besoin."
else
  mkdir -p "$FF_POLICY_DIR"
  cat > "$FF_POLICY" <<'JSON'
{
  "policies": {
    "Certificates": {
      "ImportEnterpriseRoots": true
    }
  }
}
JSON
  chmod 644 "$FF_POLICY"
  INSTALLED_FF_POLICY=1
  echo "==> Politique Firefox déposée ($FF_POLICY)"
fi

# 7. Génère un brocolis.toml par défaut s'il n'existe pas.
INSTALLED_CONFIG=0
if [ ! -f "$CONFIG_FILE" ]; then
  KEY="$("$BIN" keygen | head -1)"
  cat > "$CONFIG_FILE" <<EOF
listen = "127.0.0.1:$LISTEN_PORT"
ca_cert = "$CA_CRT"
ca_key = "$CA_KEY"
key = "$KEY"
encrypt_hosts = []
EOF
  chmod 600 "$CONFIG_FILE"
  INSTALLED_CONFIG=1
  echo "==> Config générée : $CONFIG_FILE (port $LISTEN_PORT)"
else
  echo "Config déjà présente : $CONFIG_FILE — non écrasée."
fi

# 8. Persiste l'état pour uninstall.sh.
{
  echo "DISTRO_FAMILY='$DISTRO_FAMILY'"
  echo "INSTALLED_CA=$INSTALLED_CA"
  echo "TRUST_FILE='$TRUST_FILE'"
  echo "TRUST_UPDATE='$TRUST_UPDATE'"
  echo "INSTALLED_FF_POLICY=$INSTALLED_FF_POLICY"
  echo "FF_POLICY='$FF_POLICY'"
  echo "INSTALLED_CONFIG=$INSTALLED_CONFIG"
  echo "CONFIG_FILE='$CONFIG_FILE'"
  echo "ETC_DIR='$ETC_DIR'"
} > "$STATE_FILE"
chmod 600 "$STATE_FILE"

cat <<EOF

✓ Installation terminée.

Démarrer le proxy :
    brocolis run --config $CONFIG_FILE

Configurez votre navigateur sur le proxy HTTP **et** HTTPS 127.0.0.1:$LISTEN_PORT
(Firefox suivra la politique entreprise automatiquement ; pour Chromium, voir
les options --proxy-server ou les paramètres système).

Désinstaller (CA retiré du magasin, politique Firefox retirée, état nettoyé) :
    sudo $SCRIPT_DIR/uninstall.sh
EOF
