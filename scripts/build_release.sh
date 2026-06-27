#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# build_release.sh — compila el release de Questline para Linux y Windows
# ─────────────────────────────────────────────────────────────────────────────
# Build Questline release binaries for all supported platforms and copy them
# into server/releases/latest/ (served by questline.gibranlp.dev).
#
# Preferred method: `cross` (Docker-based, handles everything automatically)
#   cargo install cross     ← install once
#   docker info             ← Docker must be running
#
# Fallback method: native cross-compilation toolchains
#   Linux aarch64 : sudo pacman -S aarch64-linux-gnu-gcc
#   Windows       : sudo pacman -S mingw-w64-gcc
#   macOS         : osxcross (or build natively on a Mac)
#
# Uso:
#   ./scripts/build_release.sh            # build all platforms
#   ./scripts/build_release.sh --linux    # only Linux targets
#   ./scripts/build_release.sh --windows  # only Windows
#   ./scripts/build_release.sh --macos    # only macOS targets
#   ./scripts/build_release.sh --native   # only the current host platform
#   ./scripts/build_release.sh --skip-version  # don't update version.json

set -euo pipefail
cd "$(dirname "$0")/.."

# ── Version ───────────────────────────────────────────────────────────────────
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

# ── Parse flags ───────────────────────────────────────────────────────────────
DO_LINUX=false
DO_MACOS=false
DO_WINDOWS=false
DO_NATIVE=false
UPDATE_VERSION=true

if [ $# -eq 0 ]; then
    DO_LINUX=true; DO_MACOS=true; DO_WINDOWS=true
else
    for arg in "$@"; do
        case "$arg" in
            --linux)         DO_LINUX=true ;;
            --macos)         DO_MACOS=true ;;
            --windows)       DO_WINDOWS=true ;;
            --native)        DO_NATIVE=true ;;
            --skip-version)  UPDATE_VERSION=false ;;
            --all)           DO_LINUX=true; DO_MACOS=true; DO_WINDOWS=true ;;
            *) echo "Unknown flag: $arg"; exit 1 ;;
        esac
    done
fi

# ── Directories ───────────────────────────────────────────────────────────────
OUT_DIR="server/releases/latest"
mkdir -p "$OUT_DIR"

# ── Tool detection ────────────────────────────────────────────────────────────
HOST_OS=$(uname -s)   # Linux | Darwin
HOST_ARCH=$(uname -m) # x86_64 | arm64 | aarch64

HAVE_CROSS=false
HAVE_DOCKER=false
HAVE_MINGW=false
HAVE_AARCH64_GCC=false
HAVE_OSXCROSS=false
HAVE_RUSTUP=false

command -v cross                        &>/dev/null && HAVE_CROSS=true
command -v docker                       &>/dev/null && HAVE_DOCKER=true
command -v x86_64-w64-mingw32-gcc       &>/dev/null && HAVE_MINGW=true
command -v aarch64-linux-gnu-gcc        &>/dev/null && HAVE_AARCH64_GCC=true
command -v x86_64-apple-darwin-clang    &>/dev/null && HAVE_OSXCROSS=true
command -v rustup                       &>/dev/null && HAVE_RUSTUP=true

# cross needs Docker running, not just installed
if $HAVE_CROSS && $HAVE_DOCKER; then
    docker info &>/dev/null || HAVE_DOCKER=false
fi

# cross also requires rustup to locate the host toolchain — without it,
# cross can't determine which Rust to use even though Docker does the build.
USE_CROSS=false
if $HAVE_CROSS && $HAVE_DOCKER && $HAVE_RUSTUP; then
    USE_CROSS=true
fi

# ── Pretty output ─────────────────────────────────────────────────────────────
BOLD='\033[1m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'
YELLOW='\033[1;33m'; RED='\033[0;31m'; DIM='\033[2m'; RESET='\033[0m'

ok()   { echo -e "  ${GREEN}✓${RESET} $*"; }
skip() { echo -e "  ${YELLOW}–${RESET} $* ${DIM}(skipped)${RESET}"; }
fail() { echo -e "  ${RED}✗${RESET} $*"; }
hdr()  { echo -e "\n  ${BOLD}${CYAN}$*${RESET}"; }

echo ""
echo -e "  ${BOLD}QUESTLINE — Release Builder  v${VERSION}${RESET}"
echo -e "  ${DIM}────────────────────────────────────────${RESET}"
echo ""
echo -e "  cross    : $(${HAVE_CROSS}        && echo "${GREEN}yes${RESET}"          || echo "${DIM}no${RESET}")"
echo -e "  docker   : $(${HAVE_DOCKER}       && echo "${GREEN}yes (running)${RESET}" || echo "${DIM}no${RESET}")"
echo -e "  rustup   : $(${HAVE_RUSTUP}       && echo "${GREEN}yes${RESET}"          || echo "${RED}no  ← required by cross${RESET}")"
echo -e "  mingw    : $(${HAVE_MINGW}        && echo "${GREEN}yes${RESET}"          || echo "${DIM}no${RESET}")"
echo -e "  aarch64  : $(${HAVE_AARCH64_GCC} && echo "${GREEN}yes${RESET}"          || echo "${DIM}no${RESET}")"
echo -e "  osxcross : $(${HAVE_OSXCROSS}    && echo "${GREEN}yes${RESET}"          || echo "${DIM}no${RESET}")"
echo -e "  host     : ${HOST_OS}/${HOST_ARCH}"
echo -e "  method   : $(${USE_CROSS} && echo "${GREEN}cross (Docker)${RESET}" || echo "${YELLOW}native toolchains${RESET}")"
echo ""

# ── Ensure a Rust target is installed ─────────────────────────────────────────
ensure_target() {
    local target="$1"
    if $HAVE_RUSTUP; then
        rustup target add "$target" 2>/dev/null || true
        return
    fi
    # System Rust: check if target is already available
    if cargo build --target "$target" --dry-run --manifest-path /dev/null 2>&1 | grep -q "error\[E0463\]"; then
        echo -e "  ${YELLOW}Warning: target $target may not be installed.${RESET}"
        echo -e "  ${DIM}Install rustup to manage targets: https://rustup.rs${RESET}"
    fi
}

# ── Core build function ───────────────────────────────────────────────────────
# build_target <rust-target> <output-filename> <src-binary-name>
# Returns 0 on success, 1 on skip.
BUILT=()
FAILED=()

build_target() {
    local rust_target="$1"
    local out_name="$2"
    local src_bin="$3"

    echo -e "  ${DIM}Target: ${rust_target}${RESET}"

    # Clean our package so the version string is always freshly embedded.
    cargo clean -p questline --target "$rust_target" 2>/dev/null || true

    if $USE_CROSS; then
        cross build --release --target "$rust_target" 2>&1 | sed 's/^/    /'
    else
        cargo build --release --target "$rust_target" 2>&1 | sed 's/^/    /'
    fi

    local src_path="target/${rust_target}/release/${src_bin}"
    if [ ! -f "$src_path" ]; then
        fail "$out_name — binary not found after build"
        FAILED+=("$out_name")
        return 1
    fi

    cp "$src_path" "${OUT_DIR}/${out_name}"
    local size
    size=$(du -sh "${OUT_DIR}/${out_name}" | cut -f1)
    ok "$out_name  ${DIM}(${size})${RESET}"
    BUILT+=("$out_name")
}

# ── Native fallback helper ─────────────────────────────────────────────────────
# Tries to build a target without cross; sets linker env var if provided.
build_native_target() {
    local rust_target="$1"
    local out_name="$2"
    local src_bin="$3"
    local linker_var="${4:-}"
    local linker_bin="${5:-}"

    if [ -n "$linker_var" ] && [ -n "$linker_bin" ]; then
        export "${linker_var}=${linker_bin}"
    fi
    build_target "$rust_target" "$out_name" "$src_bin"
    if [ -n "$linker_var" ]; then
        unset "$linker_var" 2>/dev/null || true
    fi
}

# ══════════════════════════════════════════════════════════════════════════════
# NATIVE HOST BUILD
# ══════════════════════════════════════════════════════════════════════════════
if $DO_NATIVE || [ $# -eq 0 ]; then
    hdr "Native (${HOST_OS}/${HOST_ARCH})"
    # Clean only our package so env!("CARGO_PKG_VERSION") is always re-embedded.
    # Deps are untouched, so this adds only a few seconds.
    cargo clean -p questline 2>/dev/null || true
    case "${HOST_OS}/${HOST_ARCH}" in
        Linux/x86_64)
            cargo build --release 2>&1 | sed 's/^/    /'
            cp target/release/questline "${OUT_DIR}/questline-linux-x86_64"
            ok "questline-linux-x86_64  ${DIM}(native)${RESET}"
            BUILT+=("questline-linux-x86_64")
            ;;
        Linux/aarch64)
            cargo build --release 2>&1 | sed 's/^/    /'
            cp target/release/questline "${OUT_DIR}/questline-linux-aarch64"
            ok "questline-linux-aarch64  ${DIM}(native)${RESET}"
            BUILT+=("questline-linux-aarch64")
            ;;
        Darwin/x86_64)
            cargo build --release 2>&1 | sed 's/^/    /'
            cp target/release/questline "${OUT_DIR}/questline-macos-x86_64"
            ok "questline-macos-x86_64  ${DIM}(native)${RESET}"
            BUILT+=("questline-macos-x86_64")
            ;;
        Darwin/arm64)
            cargo build --release 2>&1 | sed 's/^/    /'
            cp target/release/questline "${OUT_DIR}/questline-macos-arm64"
            ok "questline-macos-arm64  ${DIM}(native)${RESET}"
            BUILT+=("questline-macos-arm64")
            ;;
    esac
fi

# ══════════════════════════════════════════════════════════════════════════════
# LINUX TARGETS
# ══════════════════════════════════════════════════════════════════════════════
if $DO_LINUX; then
    hdr "Linux"

    # x86_64
    if [ "${HOST_OS}/${HOST_ARCH}" = "Linux/x86_64" ] && ! $DO_NATIVE; then
        # Already built natively above; don't rebuild
        skip "questline-linux-x86_64 (use --native or was built above)"
    elif $USE_CROSS || [ "${HOST_OS}/${HOST_ARCH}" = "Linux/x86_64" ]; then
        ensure_target "x86_64-unknown-linux-gnu"
        build_target "x86_64-unknown-linux-gnu" "questline-linux-x86_64" "questline" || true
    else
        skip "questline-linux-x86_64  ${DIM}(needs cross or Linux x86_64 host)${RESET}"
    fi

    # aarch64
    if [ "${HOST_OS}/${HOST_ARCH}" = "Linux/aarch64" ] && ! $DO_NATIVE; then
        skip "questline-linux-aarch64 (use --native or was built above)"
    elif $USE_CROSS; then
        ensure_target "aarch64-unknown-linux-gnu"
        build_target "aarch64-unknown-linux-gnu" "questline-linux-aarch64" "questline" || true
    elif $HAVE_AARCH64_GCC; then
        ensure_target "aarch64-unknown-linux-gnu"
        build_native_target \
            "aarch64-unknown-linux-gnu" "questline-linux-aarch64" "questline" \
            "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER" "aarch64-linux-gnu-gcc" || true
    else
        skip "questline-linux-aarch64  ${DIM}(needs cross or aarch64-linux-gnu-gcc)${RESET}"
    fi
fi

# ══════════════════════════════════════════════════════════════════════════════
# WINDOWS TARGET
# ══════════════════════════════════════════════════════════════════════════════
if $DO_WINDOWS; then
    hdr "Windows"

    if $USE_CROSS; then
        ensure_target "x86_64-pc-windows-gnu"
        build_target "x86_64-pc-windows-gnu" "questline-windows-x86_64.exe" "questline.exe" || true
    elif $HAVE_MINGW; then
        ensure_target "x86_64-pc-windows-gnu"
        build_native_target \
            "x86_64-pc-windows-gnu" "questline-windows-x86_64.exe" "questline.exe" \
            "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER" "x86_64-w64-mingw32-gcc" || true
    else
        skip "questline-windows-x86_64.exe  ${DIM}(needs cross or x86_64-w64-mingw32-gcc)${RESET}"
    fi
fi

# ══════════════════════════════════════════════════════════════════════════════
# macOS TARGETS
# ══════════════════════════════════════════════════════════════════════════════
if $DO_MACOS; then
    hdr "macOS"

    build_macos_target() {
        local rust_target="$1"
        local out_name="$2"
        local linker="$3"

        if [ "$HOST_OS" = "Darwin" ]; then
            ensure_target "$rust_target"
            build_target "$rust_target" "$out_name" "questline" || true
        elif $HAVE_OSXCROSS; then
            ensure_target "$rust_target"
            export "CARGO_TARGET_$(echo "$rust_target" | tr '[:lower:]-' '[:upper:]_')_LINKER=$linker"
            build_target "$rust_target" "$out_name" "questline" || true
        else
            skip "$out_name  ${DIM}(needs macOS host or osxcross)${RESET}"
        fi
    }

    build_macos_target \
        "x86_64-apple-darwin"   "questline-macos-x86_64" "x86_64-apple-darwin-clang"
    build_macos_target \
        "aarch64-apple-darwin"  "questline-macos-arm64"  "aarch64-apple-darwin-clang"
fi

# ══════════════════════════════════════════════════════════════════════════════
# Update version.json
# ══════════════════════════════════════════════════════════════════════════════
if $UPDATE_VERSION && [ ${#BUILT[@]} -gt 0 ]; then
    printf '{\n  "version": "%s"\n}\n' "$VERSION" > server/version.json
    ok "server/version.json → ${VERSION}"
fi

# ══════════════════════════════════════════════════════════════════════════════
# Summary
# ══════════════════════════════════════════════════════════════════════════════
echo ""
echo -e "  ${BOLD}Summary${RESET}  v${VERSION}"
echo -e "  ${DIM}────────────────────────────────────────${RESET}"

if [ ${#BUILT[@]} -gt 0 ]; then
    echo -e "  ${GREEN}Built (${#BUILT[@]}):${RESET}"
    for b in "${BUILT[@]}"; do
        echo -e "    ${DIM}${OUT_DIR}/${b}${RESET}"
    done
fi

if [ ${#FAILED[@]} -gt 0 ]; then
    echo ""
    echo -e "  ${RED}Failed (${#FAILED[@]}):${RESET}"
    for f in "${FAILED[@]}"; do
        echo -e "    ${f}"
    done
fi

# ── Setup guide for missing tools ─────────────────────────────────────────────
NEED_GUIDE=false
$USE_CROSS || NEED_GUIDE=true
$HAVE_OSXCROSS || [ "$HOST_OS" = "Darwin" ] || NEED_GUIDE=true

if $NEED_GUIDE; then
    echo ""
    echo -e "  ${YELLOW}To unlock all targets:${RESET}"
    echo ""

    if ! $USE_CROSS; then
        if ! $HAVE_RUSTUP; then
            echo -e "  ${BOLD}1. Install rustup${RESET}  ${DIM}(required by cross and for target management)${RESET}"
            echo -e "    sudo pacman -S rustup"
            echo -e "    rustup default stable"
            echo ""
        fi
        if $HAVE_CROSS && $HAVE_DOCKER && ! $HAVE_RUSTUP; then
            echo -e "  ${BOLD}2. After rustup is installed, cross will work automatically${RESET}"
            echo -e "    ${DIM}cross + Docker handles Linux/aarch64 and Windows without any extra toolchains${RESET}"
            echo ""
        elif ! $HAVE_CROSS; then
            echo -e "  ${BOLD}2. Install cross${RESET}  ${DIM}(Docker-based cross-compiler, no toolchain setup needed)${RESET}"
            echo -e "    cargo install cross"
            echo ""
        fi
        if ! $HAVE_MINGW && ! $HAVE_CROSS; then
            echo -e "  ${BOLD}Windows fallback (without cross):${RESET}"
            echo -e "    sudo pacman -S mingw-w64-gcc"
            echo -e "    rustup target add x86_64-pc-windows-gnu"
            echo ""
        fi
        if ! $HAVE_AARCH64_GCC && ! $HAVE_CROSS; then
            echo -e "  ${BOLD}Linux aarch64 fallback (without cross):${RESET}"
            echo -e "    sudo pacman -S aarch64-linux-gnu-gcc"
            echo -e "    rustup target add aarch64-unknown-linux-gnu"
            echo ""
        fi
    fi

    if ! $HAVE_OSXCROSS && [ "$HOST_OS" != "Darwin" ]; then
        echo -e "  ${BOLD}macOS targets:${RESET}  ${DIM}cross does not support macOS (Apple SDK is proprietary)${RESET}"
        echo -e "    Option A: run this script on a Mac"
        echo -e "    Option B: use GitHub Actions (macos-latest runner)"
        echo ""
    fi
fi

echo ""
