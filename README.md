# PT — Prompt Tuner

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A Claude Code hook that catches vague prompts and suggests better versions before they waste cycles.

## How it works

PT runs as a `UserPromptSubmit` hook. Every prompt goes through a fast heuristic check (no API calls). If the prompt lacks actionable content — no file paths, no code references, no error messages, no technical detail — it blocks the submission and suggests a rewrite using Haiku.

Specific prompts, commands, and conversational responses pass through with zero overhead.

## Install

```sh
cargo build --release
```

Requires [Claude Code CLI](https://docs.anthropic.com/claude-code) installed and authenticated.

## Setup

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "type": "command",
        "command": "/absolute/path/to/pt --hook"
      }
    ]
  }
}
```

## Usage

```sh
pt --test "your prompt"   # Test detection without the hook
pt --disable              # Turn off until re-enabled
pt --enable               # Turn back on
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

## License

MIT
