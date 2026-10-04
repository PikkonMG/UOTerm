#!/usr/bin/env bash
# Builds the crates the web client runs for the browser target. A crate
# that reaches a file, a thread or the wall clock fails here, not in the
# browser.
set -euo pipefail
cd "$(dirname "$0")/.."
for crate in "$@"; do
  cargo build -p "$crate" --target wasm32-unknown-unknown
done
