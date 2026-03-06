# zjbar Development Guide

## Overview

zjbar is a Zellij WASM plugin that replaces the default tab bar with a Tokyo Night powerline-themed status bar featuring Claude Code activity awareness.

## Architecture

```
src/
├── main.rs           # Plugin entry point (ZellijPlugin trait impl), event routing, state management
├── render.rs         # Status bar rendering with ANSI escape codes and powerline arrows
├── state.rs          # State types: Activity, SessionInfo, Settings, HookPayload, etc.
├── event_handler.rs  # Maps Claude Code hook events to Activity states
├── installer.rs      # Auto-installs hook script and registers Claude Code hooks
└── tab_pane_map.rs   # Maps pane IDs to (tab_index, tab_name) pairs
scripts/
├── zjbar-hook.sh     # Claude Code hook → zellij pipe bridge (embedded in WASM via include_str!)
└── install-hooks.sh  # Standalone hook installer (used by install.sh, not the plugin itself)
```

## Build & Test

```bash
# Build WASM plugin
cargo build --release --target wasm32-wasip1

# Install to zellij plugins directory
cp target/wasm32-wasip1/release/zjbar.wasm ~/.config/zellij/plugins/

# Test with a layout
zellij --layout layout.kdl
```

## Key Concepts

- **Rendering**: `render.rs` outputs raw ANSI escape codes via `print!()` in the `render()` method. Zellij captures stdout as pane content.
- **IPC**: Claude Code hooks → `zjbar-hook.sh` → `zellij pipe --name zjbar` → plugin's `pipe()` method.
- **Multi-instance sync**: Each tab has its own plugin instance. They sync state via `pipe_message_to_plugin()` with names like `zjbar:sync`, `zjbar:request`, `zjbar:settings`.
- **Auto-install**: On first load (after permissions granted), the plugin writes `zjbar-hook.sh` to `~/.config/zellij/plugins/` and registers it in `~/.claude/settings.json`.
- **Settings**: Persisted as JSON to `~/.config/zellij/plugins/zjbar.json`.

## Conventions

- All commit messages and code comments must be in **English**.
- The WASM target is `wasm32-wasip1` (configured in `.cargo/config.toml`).
- Release profile uses `opt-level = "s"` and LTO for minimal binary size.
- Color palette follows Tokyo Night. All color constants are defined in `render.rs`.
