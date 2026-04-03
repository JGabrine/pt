# PT — Prompt Tuner

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A Claude Code hook that catches vague prompts and suggests better versions before they waste cycles.

> **Disclaimer:** This is a personal tool made public for convenience. No support, issues, or PRs will be handled beyond what's in this README.
>
> - The author takes **no responsibility** for how this tool is used, how it may affect or alter Claude's behavior, or any API costs incurred by its operation.
> - **Data sent to Claude:** When a vague prompt is detected, your prompt text, current working directory path, and recent conversation history are sent to Claude Haiku for rewrite suggestions. Be aware of this if you work with sensitive or proprietary code.
> - **Cost:** Each vague prompt triggers a Haiku API call billed to your Anthropic account. Calls are small but not free.
> - **Failure behavior:** If Claude CLI is unavailable or the rewrite times out (15s), the prompt passes through unmodified.
> - By installing this tool, you accept full responsibility for its effects on your workflow, API usage, and costs.

## How it works

PT runs as a `UserPromptSubmit` hook. Every prompt goes through a fast heuristic check (no API calls). If the prompt lacks actionable content — no file paths, no code references, no error messages, no technical detail — it blocks the submission and suggests a rewrite using Haiku.

Specific prompts, commands, and conversational responses pass through with zero overhead.

## Before you install

- **What it modifies:** `--setup` writes to `~/.claude/settings.json` and copies the binary to `~/.local/bin/pt` (Linux/macOS) or `%LOCALAPPDATA%\pt\` (Windows). It does not modify any other files.
- **Requires:** [Rust toolchain](https://rustup.rs) (to build) and [Claude Code CLI](https://docs.anthropic.com/claude-code) (installed and authenticated).

## Install

**Linux / macOS:**

```sh
curl -fsSL https://raw.githubusercontent.com/JGabrine/pt/main/install.sh | sh
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/JGabrine/pt/main/install.ps1 | iex
```

Or manually:

```sh
cargo install --git https://github.com/JGabrine/pt
pt --setup
```

Requires [Claude Code CLI](https://docs.anthropic.com/claude-code) installed and authenticated.

## Usage

```sh
pt --test "your prompt"   # Test detection without the hook
pt --disable              # Turn off until re-enabled
pt --enable               # Turn back on
pt --setup                # Register hook in Claude Code settings
pt --uninstall            # Remove hook from Claude Code settings
pt --update               # Pull latest and rebuild
pt                        # TUI mode (standalone interactive refinement)
```

## Examples

```
$ pt --test "fix the bug"
BLOCK (score: 13)

$ pt --test "fix the null pointer in src/auth.rs:45"
ALLOW (score: -3)

$ pt --test "run the tests"
ALLOW (score: -1)

$ pt --test "yes"
ALLOW (score: 0)
```

## Detection

Instead of pattern-matching vague phrases (infinite, language-dependent), PT detects **specificity**. A prompt is vague if:

1. It's short (15 words or less)
2. It contains nothing actionable (no file paths, code references, error messages, or technical nouns)

Conversational responses and clear commands are always exempt.

## Uninstall

```sh
pt --uninstall   # Remove hook from Claude Code settings
```

To fully remove the binary and repo:

```sh
# Linux / macOS
rm ~/.local/bin/pt
rm -rf ~/.local/share/pt
```

```powershell
# Windows
Remove-Item "$env:LOCALAPPDATA\pt" -Recurse -Force
```

## License

MIT
