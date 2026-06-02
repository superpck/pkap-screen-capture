#!/bin/bash
# build.sh — pkap release builder
#
# Usage:
#   ./build.sh                  → macOS universal (arm64 + x86_64)
#   ./build.sh --arm            → macOS Apple Silicon only
#   ./build.sh --intel          → macOS Intel only
#   ./build.sh --universal      → macOS universal binary (default)
#   ./build.sh --linux          → Linux x86_64 (requires Docker)
#   ./build.sh --windows        → Windows x86_64 (requires Docker + Wine)
#   ./build.sh --all            → all platforms (requires Docker)

set -e

# ── Terminal colours ───────────────────────────────────────────────────────────
B='\033[1m'; BLUE='\033[1;34m'; GREEN='\033[1;32m'
YELLOW='\033[1;33m'; RED='\033[1;31m'; NC='\033[0m'

step()  { echo -e "\n${BLUE}▶ $*${NC}"; }
ok()    { echo -e "${GREEN}✓ $*${NC}"; }
warn()  { echo -e "${YELLOW}! $*${NC}"; }
err()   { echo -e "${RED}✗ $*${NC}"; exit 1; }
info()  { echo -e "  $*"; }

APP_NAME="pkap"
TARGET_DIR="src-tauri/target"
DIST_DIR="app"

# ── Helpers ────────────────────────────────────────────────────────────────────

ensure_dist_dir() {
    if [ ! -d "$DIST_DIR" ]; then
        mkdir -p "$DIST_DIR"
        info "Created $DIST_DIR/ folder"
    fi
}

copy_to_dist() {
    local triple="$1"
    local bundle_dir="$TARGET_DIR/$triple/release/bundle"
    
    ensure_dist_dir
    
    step "Copying artifacts to $DIST_DIR/"
    
    # Copy .app bundles
    find "$bundle_dir" -name "*.app" -maxdepth 2 2>/dev/null | while read -r app; do
        local basename=$(basename "$app")
        local dest="$DIST_DIR/$basename"
        rm -rf "$dest"
        cp -R "$app" "$dest"
        ok "Copied: $basename"
    done
    
    # Copy .dmg files
    find "$bundle_dir" -name "*.dmg" -maxdepth 2 2>/dev/null | while read -r dmg; do
        local basename=$(basename "$dmg")
        cp "$dmg" "$DIST_DIR/$basename"
        ok "Copied: $basename"
    done
    
    # Copy Linux packages
    find "$bundle_dir" -name "*.deb" -o -name "*.AppImage" -maxdepth 2 2>/dev/null | while read -r pkg; do
        local basename=$(basename "$pkg")
        cp "$pkg" "$DIST_DIR/$basename"
        ok "Copied: $basename"
    done
    
    # Copy Windows packages
    find "$bundle_dir" -name "*.msi" -o -name "*.exe" -maxdepth 2 2>/dev/null | while read -r exe; do
        local basename=$(basename "$exe")
        cp "$exe" "$DIST_DIR/$basename"
        ok "Copied: $basename"
    done
}

ensure_target() {
    local target="$1"
    if ! rustup target list --installed 2>/dev/null | grep -q "^$target"; then
        step "Adding Rust target: $target"
        rustup target add "$target"
    fi
}

print_artifacts() {
    local triple="$1"
    local bundle_dir="$TARGET_DIR/$triple/release/bundle"
    echo ""
    echo -e "${B}Output files:${NC}"
    find "$bundle_dir" \
        \( -name "*.app" -o -name "*.dmg" \
        -o -name "*.deb" -o -name "*.AppImage" \
        -o -name "*.msi" -o -name "*.exe" \) 2>/dev/null \
      | sort | while read -r f; do
            size=$(du -sh "$f" 2>/dev/null | cut -f1)
            echo -e "  ${GREEN}$f${NC}  (${size})"
        done
}

# ── macOS builds ───────────────────────────────────────────────────────────────

build_mac_arm() {
    step "macOS Apple Silicon  (aarch64-apple-darwin)"
    ensure_target aarch64-apple-darwin
    cargo tauri build --target aarch64-apple-darwin
    print_artifacts "aarch64-apple-darwin"
    copy_to_dist "aarch64-apple-darwin"
    ok "Done — Apple Silicon build"
}

build_mac_intel() {
    step "macOS Intel  (x86_64-apple-darwin)"
    ensure_target x86_64-apple-darwin
    cargo tauri build --target x86_64-apple-darwin
    print_artifacts "x86_64-apple-darwin"
    copy_to_dist "x86_64-apple-darwin"
    ok "Done — Intel build"
}

build_mac_universal() {
    step "macOS Universal  (arm64 + x86_64)"
    ensure_target aarch64-apple-darwin
    ensure_target x86_64-apple-darwin
    cargo tauri build --target universal-apple-darwin
    print_artifacts "universal-apple-darwin"
    copy_to_dist "universal-apple-darwin"
    ok "Done — Universal binary"
}

# ── Linux build (via Docker) ───────────────────────────────────────────────────

build_linux() {
    step "Linux x86_64  (via Docker)"

    if ! command -v docker &>/dev/null; then
        err "Docker not found. Install Docker Desktop and try again."
    fi

    # Uses the official Tauri cross-build image.
    docker run --rm \
        -v "$(pwd)":/app \
        -w /app \
        -e WEBKIT_DISABLE_COMPOSITING_MODE=1 \
        ghcr.io/cross-rs/x86_64-unknown-linux-gnu:latest \
        bash -c "
            apt-get update -q &&
            apt-get install -y -q \
                libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev \
                libayatana-appindicator3-dev librsvg2-dev &&
            curl https://sh.rustup.rs -sSf | sh -s -- -y &&
            source ~/.cargo/env &&
            cargo install tauri-cli &&
            cargo tauri build --target x86_64-unknown-linux-gnu
        " 2>&1

    print_artifacts "x86_64-unknown-linux-gnu"
    copy_to_dist "x86_64-unknown-linux-gnu"
    ok "Done — Linux build"
}

# ── Windows build ──────────────────────────────────────────────────────────────

build_windows() {
    step "Windows x86_64"

    warn "Cross-compiling to Windows from macOS is not currently supported."
    warn "The 'scap' screen capture library has dependency conflicts when cross-compiling."
    info ""
    info "To build Windows binaries, you have two options:"
    info ""
    info "1. Build on a Windows machine:"
    info "   ${B}cargo tauri build --target x86_64-pc-windows-msvc${NC}"
    info ""
    info "2. Use GitHub Actions (recommended):"
    info "   • Create .github/workflows/release.yml"
    info "   • Use the official Tauri workflow template:"
    info "   • https://tauri.app/distribute/github-actions/"
    info ""
    info "This will create:"
    info "   • pkap.exe           (standalone executable)"
    info "   • pkap_0.1.0_x64.msi (Windows installer)"
    info ""
    
    echo -e "${YELLOW}For a complete GitHub Actions workflow, see:${NC}"
    echo -e "${BLUE}https://github.com/tauri-apps/tauri-action${NC}"
}

# ── Summary header ─────────────────────────────────────────────────────────────

echo -e "${B}"
echo "╔══════════════════════════════════════╗"
echo "║       pkap  —  Release Builder       ║"
echo "╚══════════════════════════════════════╝"
echo -e "${NC}"

# ── Arg dispatch ───────────────────────────────────────────────────────────────

case "${1:-}" in
    --arm)
        build_mac_arm
        ;;
    --intel)
        build_mac_intel
        ;;
    "" | --universal | --mac)
        build_mac_universal
        ;;
    --linux)
        build_linux
        ;;
    --windows | --win)
        build_windows
        ;;
    --all)
        build_mac_universal
        build_linux
        build_windows
        ;;
    -h | --help)
        echo "Usage: $0 [option]"
        echo ""
        echo "  (none) / --universal   macOS Universal binary  [default]"
        echo "  --arm                  macOS Apple Silicon"
        echo "  --intel                macOS Intel"
        echo "  --linux                Linux x86_64  (requires Docker)"
        echo "  --windows              Windows x86_64  (requires Docker, see notes)"
        echo "  --all                  All platforms"
        ;;
    *)
        err "Unknown option: $1  (try --help)"
        ;;
esac
