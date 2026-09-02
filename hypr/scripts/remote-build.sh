#!/usr/bin/env bash
# ==============================================================================
# Remote Build Script for the rustbar / control-center workspace
# Offloads compilation to a fast remote server via SSH.
#
# Usage: ./remote-build.sh [rustbar|control-center]   (default: rustbar)
# ==============================================================================

set -euo pipefail

# Configuration
REMOTE_HOST="kams@lxhalle.in.tum.de"
REMOTE_DIR="~/remote_builds/hypr-shell"
LOCAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_NAME="${1:-rustbar}"

case "$BINARY_NAME" in
    rustbar|control-center) ;;
    *)
        echo "Unknown binary '$BINARY_NAME' (expected: rustbar or control-center)" >&2
        exit 1
        ;;
esac

echo "=== 📦 Packaging and Syncing Workspace ==="
# Ensure remote directory exists
ssh "$REMOTE_HOST" "mkdir -p $REMOTE_DIR"

# Send the whole workspace (both members depend on shell-common), excluding
# target/ and the sibling shell scripts that aren't part of the build.
tar -czf - -C "$LOCAL_DIR" --exclude=target \
    Cargo.toml Cargo.lock shell-common rustbar control-center \
    | ssh "$REMOTE_HOST" "tar -xzf - -C $REMOTE_DIR"

echo "=== ⚙️  Compiling '$BINARY_NAME' on Remote Host ==="
# Sourcing cargo environment, setting custom PKG_CONFIG_PATH, and building in release mode
ssh "$REMOTE_HOST" "
    . ~/.cargo/env &&
    export PKG_CONFIG_PATH=\$HOME/.local/lib/x86_64-linux-gnu/pkgconfig &&
    cd $REMOTE_DIR &&
    cargo build --release -p $BINARY_NAME
"

echo "=== 📥 Copying Binary Back ==="
# Ensure the workspace target/release directory exists locally
mkdir -p "$LOCAL_DIR/target/release"

# Stream the binary back to the local target path
ssh "$REMOTE_HOST" "cat $REMOTE_DIR/target/release/$BINARY_NAME" > "$LOCAL_DIR/target/release/$BINARY_NAME"
chmod +x "$LOCAL_DIR/target/release/$BINARY_NAME"

echo "=== 🎉 Success! Binary built remotely and saved to: ==="
echo "    $LOCAL_DIR/target/release/$BINARY_NAME"
