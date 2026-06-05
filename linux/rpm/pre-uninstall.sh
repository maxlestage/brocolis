#!/bin/sh
# Brocolis — RPM %preun.
# Arg $1 : 0 = désinstallation complète, 1 = upgrade.
# On n'arrête le service qu'en désinstallation complète.
set -e
[ "${1:-0}" -eq 0 ] || exit 0

if command -v systemctl >/dev/null 2>&1; then
    systemctl disable --now brocolis.service >/dev/null 2>&1 || true
fi

exit 0
