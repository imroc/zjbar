---
description: Download zjbar WASM plugin and install Zellij layouts
allowed-tools: Bash
---

# Install zjbar Zellij Plugin

Download the zjbar WASM plugin from GitHub Releases and install Zellij layouts.

## Install

```bash
set -e

PLUGIN_DIR="$HOME/.config/zellij/plugins"
LAYOUT_DIR="$HOME/.config/zellij/layouts"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-}"

# Get latest release download URL
WASM_URL="https://github.com/imroc/zjbar/releases/latest/download/zjbar.wasm"

# Install WASM plugin
mkdir -p "$PLUGIN_DIR"
echo "Downloading zjbar.wasm..."
if curl -fsSL "$WASM_URL" -o "$PLUGIN_DIR/zjbar.wasm"; then
  echo "Installed: $PLUGIN_DIR/zjbar.wasm"
else
  echo "Error: failed to download zjbar.wasm" >&2
  exit 1
fi

# Install layout files from plugin source
if [ -n "$PLUGIN_ROOT" ] && [ -f "$PLUGIN_ROOT/layout.kdl" ]; then
  mkdir -p "$LAYOUT_DIR"
  cp "$PLUGIN_ROOT/layout.kdl" "$LAYOUT_DIR/zjbar.kdl"
  cp "$PLUGIN_ROOT/layout.swap.kdl" "$LAYOUT_DIR/zjbar.swap.kdl"
  echo "Installed layouts: $LAYOUT_DIR/zjbar.kdl"
else
  echo "Skipped layout install (plugin source not found)"
fi

echo ""
echo "Done! Start zellij with: zellij --layout zjbar"
```
