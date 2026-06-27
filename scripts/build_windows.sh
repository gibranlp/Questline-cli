#!/bin/bash
# ─────────────────────────────────────────────────────────────────────────────
# build_windows.sh — cross-compila Questline para Windows desde Linux
# ─────────────────────────────────────────────────────────────────────────────
# Cross-compiles Questline for Windows (x86_64) from a Linux host.
# Produces questline.exe in dist/windows/.
#
# Requires:
#   rustup target add x86_64-pc-windows-gnu
#   pacman -S mingw-w64-gcc        (Arch)
#   apt-get install gcc-mingw-w64  (Debian/Ubuntu)
#
# Uso: ./scripts/build_windows.sh [version]
# If version is omitted it is read from Cargo.toml.

set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="${1:-$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')}"
TARGET="x86_64-pc-windows-gnu"
DIST="dist/windows"

echo "Building questline v${VERSION} for Windows (${TARGET})..."

# Ensure the Rust target is installed
if ! rustup target list --installed | grep -q "$TARGET"; then
    echo "  Adding Rust target ${TARGET}..."
    rustup target add "$TARGET"
fi

# Ensure the MinGW linker is available
if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "Error: x86_64-w64-mingw32-gcc not found."
    echo "  Arch:          sudo pacman -S mingw-w64-gcc"
    echo "  Debian/Ubuntu: sudo apt-get install gcc-mingw-w64-x86-64"
    exit 1
fi

cargo build --release --target "$TARGET"

mkdir -p "$DIST"
cp "target/${TARGET}/release/questline.exe" "${DIST}/questline.exe"

# Bundle a minimal README for the Windows zip
cat > "${DIST}/README.txt" <<EOF
Questline v${VERSION} for Windows
===================================

Run questline.exe from PowerShell or Windows Terminal.

For the best experience use Windows Terminal with a Nerd Font.

Source: https://github.com/gibranlp/Questline
EOF

# Create a zip archive
ZIP_NAME="questline-v${VERSION}-windows-x86_64.zip"
(cd "$DIST" && zip -r "../../${ZIP_NAME}" .)
mv "${ZIP_NAME}" "dist/${ZIP_NAME}"

echo "Windows build complete:"
echo "  Executable : ${DIST}/questline.exe"
echo "  Archive    : dist/${ZIP_NAME}"
