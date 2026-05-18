#!/usr/bin/env bash
# Assemble un bundle macOS Brocolis.app à partir d'un binaire déjà compilé.
#
# Usage :
#   scripts/build-macos-app.sh <chemin-binaire> [dossier-sortie] [version]
#
# Exemple (depuis la CI macOS, après un build universel) :
#   scripts/build-macos-app.sh target/universal/brocolis-gui dist 0.1.0
#
# Le binaire doit avoir été compilé pour macOS avec la fonctionnalité `gui` :
#   cargo build --release --features gui --bin brocolis-gui
#
# Ce script peut s'exécuter sur n'importe quel OS pour vérifier la structure du
# bundle ; l'application ne s'exécutera toutefois que sur macOS.
set -euo pipefail

BIN_PATH="${1:?chemin du binaire requis}"
OUT_DIR="${2:-dist}"
VERSION="${3:-0.1.0}"

APP_NAME="Brocolis"
IDENTIFIER="com.maxlestage.brocolis"
APP_DIR="${OUT_DIR}/${APP_NAME}.app"

if [ ! -f "${BIN_PATH}" ]; then
  echo "erreur : binaire introuvable : ${BIN_PATH}" >&2
  exit 1
fi

rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS" "${APP_DIR}/Contents/Resources"

cp "${BIN_PATH}" "${APP_DIR}/Contents/MacOS/${APP_NAME}"
chmod +x "${APP_DIR}/Contents/MacOS/${APP_NAME}"

# Icône optionnelle si présente.
if [ -f "assets/${APP_NAME}.icns" ]; then
  cp "assets/${APP_NAME}.icns" "${APP_DIR}/Contents/Resources/${APP_NAME}.icns"
  ICON_KEY="  <key>CFBundleIconFile</key>
  <string>${APP_NAME}</string>"
else
  ICON_KEY=""
fi

cat > "${APP_DIR}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleExecutable</key>
  <string>${APP_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>${IDENTIFIER}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.utilities</string>
${ICON_KEY}
</dict>
</plist>
PLIST

echo "Bundle créé : ${APP_DIR}"
