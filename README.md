# zjbar

A Zellij status bar plugin with a Tokyo Night powerline theme and optional Claude Code activity awareness.

## Features

- **Powerline tab bar** — Tokyo Night themed tab bar with sharp powerline arrows between segments
- **Session & mode display** — shows session name and input mode (NORMAL, LOCKED, PANE, etc.) with color-coded pills
- **Clickable tabs** — click any tab to switch
- **Optional Claude Code integration** — live activity indicators, permission flash, desktop notifications, and click-to-focus
- **Multi-instance sync** — all Zellij tabs show a unified view of all Claude sessions

## Install

### Prerequisites

- [Zellij](https://zellij.dev)

### Option 1: Claude Code plugin (recommended)

Install as a Claude Code plugin to get automatic hook registration and a one-command setup:

```
/plugin marketplace add imroc/zjbar
/plugin install zjbar@zjbar
```

Then download the WASM plugin and layouts:

```
/zjbar:install
```

Restart Claude Code for hooks to take effect, then start Zellij:

```bash
zellij --layout zjbar
```

### Option 2: Zellij layout only

Add the plugin to your Zellij layout directly (no Claude Code integration):

```kdl
default_tab_template {
    children
    pane size=1 borderless=true {
        plugin location="https://github.com/imroc/zjbar/releases/latest/download/zjbar.wasm"
    }
}
```

### Option 3: Build from source

Prerequisites: [Rust](https://rustup.rs), [jq](https://jqlang.github.io/jq/) (for hooks)

```bash
git clone https://github.com/imroc/zjbar.git
cd zjbar
./install.sh
```

Or use make targets directly:

```bash
make               # build wasm + update plugin
make install       # build + install layouts
make install-hooks # register Claude Code hooks
make uninstall     # remove plugin and layouts
make release       # create GitHub release (requires tag on HEAD)
```

The hook installer auto-detects the settings path (`~/.claude-internal/settings.json` or `~/.claude/settings.json`). To specify a custom path:

```bash
CLAUDE_SETTINGS=~/.codebuddy/settings.json make install-hooks
```

### Optional: click-to-focus notifications

```bash
brew install terminal-notifier
```

## Claude Code Activity Symbols

| Symbol | Meaning                   |
| ------ | ------------------------- |
| ◆      | Session starting          |
| ●      | Thinking                  |
| ⚡     | Running Bash              |
| ◉      | Reading / searching files |
| ✎      | Editing / writing files   |
| ⊜      | Spawning subagent         |
| ◈      | Web search / fetch        |
| ⚙      | Other tool                |
| ▶      | Waiting for user prompt   |
| ⚠      | Waiting for permission    |
| ✓      | Done                      |

## Settings

Click the session/mode prefix on the left side of the bar to open the settings menu.

| Setting       | Options                  | Default |
| ------------- | ------------------------ | ------- |
| Notifications | Always / Unfocused / Off | Always  |
| Flash         | Persist / Brief / Off    | Brief   |
| Elapsed time  | On / Off                 | On      |

Settings are persisted to `~/.config/zellij/plugins/zjbar.json`.

## How It Works

1. **WASM plugin** — runs inside Zellij, renders the status bar, manages state
2. **Hook script** (optional) — bash bridge forwarding Claude Code events via `zellij pipe`

```
Claude Code hook → zjbar-hook.sh → zellij pipe → plugin → render
```

## Uninstall

```bash
make uninstall
```

Or if installed as a Claude Code plugin:

```
/plugin uninstall zjbar@zjbar
```

## License

MIT
