#!/usr/bin/env bash
# install.sh — Bootstrap installer for first-time setup
#
# Checks prerequisites (jq, cargo, wasm target) then delegates to make.
#
# Usage:
#   ./install.sh            # install everything
#   ./install.sh --uninstall # remove everything
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

# ── Uninstall ──────────────────────────────────────────────

if [ "${1:-}" = "--uninstall" ]; then
    make -C "$PROJECT_DIR" uninstall
    exit 0
fi

# ── Prerequisites ──────────────────────────────────────────

missing=()
command -v jq    &>/dev/null || missing+=(jq)
command -v cargo &>/dev/null || {
    export PATH="$HOME/.cargo/bin:$PATH"
    command -v cargo &>/dev/null || missing+=(rust/cargo)
}

if [ ${#missing[@]} -gt 0 ]; then
    red "Missing: ${missing[*]}"
    echo "Install with:"
    for dep in "${missing[@]}"; do
        case "$dep" in
            jq)          echo "  brew install jq" ;;
            rust/cargo)  echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" ;;
        esac
    done
    exit 1
fi

if ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip1; then
    echo "Adding wasm32-wasip1 target..."
    rustup target add wasm32-wasip1
fi

# ── Install ────────────────────────────────────────────────

make -C "$PROJECT_DIR" install

green ""
green "Installed! Start with: zellij --layout $PROJECT_DIR/layout.kdl"
