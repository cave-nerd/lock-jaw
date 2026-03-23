#!/bin/bash
# Build LockJaw AppImage (arch-aware: x86_64 and aarch64)
# Usage: ./build-appimage.sh
set -e

ARCH="${ARCH:-$(uname -m)}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DIST="$SCRIPT_DIR/dist"
APPDIR="$DIST/LockJaw.AppDir"
OUTPUT="$DIST/LockJaw-${ARCH}.AppImage"
APPIMAGETOOL_EXTRACT="$DIST/squashfs-root-${ARCH}"

# ── 1. Build the Rust binary ─────────────────────────────────────────────────
echo "==> Building lockjaw (release) for ${ARCH}..."
cd "$SCRIPT_DIR"
cargo build --release -p lj-ui

# ── 2. Set up AppDir ──────────────────────────────────────────────────────────
echo "==> Setting up AppDir..."
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"

cp target/release/lockjaw "$APPDIR/usr/bin/lockjaw"

# AppRun
cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE="${SELF%/*}"
exec "${HERE}/usr/bin/lockjaw" "$@"
APPRUN
chmod +x "$APPDIR/AppRun"

# .desktop
cat > "$APPDIR/lockjaw.desktop" <<'DESKTOP'
[Desktop Entry]
Name=Lock Jaw
GenericName=Markdown Notes
Comment=Fast, local-first markdown note-taking
Exec=lockjaw
Icon=lockjaw
Type=Application
Categories=Utility;TextEditor;
Keywords=notes;markdown;editor;text;
StartupNotify=true
DESKTOP

# Icon — generate directly into AppDir
LJ_ICON_OUT="$APPDIR/lockjaw.png" python3 -c "
import os
from PIL import Image, ImageDraw
size=256; img=Image.new('RGBA',(size,size),(0,0,0,0)); draw=ImageDraw.Draw(img)
cx=size//2
draw.rounded_rectangle([12,12,244,244],radius=40,fill=(26,26,46,255))
draw.arc([cx-40,40,cx+40,110],start=180,end=0,fill=(233,69,96,255),width=16)
draw.rounded_rectangle([cx-52,118,cx+52,210],radius=14,fill=(233,69,96,255))
draw.ellipse([cx-16,142,cx+16,174],fill=(26,26,46,255))
draw.rectangle([cx-7,158,cx+7,180],fill=(26,26,46,255))
img.save(os.environ['LJ_ICON_OUT'])
print('    icon written')
" || echo "    WARNING: PIL not available; add lockjaw.png to $APPDIR/ manually"

# ── 3. Ensure appimagetool is available ──────────────────────────────────────
APPIMAGETOOL_BIN="$DIST/appimagetool-${ARCH}"
if [ ! -f "$APPIMAGETOOL_BIN" ]; then
    echo "==> Downloading go-appimage appimagetool for ${ARCH}..."
    APPTOOL_URL=$(curl -fsSL "https://api.github.com/repos/probonopd/go-appimage/releases/tags/continuous" \
      | grep "browser_download_url" \
      | grep "appimagetool.*${ARCH}\.AppImage\"" \
      | head -1 | cut -d'"' -f4)
    echo "    URL: $APPTOOL_URL"
    curl -fsSL -o "$APPIMAGETOOL_BIN" "$APPTOOL_URL"
    chmod +x "$APPIMAGETOOL_BIN"
fi

# ── 4. Build AppImage ─────────────────────────────────────────────────────────
echo "==> AppDir contents (ELF check):"
find "$APPDIR" -type f -exec file {} \;

echo "==> Packaging AppImage (ARCH=${ARCH})..."
export ARCH
cd "$SCRIPT_DIR"
"$APPIMAGETOOL_BIN" "$APPDIR" "$OUTPUT"

echo ""
echo "Done!  $(ls -lh "$OUTPUT" | awk '{print $5, $9}')"
