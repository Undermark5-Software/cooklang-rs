#!/bin/bash
#
# Runs the Swift package tests (swift/Tests) against the *current* Rust sources.
#
# The committed swift/Sources/CooklangParser/CooklangParser.swift is paired with the
# released xcframework, and UniFFI checks API checksums at load time, so testing local
# changes needs both halves rebuilt. This script:
#
#   1. builds the host (macOS) static library,
#   2. regenerates the Swift wrapper from it,
#   3. wraps the library in a local, macOS-only CooklangParserFFI.xcframework,
#   4. runs `swift test` with USE_LOCAL_XCFRAMEWORK=1 (see Package.swift),
#   5. restores the committed wrapper (set KEEP_WRAPPER=1 to keep the regenerated one).
#
# macOS only (needs xcodebuild and the Swift toolchain).

set -euo pipefail

cd "$(dirname "$0")"
REPO_ROOT="$(cd .. && pwd)"

OUT_PATH="out/swift-test"
XCFRAMEWORK="out/CooklangParserFFI.xcframework"
WRAPPER="$REPO_ROOT/swift/Sources/CooklangParser/CooklangParser.swift"

echo "Building host library..."
cargo build --lib

echo "Generating Swift wrapper..."
rm -rf "$OUT_PATH"
mkdir -p "$OUT_PATH/headers"
cargo run --quiet --features=uniffi/cli --bin uniffi-bindgen -- generate \
  --library ../target/debug/libcooklang_bindings.dylib \
  --language swift \
  --out-dir "$OUT_PATH"

cp "$OUT_PATH/CooklangParserFFI.h" "$OUT_PATH/headers/"
cp "$OUT_PATH/CooklangParserFFI.modulemap" "$OUT_PATH/headers/module.modulemap"

echo "Creating local xcframework..."
rm -rf "$XCFRAMEWORK"
xcodebuild -create-xcframework \
  -library ../target/debug/libcooklang_bindings.a \
  -headers "$OUT_PATH/headers" \
  -output "$XCFRAMEWORK"

if [ -z "${KEEP_WRAPPER:-}" ]; then
  BACKUP="$(mktemp)"
  cp "$WRAPPER" "$BACKUP"
  trap 'cp "$BACKUP" "$WRAPPER"; rm -f "$BACKUP"' EXIT
fi
cp "$OUT_PATH/CooklangParser.swift" "$WRAPPER"

echo "Running swift test..."
cd "$REPO_ROOT"
USE_LOCAL_XCFRAMEWORK=1 swift test
