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

# ── 1. Build the Rust binary ────────────────────────────────────────────────
echo "==> Building lockjaw (release) for ${ARCH}..."
cd "$SCRIPT_DIR"
cargo build --release -p lj-ui

# ── 2. Set up AppDir ─────────────────────────────────────────────────────────
echo "==> Setting up AppDir..."
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/lib"

cp target/release/lockjaw "$APPDIR/usr/bin/lockjaw"

# Bundle libssl/libcrypto so the AppImage works regardless of system OpenSSL version
for lib in libssl.so.3 libcrypto.so.3; do
    src=$(readlink -f /lib/${ARCH}-linux-gnu/$lib 2>/dev/null || \
          readlink -f /usr/lib/${ARCH}-linux-gnu/$lib 2>/dev/null || true)
    if [ -n "$src" ] && [ -f "$src" ]; then
        cp "$src" "$APPDIR/usr/lib/$lib"
        echo "    bundled $lib"
    else
        echo "    WARNING: $lib not found, skipping"
    fi
done

# AppRun
cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE="${SELF%/*}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
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

# ── 3. Ensure appimagetool is extracted ──────────────────────────────────────
if [ ! -f "$APPIMAGETOOL_EXTRACT/AppRun" ]; then
    echo "==> Downloading appimagetool for ${ARCH}..."
    curl -L -o "$DIST/appimagetool-${ARCH}" \
      "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-${ARCH}.AppImage"
    chmod +x "$DIST/appimagetool-${ARCH}"
    echo "==> Extracting appimagetool (bypassing FUSE requirement)..."
    cd "$DIST" && "./appimagetool-${ARCH}" --appimage-extract
    mv "$DIST/squashfs-root" "$APPIMAGETOOL_EXTRACT"
fi

# ── 4. Build AppImage ─────────────────────────────────────────────────────────
echo "==> Packaging AppImage..."
cd "$DIST"
ARCH=${ARCH} "$APPIMAGETOOL_EXTRACT/AppRun" "$APPDIR" "$OUTPUT"

echo ""
echo "Done!  $(ls -lh "$OUTPUT" | awk '{print $5, $9}')"
