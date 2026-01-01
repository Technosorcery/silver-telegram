#!/bin/bash
# Install build dependencies for silver-telegram
# This script is run by Claude Code's SessionStart hook in remote sessions.

set -e

# Only run in Claude Code remote sessions
if [ "$CLAUDE_CODE_REMOTE" != "true" ]; then
    exit 0
fi

echo "Installing silver-telegram build dependencies..."

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install cargo-leptos
cargo install cargo-leptos

# Install wasm-bindgen-cli (version must match wasm-bindgen crate)
cargo install wasm-bindgen-cli@0.2.106

echo "Build dependencies installed successfully."
