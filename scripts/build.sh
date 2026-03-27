#!/usr/bin/env bash
set -euo pipefail

TAG="${1:-dev}"
BINARY_NAME="gh-issue-graph"

mkdir -p dist

build_target() {
    local TARGET="$1"
    local OS="$2"
    local ARCH="$3"
    local EXT="${4:-}"

    echo "Building for $TARGET ($OS-$ARCH)..."

    case "$TARGET" in
        aarch64-unknown-linux-gnu)
            apt-get install -y gcc-aarch64-linux-gnu 2>/dev/null || true
            ;;
    esac

    cargo build --release --target "$TARGET" 2>&1

    local SRC="target/$TARGET/release/$BINARY_NAME$EXT"
    local DST="dist/${BINARY_NAME}_${TAG}_${OS}-${ARCH}${EXT}"

    cp "$SRC" "$DST"
    echo "  -> $DST"
}

# gh-extension-precompile から呼ばれる場合: CARGO_BUILD_TARGET が設定されている
if [[ -n "${CARGO_BUILD_TARGET:-}" ]]; then
    TARGET="$CARGO_BUILD_TARGET"
    case "$TARGET" in
        x86_64-apple-darwin)      build_target "$TARGET" darwin  amd64 ;;
        aarch64-apple-darwin)     build_target "$TARGET" darwin  arm64 ;;
        x86_64-unknown-linux-gnu) build_target "$TARGET" linux   amd64 ;;
        aarch64-unknown-linux-gnu)build_target "$TARGET" linux   arm64 ;;
        x86_64-pc-windows-gnu)    build_target "$TARGET" windows amd64 .exe ;;
        x86_64-pc-windows-msvc)   build_target "$TARGET" windows amd64 .exe ;;
        *)
            echo "Unknown target: $TARGET"
            exit 1
            ;;
    esac
else
    # ローカルビルド (全ターゲット)
    rustup target add \
        x86_64-apple-darwin \
        aarch64-apple-darwin \
        x86_64-unknown-linux-gnu \
        aarch64-unknown-linux-gnu \
        2>/dev/null || true

    build_target x86_64-apple-darwin      darwin  amd64
    build_target aarch64-apple-darwin     darwin  arm64
    build_target x86_64-unknown-linux-gnu linux   amd64
    build_target aarch64-unknown-linux-gnu linux  arm64

    echo "Build complete. Artifacts in dist/:"
    ls -lh dist/
fi
