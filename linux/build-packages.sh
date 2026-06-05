#!/usr/bin/env bash
# Brocolis — construction des paquets .deb et .rpm pour Linux.
#
# Utilise cargo-deb et cargo-generate-rpm, qui lisent les métadonnées du
# Cargo.toml ([package.metadata.deb] et [package.metadata.generate-rpm]).
#
# Installe les outils s'ils manquent (cargo install …). La compilation
# release est lancée une seule fois ; les deux packagers la réutilisent.
#
# Les paquets atterrissent dans linux/dist/.
#
# Usage :
#   bash linux/build-packages.sh [deb|rpm]
#
# Sans argument : produit les deux.
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

WHAT="${1:-all}"
case "$WHAT" in
  all|deb|rpm) ;;
  *)
    echo "Argument inconnu : $WHAT (attendu : deb, rpm, ou rien)" >&2
    exit 2
    ;;
esac

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo introuvable — installez Rust (https://rustup.rs)." >&2
  exit 2
fi

DIST="$ROOT_DIR/linux/dist"
mkdir -p "$DIST"

want_deb=0
want_rpm=0
[ "$WHAT" = "all" ] || [ "$WHAT" = "deb" ] && want_deb=1
[ "$WHAT" = "all" ] || [ "$WHAT" = "rpm" ] && want_rpm=1

ensure_tool() {
  local cmd="$1" pkg="$2"
  if cargo "$cmd" --version >/dev/null 2>&1; then
    return 0
  fi
  echo "==> Installation de $pkg (cargo install $pkg)…"
  cargo install "$pkg"
}

if [ "$want_deb" -eq 1 ]; then ensure_tool deb cargo-deb; fi
if [ "$want_rpm" -eq 1 ]; then ensure_tool generate-rpm cargo-generate-rpm; fi

echo "==> Compilation release (cargo build --release --bin brocolis)…"
cargo build --release --bin brocolis

if [ "$want_deb" -eq 1 ]; then
  echo "==> Construction du paquet .deb…"
  cargo deb --no-build
  mv target/debian/*.deb "$DIST/"
fi

if [ "$want_rpm" -eq 1 ]; then
  echo "==> Construction du paquet .rpm…"
  cargo generate-rpm
  mv target/generate-rpm/*.rpm "$DIST/"
fi

echo
ls -lh "$DIST"/*.{deb,rpm} 2>/dev/null || true
echo
echo "✓ Paquets disponibles dans : $DIST/"
echo
echo "Installation :"
echo "  Debian/Ubuntu : sudo apt install $DIST/brocolis_*_amd64.deb"
echo "  Fedora/RHEL   : sudo dnf install $DIST/brocolis-*.x86_64.rpm"
