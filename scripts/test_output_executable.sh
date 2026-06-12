#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="$ROOT/example_source/extern/printf.txt"
BIN="${TMPDIR:-/tmp}/myrust_printf_from_o.$$"

cleanup() {
  rm -f "$BIN"
}
trap cleanup EXIT

cd "$ROOT"
cargo run -- "$SOURCE" -o "$BIN" >/dev/null

OUTPUT="$("$BIN")"
EXPECTED=$'answer = 42\nMore tests!'
if [ "$OUTPUT" != "$EXPECTED" ]; then
  printf 'target output test failed: expected "%s", got "%s"\n' "$EXPECTED" "$OUTPUT" >&2
  exit 1
fi

printf 'target output test passed\n'
