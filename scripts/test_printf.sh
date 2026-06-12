#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="$ROOT/example_source/extern/printf.txt"
LL="$ROOT/example_source/extern/printf.ll"
BIN="${TMPDIR:-/tmp}/myrust_printf"

find_tool() {
  local name="$1"
  if command -v "$name" >/dev/null 2>&1; then
    command -v "$name"
    return 0
  fi

  local homebrew_tool="/opt/homebrew/bin/$name"
  if [ -x "$homebrew_tool" ]; then
    printf '%s\n' "$homebrew_tool"
    return 0
  fi

  printf 'missing required tool: %s\n' "$name" >&2
  return 1
}

CLANG="$(find_tool clang)"

cd "$ROOT"
cargo run -- "$SOURCE" -ll >/dev/null
"$CLANG" "$LL" -o "$BIN"

OUTPUT="$("$BIN")"
EXPECTED=$'answer = 42\nMore tests!'
if [ "$OUTPUT" != "$EXPECTED" ]; then
  printf 'printf test failed: expected "%s", got "%s"\n' "$EXPECTED" "$OUTPUT" >&2
  exit 1
fi

printf 'printf test passed\n'
