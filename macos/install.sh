#!/usr/bin/env bash
# Brocolis — installation système sur macOS (alternative à `brocolis auto`).
#
# `brocolis auto` est la voie native sur macOS : il fait exactement la même
# chose en interactif et tout restaure à l'arrêt (Ctrl-C). Ce script est
# l'équivalent persistant — utile quand on veut laisser le proxy tourner
# en LaunchDaemon, ou contrôler chaque étape à la main.
#
# Setup automatique :
#   - CA dans /etc/brocolis/, ajouté au **trousseau Système** ;
#   - politique entreprise Firefox dans chaque variante détectée
#     (Firefox, Developer Edition, Nightly, ESR) ;
#   - profil .mobileconfig à côté du CA (alternative ne modifiant pas
#     le bundle Firefox — installation manuelle dans Réglages Système) ;
#   - brocolis.toml prêt à l'emploi avec une clé fraîche.
#
# Flag optionnel :
#   --enable-launchd   Copie le binaire dans /usr/local/bin/, installe le
#                      plist dans /Library/LaunchDaemons/, le bootstrap
#                      et le démarre. Le proxy tourne dès lors en service.
#
# Variables d'environnement :
#   BROCOLIS_PORT      Port d'écoute (défaut 8080).
#
# Idempotent : ré-exécutable sans dommage. L'état (ce qui a été créé par
# nous) est suivi dans /var/lib/brocolis/state.env pour que uninstall.sh
# restaure proprement.
#
# Usage :
#   sudo macos/install.sh [--enable-launchd]
set -u

ETC_DIR="/etc/brocolis"
STATE_DIR="/var/lib/brocolis"
LOG_DIR="/var/log/brocolis"
STATE_FILE="$STATE_DIR/state.env"
CA_CRT="$ETC_DIR/brocolis-ca.crt"
CA_KEY="$ETC_DIR/brocolis-ca.key"
CONFIG_FILE="$ETC_DIR/brocolis.toml"
MOBILECONFIG="$ETC_DIR/brocolis-firefox.mobileconfig"
KEYCHAIN="/Library/Keychains/System.keychain"
CA_CN="Brocolis Root CA"
LAUNCHD_PLIST="/Library/LaunchDaemons/com.maxlestage.brocolis.plist"
LAUNCHD_BIN="/usr/local/bin/brocolis"
LAUNCHD_LABEL="com.maxlestage.brocolis"
LISTEN_PORT="${BROCOLIS_PORT:-8080}"

# Variantes de Firefox à scanner — strict miroir de src/system/macos.rs.
FIREFOX_APPS=(
  "/Applications/Firefox.app"
  "/Applications/Firefox Developer Edition.app"
  "/Applications/Firefox Nightly.app"
  "/Applications/Firefox ESR.app"
)

# Parsing des flags.
ENABLE_LAUNCHD=0
for arg in "$@"; do
  case "$arg" in
    --enable-launchd) ENABLE_LAUNCHD=1 ;;
    -h|--help)
      sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Argument inconnu : $arg" >&2
      echo "Utilisez -h pour l'aide." >&2
      exit 2
      ;;
  esac
done

if [ "$(uname)" != "Darwin" ]; then
  echo "Ce script doit être exécuté sur macOS (uname=$(uname))." >&2
  exit 2
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "==> Élévation des privilèges (sudo) pour l'installation système…"
  exec sudo --preserve-env=BROCOLIS_PORT "$0" "$@"
fi

# 1. Localise le binaire brocolis.
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

# 2. Crée les dossiers de travail.
mkdir -p "$ETC_DIR" "$STATE_DIR" "$LOG_DIR"
chmod 755 "$ETC_DIR"
chmod 700 "$STATE_DIR"
chmod 755 "$LOG_DIR"

# 3. Recharge l'état précédent pour préserver les flags INSTALLED_*=1
#    si la ressource correspondante existe toujours.
OLD_INSTALLED_CA=0
OLD_INSTALLED_FF_POLICIES=""
OLD_INSTALLED_MOBILECONFIG=0
OLD_INSTALLED_CONFIG=0
OLD_INSTALLED_LAUNCHD=0
OLD_INSTALLED_BIN=0
if [ -f "$STATE_FILE" ]; then
  # shellcheck disable=SC1090
  . "$STATE_FILE"
  OLD_INSTALLED_CA="${INSTALLED_CA:-0}"
  OLD_INSTALLED_FF_POLICIES="${INSTALLED_FF_POLICIES:-}"
  OLD_INSTALLED_MOBILECONFIG="${INSTALLED_MOBILECONFIG:-0}"
  OLD_INSTALLED_CONFIG="${INSTALLED_CONFIG:-0}"
  OLD_INSTALLED_LAUNCHD="${INSTALLED_LAUNCHD:-0}"
  OLD_INSTALLED_BIN="${INSTALLED_BIN:-0}"
fi

# 4. Génère (ou recharge) le CA.
if [ ! -f "$CA_CRT" ] || [ ! -f "$CA_KEY" ]; then
  echo "==> Génération du CA dans $ETC_DIR…"
  "$BIN" ca --ca-cert "$CA_CRT" --ca-key "$CA_KEY" >/dev/null
else
  echo "CA déjà présent dans $ETC_DIR : conservé."
fi

# 5. Ajoute le CA au trousseau Système.
INSTALLED_CA=0
if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
  INSTALLED_CA=$OLD_INSTALLED_CA
  echo "CA déjà présent dans le trousseau Système : conservé à l'arrêt."
else
  if security add-trusted-cert -d -r trustRoot -k "$KEYCHAIN" "$CA_CRT" 2>/dev/null; then
    INSTALLED_CA=1
    echo "==> CA approuvé dans le trousseau Système"
  else
    echo "Avertissement : security add-trusted-cert a échoué (CA peut déjà être approuvé)" >&2
  fi
fi

# 6. Politique entreprise Firefox (toutes variantes détectées).
NEW_FF_POLICIES=()
ANY_FIREFOX=0
for app in "${FIREFOX_APPS[@]}"; do
  [ -d "$app" ] || continue
  ANY_FIREFOX=1
  dist="$app/Contents/Resources/distribution"
  policy="$dist/policies.json"
  if [ -f "$policy" ]; then
    echo "Politique Firefox déjà présente ($policy) : laissée intacte."
    # Si on l'avait déjà installée selon l'ancien state, on la conserve.
    case ":$OLD_INSTALLED_FF_POLICIES:" in
      *":$policy:"*) NEW_FF_POLICIES+=("$policy") ;;
    esac
    continue
  fi
  mkdir -p "$dist"
  cat > "$policy" <<'JSON'
{
  "policies": {
    "Certificates": {
      "ImportEnterpriseRoots": true
    }
  }
}
JSON
  chmod 644 "$policy"
  NEW_FF_POLICIES+=("$policy")
  echo "==> Politique Firefox déposée ($policy)"
done
if [ "$ANY_FIREFOX" -eq 0 ]; then
  echo "Aucun Firefox détecté dans /Applications/ — étape ignorée."
fi
# Sérialise en string « path1:path2:... » pour state.env.
INSTALLED_FF_POLICIES="$(IFS=:; printf '%s' "${NEW_FF_POLICIES[*]:-}")"

# 7. Profil .mobileconfig (alternative ne modifiant pas le bundle Firefox).
INSTALLED_MOBILECONFIG=0
if [ -f "$MOBILECONFIG" ]; then
  INSTALLED_MOBILECONFIG=$OLD_INSTALLED_MOBILECONFIG
  echo "$MOBILECONFIG déjà présent : laissé intact."
else
  if "$BIN" --help 2>&1 | grep -q "ca"; then
    # On régénère le .mobileconfig en utilisant ca --export à coté + un script
    # inline qui écrit le plist. En réalité, brocolis auto écrit le
    # .mobileconfig directement ; ici on duplique la logique en shell.
    cat > "$MOBILECONFIG" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>PayloadContent</key>
  <array>
    <dict>
      <key>PayloadType</key>
      <string>com.apple.ManagedClient.preferences</string>
      <key>PayloadVersion</key>
      <integer>1</integer>
      <key>PayloadIdentifier</key>
      <string>com.maxlestage.brocolis.firefox</string>
      <key>PayloadUUID</key>
      <string>4F0E8A10-2C3D-4E5F-9A1B-BR0C0L15F1RE</string>
      <key>PayloadDisplayName</key>
      <string>Brocolis Firefox</string>
      <key>PayloadContent</key>
      <dict>
        <key>org.mozilla.firefox</key>
        <dict>
          <key>Forced</key>
          <array>
            <dict>
              <key>mcx_preference_settings</key>
              <dict>
                <key>EnterprisePoliciesEnabled</key>
                <true/>
                <key>Certificates</key>
                <dict>
                  <key>ImportEnterpriseRoots</key>
                  <true/>
                </dict>
              </dict>
            </dict>
          </array>
        </dict>
      </dict>
    </dict>
  </array>
  <key>PayloadDisplayName</key>
  <string>Brocolis — Confiance CA Firefox</string>
  <key>PayloadIdentifier</key>
  <string>com.maxlestage.brocolis</string>
  <key>PayloadType</key>
  <string>Configuration</string>
  <key>PayloadUUID</key>
  <string>1B2C3D4E-5F60-4712-8A9B-BR0C0L15C0NF</string>
  <key>PayloadVersion</key>
  <integer>1</integer>
  <key>PayloadScope</key>
  <string>System</string>
</dict>
</plist>
PLIST
    chmod 644 "$MOBILECONFIG"
    INSTALLED_MOBILECONFIG=1
    echo "==> Profil .mobileconfig écrit ($MOBILECONFIG)"
    echo "    Double-cliquez pour l'installer dans Réglages Système → Profils,"
    echo "    puis approuvez-le (alternative qui ne modifie pas le bundle Firefox)."
  fi
fi

# 8. Génère un brocolis.toml par défaut.
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
  INSTALLED_CONFIG=$OLD_INSTALLED_CONFIG
  echo "Config déjà présente : $CONFIG_FILE — non écrasée."
fi
chmod 600 "$CA_KEY" "$CONFIG_FILE"
chmod 644 "$CA_CRT"

# 9. LaunchDaemon (optionnel).
INSTALLED_LAUNCHD=0
INSTALLED_BIN=0
if [ "$ENABLE_LAUNCHD" -eq 1 ]; then
  if ! command -v launchctl >/dev/null 2>&1; then
    echo "launchctl introuvable — --enable-launchd ignoré." >&2
  else
    if [ ! -f "$LAUNCHD_BIN" ] || ! cmp -s "$BIN" "$LAUNCHD_BIN"; then
      cp "$BIN" "$LAUNCHD_BIN"
      chmod 755 "$LAUNCHD_BIN"
      INSTALLED_BIN=1
      echo "==> Binaire copié dans $LAUNCHD_BIN"
    else
      INSTALLED_BIN=$OLD_INSTALLED_BIN
    fi

    PLIST_SRC="$SCRIPT_DIR/com.maxlestage.brocolis.plist"
    if [ ! -f "$PLIST_SRC" ]; then
      echo "Plist LaunchDaemon introuvable : $PLIST_SRC" >&2
      exit 2
    fi
    cp "$PLIST_SRC" "$LAUNCHD_PLIST"
    chown root:wheel "$LAUNCHD_PLIST"
    chmod 644 "$LAUNCHD_PLIST"

    # Si déjà chargé (réinstallation), on décharge avant de recharger.
    launchctl bootout "system/$LAUNCHD_LABEL" 2>/dev/null || true
    launchctl bootstrap system "$LAUNCHD_PLIST"
    launchctl enable "system/$LAUNCHD_LABEL"
    launchctl kickstart -k "system/$LAUNCHD_LABEL"
    INSTALLED_LAUNCHD=1
    echo "==> LaunchDaemon installé et démarré : $LAUNCHD_LABEL"
  fi
fi

# 10. Persiste l'état.
{
  echo "INSTALLED_CA=$INSTALLED_CA"
  echo "INSTALLED_FF_POLICIES='$INSTALLED_FF_POLICIES'"
  echo "INSTALLED_MOBILECONFIG=$INSTALLED_MOBILECONFIG"
  echo "INSTALLED_CONFIG=$INSTALLED_CONFIG"
  echo "CONFIG_FILE='$CONFIG_FILE'"
  echo "CA_CRT='$CA_CRT'"
  echo "CA_KEY='$CA_KEY'"
  echo "MOBILECONFIG='$MOBILECONFIG'"
  echo "ETC_DIR='$ETC_DIR'"
  echo "INSTALLED_LAUNCHD=$INSTALLED_LAUNCHD"
  echo "LAUNCHD_PLIST='$LAUNCHD_PLIST'"
  echo "LAUNCHD_LABEL='$LAUNCHD_LABEL'"
  echo "INSTALLED_BIN=$INSTALLED_BIN"
  echo "LAUNCHD_BIN='$LAUNCHD_BIN'"
} > "$STATE_FILE"
chmod 600 "$STATE_FILE"

# 11. Récap.
if [ "$INSTALLED_LAUNCHD" -eq 1 ]; then
  cat <<EOF

✓ Installation terminée.

Le service brocolis tourne en arrière-plan.
  Statut    : sudo launchctl print system/$LAUNCHD_LABEL | head
  Logs      : tail -f $LOG_DIR/brocolis.log
  Recharger la clé : sudo launchctl kill USR1 system/$LAUNCHD_LABEL
  Arrêter   : sudo launchctl bootout system/$LAUNCHD_LABEL

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
(Safari & Chrome suivront le trousseau Système ; Firefox suivra la politique
entreprise déposée ou le profil .mobileconfig — à approuver manuellement
dans Réglages Système → Confidentialité et sécurité → Profils).

Pour démarrer en LaunchDaemon : relancez avec --enable-launchd.

Désinstaller :
    sudo $SCRIPT_DIR/uninstall.sh
EOF
fi
