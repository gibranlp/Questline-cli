#!/bin/bash
# ─────────────────────────────────────────────────────────────────────────────
# build_appimage.sh — empaqueta Questline como AppImage para Linux
# ─────────────────────────────────────────────────────────────────────────────
set -e

# Change to project root directory
cd "$(dirname "$0")/.."

echo "Building Questline in release mode..."
cargo build --release

echo "Setting up AppDir..."
rm -rf AppDir
mkdir -p AppDir/usr/bin
mkdir -p AppDir/usr/lib
mkdir -p AppDir/usr/share/questline/assets/icons

cp target/release/questline AppDir/usr/bin/
cp packaging/questline.desktop AppDir/
cp packaging/questline.png AppDir/
cp -R assets/icons/notifications AppDir/usr/share/questline/assets/icons/

# Create AppRun script
cat << 'EOF' > AppDir/AppRun
#!/bin/sh
SELF=$(readlink -f "$0")
HERE=$(dirname "$SELF")
exec "$HERE/usr/bin/questline" "$@"
EOF
chmod +x AppDir/AppRun

# Download appimagetool if not present
if [ ! -f appimagetool ]; then
    echo "Downloading appimagetool..."
    curl -Lo appimagetool https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage
    chmod +x appimagetool
fi

echo "Generating AppImage..."
# Disable WAF / sandbox restrictions if running in containers
export ARCH=x86_64
./appimagetool --appimage-extract-and-run AppDir Questline-x86_64.AppImage

echo "AppImage created successfully: Questline-x86_64.AppImage"
