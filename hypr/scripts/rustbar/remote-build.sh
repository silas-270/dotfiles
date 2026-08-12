#!/usr/bin/env bash
# ==============================================================================
# Remote Build Script for rustbar
# Offloads the compilation of rustbar to a fast remote server via SSH.
# ==============================================================================

set -euo pipefail

# Configuration
REMOTE_HOST="kams@lxhalle.in.tum.de"
REMOTE_DIR="~/remote_builds/rustbar"
LOCAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_NAME="rustbar"

echo "=== 📦 Packaging and Syncing Source Files ==="
# Ensure remote directory exists
ssh "$REMOTE_HOST" "mkdir -p $REMOTE_DIR"

# Send local directory excluding target/ folder to the remote host
tar -czf - -C "$LOCAL_DIR" --exclude=target . | ssh "$REMOTE_HOST" "tar -xzf - -C $REMOTE_DIR"

echo "=== ⚙️  Compiling on Remote Host ==="
# Sourcing cargo environment, setting custom PKG_CONFIG_PATH, and building in release mode
ssh "$REMOTE_HOST" "
    . ~/.cargo/env &&
    export PKG_CONFIG_PATH=\$HOME/.local/lib/x86_64-linux-gnu/pkgconfig &&
    cd $REMOTE_DIR &&
    cargo build --release
"

echo "=== 📥 Copying Binary Back ==="
# Ensure target/release directory exists locally
mkdir -p "$LOCAL_DIR/target/release"

# Stream the binary back to the local target path
ssh "$REMOTE_HOST" "cat $REMOTE_DIR/target/release/$BINARY_NAME" > "$LOCAL_DIR/target/release/$BINARY_NAME"
chmod +x "$LOCAL_DIR/target/release/$BINARY_NAME"

echo "=== 🎉 Success! Binary built remotely and saved to: ==="
echo "    $LOCAL_DIR/target/release/$BINARY_NAME"
