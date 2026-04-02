# PT — Prompt Tuner

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A TUI for refining prompts before sending them to Claude.

## Install

```sh
cargo install pt
# or
cargo build --release
```

Requires [Claude Code CLI](https://docs.anthropic.com/claude-code) installed and authenticated.

## Keybinds

| Key | Action |
|-----|--------|
| `Ctrl+R` | Refine prompt |
| `Ctrl+W` | Refine + explain why |
| `Ctrl+Y` | Copy refined output to clipboard |
| `Ctrl+L` | Clear both panes |
| `Tab` | Toggle focus between panes |
| `Ctrl+C` | Quit |

## License

MIT
