#!/bin/sh
set -e

REPO="https://github.com/JGabrine/pt.git"
INSTALL_DIR="$HOME/.local/share/pt"
BIN_DIR="$HOME/.local/bin"

echo "Installing Prompt Tuner..."

# Check dependencies
if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: cargo not found. Install Rust first: https://rustup.rs"
    exit 1
fi

if ! command -v claude >/dev/null 2>&1; then
    echo "Warning: Claude Code CLI not found. Install from https://docs.anthropic.com/claude-code"
fi

# Clone or update
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing installation..."
    git -C "$INSTALL_DIR" pull --ff-only
else
    echo "Cloning repository..."
    git clone "$REPO" "$INSTALL_DIR"
fi

# Build
echo "Building..."
cargo build --release --manifest-path "$INSTALL_DIR/Cargo.toml"

# Install binary
mkdir -p "$BIN_DIR"
cp "$INSTALL_DIR/target/release/pt" "$BIN_DIR/pt"

# Check PATH
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        echo ""
        echo "Note: $BIN_DIR is not in your PATH."
        echo "Add it with: export PATH=\"$BIN_DIR:\$PATH\""
        echo ""
        ;;
esac

# Register hook
"$BIN_DIR/pt" --setup

echo ""
echo "Done. Restart Claude Code to activate."
