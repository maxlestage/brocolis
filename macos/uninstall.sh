#!/usr/bin/env bash
# Brocolis — désinstallation système sur macOS.
#
# Annule ce que install.sh a fait, en respectant ce qui préexistait
# (state.env tranche : seuls les INSTALLED_*=1 sont retirés).
#
# Ordre des opérations :
#   1. arrêt + déchargement du LaunchDaemon ;
#   2. retrait du binaire copié dans /usr/local/bin/ ;
#   3. retrait du CA du trousseau Système ;
#   4. retrait des politiques Firefox (celles qu'on a créées) ;
#   5. retrait du profil .mobileconfig ;
#   6. retrait de brocolis.toml ;
#   7. nettoyage de /var/lib/brocolis/.
#
# Le CA et la clé sous /etc/brocolis/ NE sont PAS supprimés par défaut :
# ils peuvent resservir à une réinstallation. --purge les efface aussi.
#
# Usage :
#   sudo macos/uninstall.sh [--purge]
set -u

STATE_FILE="/var/lib/brocolis/state.env"
STATE_DIR="/var/lib/brocolis"
LOG_DIR="/var/log/brocolis"
KEYCHAIN="/Library/Keychains/System.keychain"
CA_CN="Brocolis Root CA"

PURGE=0
for arg in "$@"; do
  case "$arg" in
    --purge) PURGE=1 ;;
    -h|--help)
      sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Argument inconnu : $arg" >&2
      exit 2
      ;;
  esac
done

if [ "$(uname)" != "Darwin" ]; then
  echo "Ce script doit être exécuté sur macOS." >&2
  exit 2
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "==> Élévation des privilèges (sudo)…"
  exec sudo "$0" "$@"
fi

if [ ! -f "$STATE_FILE" ]; then
  echo "Aucun état d'installation trouvé ($STATE_FILE)." >&2
  echo "Rien à désinstaller — ou install.sh n'a jamais été exécuté." >&2
  exit 2
fi

# shellcheck disable=SC1090
. "$STATE_FILE"

# Valeurs par défaut pour compat avec un state.env d'avant ce fichier.
: "${INSTALLED_CA:=0}"
: "${INSTALLED_FF_POLICIES:=}"
: "${INSTALLED_MOBILECONFIG:=0}"
: "${INSTALLED_CONFIG:=0}"
: "${INSTALLED_LAUNCHD:=0}"
: "${INSTALLED_BIN:=0}"
: "${CONFIG_FILE:=/etc/brocolis/brocolis.toml}"
: "${CA_CRT:=/etc/brocolis/brocolis-ca.crt}"
: "${CA_KEY:=/etc/brocolis/brocolis-ca.key}"
: "${MOBILECONFIG:=/etc/brocolis/brocolis-firefox.mobileconfig}"
: "${ETC_DIR:=/etc/brocolis}"
: "${LAUNCHD_PLIST:=/Library/LaunchDaemons/com.maxlestage.brocolis.plist}"
: "${LAUNCHD_LABEL:=com.maxlestage.brocolis}"
: "${LAUNCHD_BIN:=/usr/local/bin/brocolis}"

# 1. LaunchDaemon : arrêter et décharger avant tout retrait de fichier.
if [ "$INSTALLED_LAUNCHD" = "1" ]; then
  if command -v launchctl >/dev/null 2>&1; then
    launchctl bootout "system/$LAUNCHD_LABEL" 2>/dev/null || true
  fi
  if [ -f "$LAUNCHD_PLIST" ]; then
    rm -f "$LAUNCHD_PLIST"
    echo "✓ LaunchDaemon arrêté et plist retiré ($LAUNCHD_PLIST)"
  fi
fi

# 2. Binaire copié dans /usr/local/bin/.
if [ "$INSTALLED_BIN" = "1" ] && [ -f "$LAUNCHD_BIN" ]; then
  rm -f "$LAUNCHD_BIN"
  echo "✓ Binaire $LAUNCHD_BIN retiré"
fi

# 3. CA du trousseau Système.
if [ "$INSTALLED_CA" = "1" ]; then
  if security find-certificate -c "$CA_CN" "$KEYCHAIN" >/dev/null 2>&1; then
    if security delete-certificate -c "$CA_CN" "$KEYCHAIN" 2>/dev/null; then
      echo "✓ CA retiré du trousseau Système"
    else
      echo "Avertissement : security delete-certificate a échoué" >&2
    fi
  else
    echo "  CA déjà absent du trousseau — rien à retirer"
  fi
else
  echo "  CA préexistant conservé (ce n'est pas nous qui l'avons posé)"
fi

# 4. Politiques Firefox : on retire uniquement les fichiers qu'on a créés.
if [ -n "$INSTALLED_FF_POLICIES" ]; then
  IFS=':' read -r -a _ff_policies <<< "$INSTALLED_FF_POLICIES"
  for policy in "${_ff_policies[@]}"; do
    [ -z "$policy" ] && continue
    if [ -f "$policy" ]; then
      rm -f "$policy"
      # Retire le dossier `distribution` s'il est devenu vide.
      dist="$(dirname "$policy")"
      rmdir "$dist" 2>/dev/null || true
      echo "✓ Politique Firefox retirée ($policy)"
    fi
  done
fi

# 5. Profil .mobileconfig.
if [ "$INSTALLED_MOBILECONFIG" = "1" ] && [ -f "$MOBILECONFIG" ]; then
  rm -f "$MOBILECONFIG"
  echo "✓ Profil .mobileconfig retiré ($MOBILECONFIG)"
  echo "  Note : s'il a été installé manuellement dans Réglages Système → Profils,"
  echo "  pensez à l'y retirer aussi."
fi

# 6. brocolis.toml.
if [ "$INSTALLED_CONFIG" = "1" ] && [ -f "$CONFIG_FILE" ]; then
  rm -f "$CONFIG_FILE"
  echo "✓ brocolis.toml retiré ($CONFIG_FILE)"
fi

# 7. Purge complète.
if [ "$PURGE" -eq 1 ]; then
  rm -rf "$ETC_DIR" "$LOG_DIR"
  echo "✓ /etc/brocolis/ et /var/log/brocolis/ supprimés — --purge"
fi

rm -f "$STATE_FILE"
rmdir "$STATE_DIR" 2>/dev/null || true

echo
echo "✓ Désinstallation terminée."
