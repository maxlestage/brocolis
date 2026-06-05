#!/usr/bin/env bash
# Brocolis — installation système sur Linux.
#
# Setup automatique :
#   - CA dans /etc/brocolis/, ajouté au magasin de confiance
#     (Debian/Ubuntu, Fedora/RHEL, Arch détectés via leurs outils) ;
#   - politique entreprise Firefox dans /etc/firefox/policies/ ;
#   - brocolis.toml prêt à l'emploi avec une clé fraîche.
#
# Flags optionnels :
#   --env-proxy        Écrit /etc/profile.d/brocolis-proxy.sh
#                      (http_proxy/https_proxy pour les nouvelles sessions
#                      shell — utile pour curl/wget/git/npm, pas pour les
#                      navigateurs qui ont leurs propres réglages).
#   --enable-systemd   Crée l'utilisateur système 'brocolis', copie le
#                      binaire dans /usr/local/bin/, installe l'unité
#                      brocolis.service, l'active et la démarre.
#
# Variables d'environnement :
#   BROCOLIS_PORT      Port d'écoute (défaut 8080).
#
# Idempotent : ré-exécutable sans dommage. L'état (ce qui a été créé par
# nous) est suivi dans /var/lib/brocolis/state.env pour que uninstall.sh
# restaure proprement.
#
# Usage :
#   sudo linux/install.sh [--env-proxy] [--enable-systemd]
set -u

ETC_DIR="/etc/brocolis"
STATE_DIR="/var/lib/brocolis"
STATE_FILE="$STATE_DIR/state.env"
CA_CRT="$ETC_DIR/brocolis-ca.crt"
CA_KEY="$ETC_DIR/brocolis-ca.key"
CONFIG_FILE="$ETC_DIR/brocolis.toml"
FF_POLICY_DIR="/etc/firefox/policies"
FF_POLICY="$FF_POLICY_DIR/policies.json"
ENV_PROXY_FILE="/etc/profile.d/brocolis-proxy.sh"
SYSTEMD_UNIT="/etc/systemd/system/brocolis.service"
SYSTEMD_BIN="/usr/local/bin/brocolis"
LISTEN_PORT="${BROCOLIS_PORT:-8080}"

# Parsing des flags.
ENABLE_ENV_PROXY=0
ENABLE_SYSTEMD=0
for arg in "$@"; do
  case "$arg" in
    --env-proxy)      ENABLE_ENV_PROXY=1 ;;
    --enable-systemd) ENABLE_SYSTEMD=1 ;;
    -h|--help)
      sed -n '2,27p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Argument inconnu : $arg" >&2
      echo "Utilisez -h pour l'aide." >&2
      exit 2
      ;;
  esac
done

if [ "$(uname)" != "Linux" ]; then
  echo "Ce script doit être exécuté sur Linux (uname=$(uname))." >&2
  exit 2
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "==> Élévation des privilèges (sudo) pour l'installation système…"
  exec sudo --preserve-env=BROCOLIS_PORT "$0" "$@"
fi

# 1. Localise le binaire brocolis : PATH d'abord, sinon target/release/.
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

# 2. Détecte le magasin de confiance système.
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
chmod 755 "$ETC_DIR"
chmod 700 "$STATE_DIR"

# 4. Recharge l'état précédent si présent (idempotence) : on préserve les
#    flags « c'est nous qui l'avons créé » pour ne pas oublier ce qu'on
#    possède quand on ré-exécute install.sh.
OLD_INSTALLED_USER=0
OLD_INSTALLED_CA=0
OLD_INSTALLED_FF_POLICY=0
OLD_INSTALLED_CONFIG=0
OLD_INSTALLED_ENV_PROXY=0
OLD_INSTALLED_SYSTEMD=0
OLD_INSTALLED_BIN=0
if [ -f "$STATE_FILE" ]; then
  # shellcheck disable=SC1090
  . "$STATE_FILE"
  OLD_INSTALLED_USER="${INSTALLED_USER:-0}"
  OLD_INSTALLED_CA="${INSTALLED_CA:-0}"
  OLD_INSTALLED_FF_POLICY="${INSTALLED_FF_POLICY:-0}"
  OLD_INSTALLED_CONFIG="${INSTALLED_CONFIG:-0}"
  OLD_INSTALLED_ENV_PROXY="${INSTALLED_ENV_PROXY:-0}"
  OLD_INSTALLED_SYSTEMD="${INSTALLED_SYSTEMD:-0}"
  OLD_INSTALLED_BIN="${INSTALLED_BIN:-0}"
fi

# 5. Utilisateur système 'brocolis' (uniquement si --enable-systemd).
INSTALLED_USER=0
if [ "$ENABLE_SYSTEMD" -eq 1 ]; then
  if id brocolis >/dev/null 2>&1; then
    INSTALLED_USER=$OLD_INSTALLED_USER
    echo "Utilisateur 'brocolis' déjà présent : conservé."
  else
    useradd --system --shell /usr/sbin/nologin \
            --home-dir /var/lib/brocolis --no-create-home brocolis
    INSTALLED_USER=1
    echo "==> Utilisateur système 'brocolis' créé"
  fi
fi

# 6. Génère (ou recharge) le CA dans $ETC_DIR.
if [ ! -f "$CA_CRT" ] || [ ! -f "$CA_KEY" ]; then
  echo "==> Génération du CA dans $ETC_DIR…"
  "$BIN" ca --ca-cert "$CA_CRT" --ca-key "$CA_KEY" >/dev/null
else
  echo "CA déjà présent dans $ETC_DIR : conservé."
fi

# 7. Ajoute le CA au magasin de confiance.
INSTALLED_CA=0
if [ -f "$TRUST_FILE" ]; then
  INSTALLED_CA=$OLD_INSTALLED_CA
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

# 8. Politique entreprise Firefox.
INSTALLED_FF_POLICY=0
if [ -f "$FF_POLICY" ]; then
  INSTALLED_FF_POLICY=$OLD_INSTALLED_FF_POLICY
  echo "Politique Firefox déjà présente ($FF_POLICY) : laissée intacte."
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

# 9. Génère un brocolis.toml par défaut.
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
  INSTALLED_CONFIG=1
  echo "==> Config générée : $CONFIG_FILE (port $LISTEN_PORT)"
else
  INSTALLED_CONFIG=$OLD_INSTALLED_CONFIG
  echo "Config déjà présente : $CONFIG_FILE — non écrasée."
fi

# 10. Permissions des fichiers /etc/brocolis/.
if [ "$ENABLE_SYSTEMD" -eq 1 ]; then
  # brocolis user lit la clé CA, lit et écrit brocolis.toml (SIGHUP).
  chown root:brocolis "$CA_KEY"
  chmod 640 "$CA_KEY"
  chown brocolis:brocolis "$CONFIG_FILE"
  chmod 600 "$CONFIG_FILE"
else
  chmod 600 "$CA_KEY" "$CONFIG_FILE"
fi
chmod 644 "$CA_CRT"

# 11. Variables proxy d'env (optionnel).
INSTALLED_ENV_PROXY=0
if [ "$ENABLE_ENV_PROXY" -eq 1 ]; then
  if [ -f "$ENV_PROXY_FILE" ]; then
    INSTALLED_ENV_PROXY=$OLD_INSTALLED_ENV_PROXY
    echo "$ENV_PROXY_FILE déjà présent : laissé intact."
  else
    cat > "$ENV_PROXY_FILE" <<EOF
# Brocolis : proxy HTTP/HTTPS via 127.0.0.1:$LISTEN_PORT.
# Géré par linux/install.sh — retiré par linux/uninstall.sh.
# Effet sur toute NOUVELLE session shell (login ou ouverture de terminal).
# Pour la session courante : source $ENV_PROXY_FILE
export http_proxy="http://127.0.0.1:$LISTEN_PORT"
export https_proxy="http://127.0.0.1:$LISTEN_PORT"
export HTTP_PROXY="http://127.0.0.1:$LISTEN_PORT"
export HTTPS_PROXY="http://127.0.0.1:$LISTEN_PORT"
export no_proxy="localhost,127.0.0.1,::1"
export NO_PROXY="localhost,127.0.0.1,::1"
EOF
    chmod 644 "$ENV_PROXY_FILE"
    INSTALLED_ENV_PROXY=1
    echo "==> Variables http_proxy/https_proxy écrites ($ENV_PROXY_FILE)"
  fi
fi

# 12. Unité systemd (optionnel).
INSTALLED_SYSTEMD=0
INSTALLED_BIN=0
if [ "$ENABLE_SYSTEMD" -eq 1 ]; then
  if ! command -v systemctl >/dev/null 2>&1; then
    echo "systemctl introuvable — distribution sans systemd ; --enable-systemd ignoré." >&2
  else
    # Copie le binaire dans /usr/local/bin/ s'il n'est pas déjà à jour.
    if [ ! -f "$SYSTEMD_BIN" ] || ! cmp -s "$BIN" "$SYSTEMD_BIN"; then
      cp "$BIN" "$SYSTEMD_BIN"
      chmod 755 "$SYSTEMD_BIN"
      INSTALLED_BIN=1
      echo "==> Binaire copié dans $SYSTEMD_BIN"
    else
      INSTALLED_BIN=$OLD_INSTALLED_BIN
    fi

    UNIT_SRC="$SCRIPT_DIR/brocolis.service"
    if [ ! -f "$UNIT_SRC" ]; then
      echo "Unité systemd introuvable : $UNIT_SRC" >&2
      exit 2
    fi
    cp "$UNIT_SRC" "$SYSTEMD_UNIT"
    chmod 644 "$SYSTEMD_UNIT"
    systemctl daemon-reload
    systemctl enable --now brocolis.service
    INSTALLED_SYSTEMD=1
    echo "==> Service systemd installé et démarré : brocolis.service"
  fi
fi

# 13. Persiste l'état pour uninstall.sh.
{
  echo "DISTRO_FAMILY='$DISTRO_FAMILY'"
  echo "INSTALLED_USER=$INSTALLED_USER"
  echo "INSTALLED_CA=$INSTALLED_CA"
  echo "TRUST_FILE='$TRUST_FILE'"
  echo "TRUST_UPDATE='$TRUST_UPDATE'"
  echo "INSTALLED_FF_POLICY=$INSTALLED_FF_POLICY"
  echo "FF_POLICY='$FF_POLICY'"
  echo "INSTALLED_CONFIG=$INSTALLED_CONFIG"
  echo "CONFIG_FILE='$CONFIG_FILE'"
  echo "ETC_DIR='$ETC_DIR'"
  echo "INSTALLED_ENV_PROXY=$INSTALLED_ENV_PROXY"
  echo "ENV_PROXY_FILE='$ENV_PROXY_FILE'"
  echo "INSTALLED_SYSTEMD=$INSTALLED_SYSTEMD"
  echo "SYSTEMD_UNIT='$SYSTEMD_UNIT'"
  echo "INSTALLED_BIN=$INSTALLED_BIN"
  echo "SYSTEMD_BIN='$SYSTEMD_BIN'"
} > "$STATE_FILE"
chmod 600 "$STATE_FILE"

# 14. Récap.
if [ "$INSTALLED_SYSTEMD" -eq 1 ]; then
  cat <<EOF

✓ Installation terminée.

Le service brocolis.service tourne en arrière-plan.
  Statut  : systemctl status brocolis
  Logs    : journalctl -u brocolis -f
  Recharger la clé partagée : systemctl reload brocolis  (SIGUSR1)
  Arrêter : systemctl stop brocolis

Proxy : 127.0.0.1:$LISTEN_PORT (HTTP + HTTPS)

Désinstaller :
    sudo $SCRIPT_DIR/uninstall.sh
EOF
else
  cat <<EOF

✓ Installation terminée.

Démarrer le proxy :
    brocolis run --config $CONFIG_FILE

Configurez votre navigateur sur le proxy HTTP **et** HTTPS 127.0.0.1:$LISTEN_PORT
(Firefox suivra la politique entreprise déposée ; pour Chromium, voir
--proxy-server ou les paramètres système).

Désinstaller :
    sudo $SCRIPT_DIR/uninstall.sh
EOF
fi
