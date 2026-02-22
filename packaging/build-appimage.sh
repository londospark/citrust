#!/bin/bash
set -euo pipefail

# Build an AppImage for citrust-gui
# Usage: ./build-appimage.sh <path-to-citrust-gui-binary>

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="${1:?Usage: $0 <path-to-citrust-gui-binary>}"
ARCH="${ARCH:-x86_64}"
APP_DIR="${SCRIPT_DIR}/AppDir"
LINUXDEPLOY="linuxdeploy-${ARCH}.AppImage"
LINUXDEPLOY_URL="https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/${LINUXDEPLOY}"

if [ ! -f "${BINARY_PATH}" ]; then
    echo "Error: Binary not found at ${BINARY_PATH}"
    exit 1
fi

echo "==> Downloading linuxdeploy..."
if [ ! -f "${SCRIPT_DIR}/${LINUXDEPLOY}" ]; then
    curl -fSL -o "${SCRIPT_DIR}/${LINUXDEPLOY}" "${LINUXDEPLOY_URL}"
    chmod +x "${SCRIPT_DIR}/${LINUXDEPLOY}"
fi

echo "==> Preparing AppDir..."
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/usr/bin"
mkdir -p "${APP_DIR}/usr/share/icons/hicolor/256x256/apps"
mkdir -p "${APP_DIR}/usr/share/metainfo"

echo "==> Copying binary..."
cp "${BINARY_PATH}" "${APP_DIR}/usr/bin/citrust-gui"
chmod +x "${APP_DIR}/usr/bin/citrust-gui"

echo "==> Copying icon..."
ICON_PATH="${SCRIPT_DIR}/citrust.png"
if [ -f "${ICON_PATH}" ]; then
    cp "${ICON_PATH}" "${APP_DIR}/usr/share/icons/hicolor/256x256/apps/citrust.png"
else
    echo "Warning: Icon not found at ${ICON_PATH}, generating placeholder"
    # Generate a 1x1 pixel PNG as a minimal placeholder
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\x18\xd8N\x00\x00\x00\x00IEND\xaeB`\x82' > "${APP_DIR}/usr/share/icons/hicolor/256x256/apps/citrust.png"
fi

echo "==> Copying desktop file..."
cp "${SCRIPT_DIR}/citrust-gui.desktop" "${APP_DIR}/usr/share/applications/citrust-gui.desktop"

echo "==> Copying metainfo..."
cp "${SCRIPT_DIR}/io.github.londospark.citrust.metainfo.xml" "${APP_DIR}/usr/share/metainfo/"

echo "==> Building AppImage..."
LDAI_OUTPUT="citrust-gui-${ARCH}.AppImage"
OUTPUT="${LDAI_OUTPUT}" "${SCRIPT_DIR}/${LINUXDEPLOY}" \
    --appdir "${APP_DIR}" \
    --desktop-file "${APP_DIR}/usr/share/applications/citrust-gui.desktop" \
    --icon-file "${APP_DIR}/usr/share/icons/hicolor/256x256/apps/citrust.png" \
    --output appimage

echo "==> Done! AppImage created: ${LDAI_OUTPUT}"
