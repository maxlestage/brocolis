#!/usr/bin/env bash
# Brocolis — construction du paquet .pkg pour macOS.
#
# Stage les artefacts (binaire, LaunchDaemon, README, brocolis-uninstall),
# puis appelle pkgbuild pour produire un installeur signable et
# distribuable.
#
# - Sans argument : utilise le binaire à target/release/brocolis (ou
#   target/universal/brocolis si présent, prioritaire).
# - Avec un chemin : utilise ce binaire-là (ex : brocolis universel
#   construit par cargo + lipo dans la CI).
#
# Le .pkg n'est PAS signé/notarisé par défaut. Pour une distribution
# « propre » macOS, signez-le :
#     productsign --sign "Developer ID Installer: …" out.pkg signed.pkg
#     xcrun notarytool submit signed.pkg --wait …
#
# Usage :
#   bash macos/build-pkg.sh [chemin-binaire]
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
MACOS_DIR="$ROOT_DIR/macos"
cd "$ROOT_DIR"

if [ "$(uname)" != "Darwin" ]; then
  echo "build-pkg.sh ne peut être exécuté que sur macOS (pkgbuild requis)." >&2
  exit 2
fi
if ! command -v pkgbuild >/dev/null 2>&1; then
  echo "pkgbuild introuvable — installez les Xcode Command Line Tools." >&2
  exit 2
fi

# 1. Localise le binaire (ou compile).
if [ "$#" -ge 1 ]; then
  BIN="$1"
elif [ -x "$ROOT_DIR/target/universal/brocolis" ]; then
  BIN="$ROOT_DIR/target/universal/brocolis"
elif [ -x "$ROOT_DIR/target/release/brocolis" ]; then
  BIN="$ROOT_DIR/target/release/brocolis"
else
  echo "==> Compilation release (cargo build --release --bin brocolis)…"
  cargo build --release --bin brocolis
  BIN="$ROOT_DIR/target/release/brocolis"
fi
if [ ! -x "$BIN" ]; then
  echo "Binaire absent ou non exécutable : $BIN" >&2
  exit 2
fi
echo "Binaire utilisé : $BIN"

# 2. Récupère version + identifiant.
VERSION="$(grep -m1 '^version *= *' "$ROOT_DIR/Cargo.toml" | sed -E 's/version *= *"([^"]+)"/\1/')"
IDENT="com.maxlestage.brocolis"
DIST="$MACOS_DIR/dist"
mkdir -p "$DIST"
OUT="$DIST/brocolis-${VERSION}.pkg"

# 3. Stage la payload (mirror de l'arbo cible).
STAGE="$(mktemp -d /tmp/brocolis-pkg-stage.XXXXXX)"
SCRIPTS="$(mktemp -d /tmp/brocolis-pkg-scripts.XXXXXX)"
trap 'rm -rf "$STAGE" "$SCRIPTS"' EXIT

install -d -m 755 "$STAGE/usr/local/bin"
install -d -m 755 "$STAGE/Library/LaunchDaemons"
install -d -m 755 "$STAGE/usr/local/share/doc/brocolis"

install -m 755 "$BIN" "$STAGE/usr/local/bin/brocolis"
install -m 755 "$MACOS_DIR/uninstall.sh" "$STAGE/usr/local/bin/brocolis-uninstall"
install -m 644 "$MACOS_DIR/com.maxlestage.brocolis.plist" \
        "$STAGE/Library/LaunchDaemons/com.maxlestage.brocolis.plist"
install -m 644 "$ROOT_DIR/Readme.md" \
        "$STAGE/usr/local/share/doc/brocolis/README.md"
install -m 644 "$MACOS_DIR/README.md" \
        "$STAGE/usr/local/share/doc/brocolis/MACOS.md"

# 4. Scripts (post-install).
#    postinstall délègue à install.sh, qui a besoin du plist et du binaire
#    à côté pour son propre lookup (SCRIPT_DIR). Le binaire n'est pas
#    rangé dans les scripts (il est déjà dans la payload, /usr/local/bin/),
#    install.sh le retrouvera via `command -v brocolis`.
install -m 755 "$MACOS_DIR/pkg/postinstall" "$SCRIPTS/postinstall"
install -m 755 "$MACOS_DIR/install.sh" "$SCRIPTS/install.sh"
install -m 755 "$MACOS_DIR/uninstall.sh" "$SCRIPTS/uninstall.sh"
install -m 644 "$MACOS_DIR/com.maxlestage.brocolis.plist" \
        "$SCRIPTS/com.maxlestage.brocolis.plist"

# 5. pkgbuild.
echo "==> pkgbuild → $OUT"
pkgbuild \
  --root "$STAGE" \
  --scripts "$SCRIPTS" \
  --identifier "$IDENT" \
  --version "$VERSION" \
  --install-location / \
  "$OUT"

ls -lh "$OUT"
echo
echo "✓ Paquet disponible : $OUT"
echo
echo "Installation :"
echo "  sudo installer -pkg \"$OUT\" -target /"
echo
echo "Désinstallation (après install) :"
echo "  sudo brocolis-uninstall            # garde /etc/brocolis/"
echo "  sudo brocolis-uninstall --purge    # supprime aussi /etc/brocolis/"
