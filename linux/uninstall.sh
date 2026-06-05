#!/usr/bin/env bash
# Brocolis — désinstallation système sur Linux.
#
# Annule ce que install.sh a fait, en respectant ce qui préexistait
# (state.env tranche : seuls les INSTALLED_*=1 sont retirés).
#
# Ordre des opérations :
#   1. arrêt + désactivation du service systemd ;
#   2. retrait du binaire copié dans /usr/local/bin/ ;
#   3. retrait du fichier /etc/profile.d/brocolis-proxy.sh ;
#   4. retrait du CA du magasin de confiance ;
#   5. retrait de la politique Firefox ;
#   6. retrait de brocolis.toml ;
#   7. retrait de l'utilisateur système 'brocolis' ;
#   8. nettoyage de /var/lib/brocolis/.
#
# Le CA et la clé sous /etc/brocolis/ NE sont PAS supprimés par défaut :
# ils peuvent resservir à une réinstallation. Passez --purge pour les
# effacer aussi.
#
# Usage :
#   sudo linux/uninstall.sh [--purge]
set -u

STATE_FILE="/var/lib/brocolis/state.env"
STATE_DIR="/var/lib/brocolis"

PURGE=0
for arg in "$@"; do
  case "$arg" in
    --purge) PURGE=1 ;;
    -h|--help)
      sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Argument inconnu : $arg" >&2
      exit 2
      ;;
  esac
done

if [ "$(uname)" != "Linux" ]; then
  echo "Ce script doit être exécuté sur Linux." >&2
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

# Valeurs par défaut si la version du state.env est antérieure.
: "${INSTALLED_USER:=0}"
: "${INSTALLED_CA:=0}"
: "${INSTALLED_FF_POLICY:=0}"
: "${INSTALLED_CONFIG:=0}"
: "${INSTALLED_ENV_PROXY:=0}"
: "${INSTALLED_SYSTEMD:=0}"
: "${INSTALLED_BIN:=0}"
: "${TRUST_FILE:=}"
: "${TRUST_UPDATE:=}"
: "${FF_POLICY:=}"
: "${CONFIG_FILE:=}"
: "${ETC_DIR:=/etc/brocolis}"
: "${ENV_PROXY_FILE:=/etc/profile.d/brocolis-proxy.sh}"
: "${SYSTEMD_UNIT:=/etc/systemd/system/brocolis.service}"
: "${SYSTEMD_BIN:=/usr/local/bin/brocolis}"

# 1. Service systemd : arrêter avant de retirer (libère les fichiers).
if [ "$INSTALLED_SYSTEMD" = "1" ]; then
  if command -v systemctl >/dev/null 2>&1; then
    systemctl disable --now brocolis.service 2>/dev/null || true
  fi
  if [ -f "$SYSTEMD_UNIT" ]; then
    rm -f "$SYSTEMD_UNIT"
    command -v systemctl >/dev/null 2>&1 && systemctl daemon-reload 2>/dev/null || true
    echo "✓ Service systemd arrêté et unité retirée"
  fi
fi

# 2. Binaire copié dans /usr/local/bin/.
if [ "$INSTALLED_BIN" = "1" ] && [ -f "$SYSTEMD_BIN" ]; then
  rm -f "$SYSTEMD_BIN"
  echo "✓ Binaire $SYSTEMD_BIN retiré"
fi

# 3. Variables proxy d'environnement.
if [ "$INSTALLED_ENV_PROXY" = "1" ] && [ -f "$ENV_PROXY_FILE" ]; then
  rm -f "$ENV_PROXY_FILE"
  echo "✓ Variables proxy retirées ($ENV_PROXY_FILE)"
  echo "  (effet sur les NOUVELLES sessions shell — fermez/rouvrez les terminaux)"
fi

# 4. CA dans le magasin de confiance.
if [ "$INSTALLED_CA" = "1" ] && [ -n "$TRUST_FILE" ] && [ -f "$TRUST_FILE" ]; then
  rm -f "$TRUST_FILE"
  if [ -n "$TRUST_UPDATE" ]; then
    eval "$TRUST_UPDATE" >/dev/null || echo "Avertissement : $TRUST_UPDATE a échoué (état partiel)" >&2
  fi
  echo "✓ CA retiré du magasin de confiance ($TRUST_FILE)"
elif [ "$INSTALLED_CA" = "1" ]; then
  echo "  CA déjà absent du magasin — rien à retirer"
else
  echo "  CA préexistant conservé (ce n'est pas nous qui l'avons posé)"
fi

# 5. Politique Firefox.
if [ "$INSTALLED_FF_POLICY" = "1" ] && [ -n "$FF_POLICY" ] && [ -f "$FF_POLICY" ]; then
  rm -f "$FF_POLICY"
  rmdir "$(dirname "$FF_POLICY")" 2>/dev/null || true
  echo "✓ Politique Firefox retirée ($FF_POLICY)"
elif [ "$INSTALLED_FF_POLICY" = "1" ]; then
  echo "  Politique Firefox déjà absente"
else
  echo "  Politique Firefox préexistante conservée"
fi

# 6. Config générée.
if [ "$INSTALLED_CONFIG" = "1" ] && [ -n "$CONFIG_FILE" ] && [ -f "$CONFIG_FILE" ]; then
  rm -f "$CONFIG_FILE"
  echo "✓ brocolis.toml retiré ($CONFIG_FILE)"
fi

# 7. Utilisateur système 'brocolis' (après l'arrêt du service).
if [ "$INSTALLED_USER" = "1" ] && id brocolis >/dev/null 2>&1; then
  if ! userdel brocolis 2>/dev/null; then
    echo "  Avertissement : userdel brocolis a échoué (processus actif ?)" >&2
  else
    echo "✓ Utilisateur système 'brocolis' retiré"
  fi
fi

# 8. Purge complète (CA + clé).
if [ "$PURGE" -eq 1 ]; then
  rm -rf "$ETC_DIR"
  echo "✓ /etc/brocolis/ supprimé (CA + clé inclus) — --purge"
fi

rm -f "$STATE_FILE"
rmdir "$STATE_DIR" 2>/dev/null || true

echo
echo "✓ Désinstallation terminée."
