#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
git submodule update --init --depth 1
if git -C upstream/porffor apply --reverse --check "$(pwd)/patches/porffor-wasi.patch" 2>/dev/null; then
  : # Already applied.
else
  git -C upstream/porffor apply "$(pwd)/patches/porffor-wasi.patch"
fi
rustup toolchain install 1.99.0 --profile minimal --target wasm32-wasip2
npm ci --ignore-scripts
if [[ ! -x tools/wasi-sdk-30.0-x86_64-linux/bin/clang && -z "${WASI_SDK_PATH:-}" ]]; then
  mkdir -p tools
  curl -fL https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-30/wasi-sdk-30.0-x86_64-linux.tar.gz -o tools/wasi-sdk.tar.gz
  tar -xzf tools/wasi-sdk.tar.gz -C tools
  rm tools/wasi-sdk.tar.gz
fi
if ! command -v wasm-tools >/dev/null; then
  cargo install --locked wasm-tools --version 1.261.0
fi
cargo fetch --locked
