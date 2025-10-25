#!/bin/bash

set -e

BIN_NAME="owl-o-meter"
DIST_DIR="dist"

echo "Adicionando targets necessários..."
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

export CROSS_CONTAINER_ENGINE=podman
export CROSS_SKIP_VERSION_CHECK=true
export CROSS_CONTAINER_OPTS=--platform=linux/amd64

echo "Compilando para Linux (x86_64)..."
cross build --release --target x86_64-unknown-linux-gnu

echo "Compilando para Windows (x86_64)..."
cross build --release --target x86_64-pc-windows-gnu

echo "Compilando para macOS (Intel)..."
cargo build --release --target x86_64-apple-darwin

echo "Compilando para macOS (Apple Silicon)..."
cargo build --release --target aarch64-apple-darwin

echo "Copiando binários para '$DIST_DIR'..."

cp "target/x86_64-unknown-linux-gnu/release/$BIN_NAME"         "$DIST_DIR/${BIN_NAME}-linux"
cp "target/x86_64-pc-windows-gnu/release/${BIN_NAME}.exe"      "$DIST_DIR/${BIN_NAME}-windows.exe"
cp "target/x86_64-apple-darwin/release/$BIN_NAME"              "$DIST_DIR/${BIN_NAME}-macos-x64"
cp "target/aarch64-apple-darwin/release/$BIN_NAME"             "$DIST_DIR/${BIN_NAME}-macos-arm64"


if command -v lipo &> /dev/null; then
    echo "Gerando binário universal para macOS..."
    lipo -create \
        "target/x86_64-apple-darwin/release/$BIN_NAME" \
        "target/aarch64-apple-darwin/release/$BIN_NAME" \
        -output "$DIST_DIR/${BIN_NAME}-macos-universal"
fi

echo "Binários gerados com sucesso:"
ls -lh "$DIST_DIR"
