# zjbar

A Zellij status bar plugin that replaces the default tab bar with Claude Code activity awareness and a Catppuccin Mocha powerline theme.

## Features

- **Powerline tab bar** — Catppuccin Mocha themed tab bar with sharp powerline arrows between segments
- **Session & mode display** — shows session name and input mode (NORMAL, LOCKED, PANE, etc.) with color-coded pills
- **Live Claude activity indicators** — see what every Claude Code session is doing at a glance
- **Clickable tabs** — click any tab to switch; clicking a waiting (⚠) session focuses the exact pane
- **Permission flash** — tabs pulse yellow when a permission request arrives
- **Desktop notifications** — macOS/Linux notification on permission requests with click-to-focus support
- **Elapsed time** — shows how long a session has been in its current state
- **Multi-instance sync** — all Zellij tabs show a unified view of all Claude sessions

### Activity Symbols

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

## Install

### Prerequisites

- [Zellij](https://zellij.dev)
- [jq](https://jqlang.github.io/jq/) — used by the hook script at runtime

### Quick install

Add the plugin to your Zellij layout — that's it:

```kdl
default_tab_template {
    children
    pane size=1 borderless=true {
        plugin location="https://github.com/imroc/zjbar/releases/latest/download/zjbar.wasm"
    }
}
```

On first load, the plugin automatically installs the hook script and registers it with Claude Code.

### Build from source

Prerequisites: [Rust](https://rustup.rs)

```bash
git clone https://github.com/imroc/zjbar.git
cd zjbar
./install.sh
```

Or use make targets directly:

```bash
make            # build wasm + update plugin
make install    # build + install layouts + register hooks
make uninstall  # remove plugin, layouts and hooks
make release    # create GitHub release (requires tag on HEAD)
```

If your Claude Code compatible tool stores settings in a different location (e.g. `~/.codebuddy/settings.json`), specify it via `CLAUDE_SETTINGS`:

```bash
CLAUDE_SETTINGS=~/.codebuddy/settings.json make install
```

### Optional: click-to-focus notifications

```bash
brew install terminal-notifier
```

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
2. **Hook script** — bash bridge forwarding Claude Code events via `zellij pipe`

```
Claude Code hook → zjbar-hook.sh → zellij pipe → plugin → render
```

## Uninstall

```bash
make uninstall
```

## License

MIT
