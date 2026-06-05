#!/usr/bin/env bash
# Brocolis — désinstallation système sur Linux.
#
# Annule ce que install.sh a fait :
#   - retire le CA du magasin de confiance si install.sh l'y avait mis ;
#   - retire la politique Firefox si install.sh l'avait déposée ;
#   - retire le brocolis.toml généré si install.sh l'avait écrit ;
#   - nettoie /var/lib/brocolis/.
#
# Les fichiers préexistants (CA, politique, config) que install.sh a respectés
# sont également respectés ici : rien n'est supprimé qui n'ait pas été créé par
# nous (l'indicateur INSTALLED_* dans state.env tranche).
#
# Le CA et la clé sous /etc/brocolis/ ne sont PAS supprimés par défaut : ils
# peuvent resservir si vous réinstallez. Passez --purge pour les effacer aussi.
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
      sed -n '2,18p' "$0"
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

# Vérifs minimales : on ne fait rien si l'état est incohérent.
: "${INSTALLED_CA:=0}"
: "${INSTALLED_FF_POLICY:=0}"
: "${INSTALLED_CONFIG:=0}"
: "${TRUST_FILE:=}"
: "${TRUST_UPDATE:=}"
: "${FF_POLICY:=}"
: "${CONFIG_FILE:=}"
: "${ETC_DIR:=/etc/brocolis}"

if [ "$INSTALLED_CA" = "1" ] && [ -n "$TRUST_FILE" ] && [ -f "$TRUST_FILE" ]; then
  rm -f "$TRUST_FILE"
  if [ -n "$TRUST_UPDATE" ]; then
    eval "$TRUST_UPDATE" >/dev/null || echo "Avertissement : $TRUST_UPDATE a échoué (état partiel)" >&2
  fi
  echo "✓ CA retiré du magasin de confiance ($TRUST_FILE)"
elif [ "$INSTALLED_CA" = "1" ]; then
  echo "  CA déjà absent du magasin de confiance — rien à retirer"
else
  echo "  CA préexistant conservé (ce n'est pas nous qui l'avons posé)"
fi

if [ "$INSTALLED_FF_POLICY" = "1" ] && [ -n "$FF_POLICY" ] && [ -f "$FF_POLICY" ]; then
  rm -f "$FF_POLICY"
  rmdir "$(dirname "$FF_POLICY")" 2>/dev/null || true
  echo "✓ Politique Firefox retirée ($FF_POLICY)"
elif [ "$INSTALLED_FF_POLICY" = "1" ]; then
  echo "  Politique Firefox déjà absente — rien à retirer"
else
  echo "  Politique Firefox préexistante conservée"
fi

if [ "$INSTALLED_CONFIG" = "1" ] && [ -n "$CONFIG_FILE" ] && [ -f "$CONFIG_FILE" ]; then
  rm -f "$CONFIG_FILE"
  echo "✓ brocolis.toml retiré ($CONFIG_FILE)"
fi

if [ "$PURGE" -eq 1 ]; then
  rm -rf "$ETC_DIR"
  echo "✓ /etc/brocolis/ supprimé (CA + clé inclus) — --purge"
fi

rm -f "$STATE_FILE"
rmdir "$STATE_DIR" 2>/dev/null || true

echo
echo "✓ Désinstallation terminée."
