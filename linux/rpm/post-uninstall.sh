#!/bin/sh
# Brocolis — RPM %postun.
# Arg $1 : 0 = désinstallation complète, 1 = upgrade.
# Le nettoyage système n'est fait qu'en désinstallation complète.
set -e
[ "${1:-0}" -eq 0 ] || exit 0

ETC=/etc/brocolis
TRUST=/etc/pki/ca-trust/source/anchors/brocolis-ca.crt
FF_POLICY_DIR=/etc/firefox/policies
FF_POLICY=$FF_POLICY_DIR/policies.json
STATE=/var/lib/brocolis

# CA + magasin de confiance.
if [ -f "$TRUST" ]; then
    rm -f "$TRUST"
    update-ca-trust >/dev/null 2>&1 || true
fi

# Politique Firefox.
if [ -f "$FF_POLICY" ]; then
    rm -f "$FF_POLICY"
    rmdir "$FF_POLICY_DIR" 2>/dev/null || true
fi

# /etc/brocolis/ et /var/lib/brocolis/.
rm -rf "$ETC" "$STATE"

# Utilisateur système.
if id brocolis >/dev/null 2>&1; then
    userdel brocolis >/dev/null 2>&1 || true
fi

# systemd : rafraîchit l'état.
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload >/dev/null 2>&1 || true
fi

exit 0
