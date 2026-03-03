#!/bin/bash
# Build and optionally install the Lock Jaw Flatpak.
# Usage:
#   ./build-flatpak.sh            # build only (in ./flatpak-build/)
#   ./build-flatpak.sh --install  # build + install for current user
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

MANIFEST="io.github.cave_nerd.LockJaw.yml"
BUILD_DIR="flatpak-build"
REPO_DIR="flatpak-repo"

# ── 1. Compile the release binary ────────────────────────────────────────────
echo "==> Building lockjaw (release)..."
cargo build --release -p lj-ui

# ── 2. Build Flatpak ─────────────────────────────────────────────────────────
echo "==> Running flatpak-builder..."
flatpak-builder \
    --force-clean \
    --repo="$REPO_DIR" \
    "$BUILD_DIR" \
    "$MANIFEST"

# ── 3. Install (optional) ────────────────────────────────────────────────────
if [[ "$1" == "--install" ]]; then
    echo "==> Installing Flatpak for current user..."
    flatpak --user remote-add --no-gpg-verify --if-not-exists \
        lockjaw-local "$REPO_DIR"
    flatpak --user install --reinstall -y lockjaw-local io.github.cave_nerd.LockJaw
    echo ""
    echo "Installed! Run with:"
    echo "  flatpak run io.github.cave_nerd.LockJaw"
else
    echo ""
    echo "Build complete.  To install and run:"
    echo "  ./build-flatpak.sh --install"
    echo "  flatpak run io.github.cave_nerd.LockJaw"
    echo ""
    echo "Or to test without installing:"
    echo "  flatpak-builder --run $BUILD_DIR $MANIFEST lockjaw"
fi
