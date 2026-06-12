#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKDIR="${TMPDIR:-/tmp}/my_rust_compiler_c_interface.$$"
mkdir -p "$WORKDIR"
trap 'rm -rf "$WORKDIR"' EXIT

find_tool() {
  local name="$1"
  if command -v "$name" >/dev/null 2>&1; then
    command -v "$name"
    return
  fi

  local homebrew_tool="/opt/homebrew/opt/llvm/bin/$name"
  if [[ -x "$homebrew_tool" ]]; then
    printf '%s\n' "$homebrew_tool"
    return
  fi

  printf 'missing required tool: %s\n' "$name" >&2
  exit 1
}

CLANG="$(find_tool clang)"

cat >"$WORKDIR/entry.txt" <<'SRC'
fn main() -> i32 {
    return 7;
}
SRC

(
  cd "$ROOT"
  cargo run -- "$WORKDIR/entry.txt" -ll >/dev/null
)

"$CLANG" "$WORKDIR/entry.ll" -o "$WORKDIR/entry"
set +e
"$WORKDIR/entry"
ENTRY_STATUS=$?
set -e
if [[ "$ENTRY_STATUS" -ne 7 ]]; then
  printf 'entry test failed: expected exit code 7, got %s\n' "$ENTRY_STATUS" >&2
  exit 1
fi

cat >"$WORKDIR/lib.txt" <<'SRC'
fn add(a:i32, b:i32) -> i32 {
    return a + b;
}

fn inc_ref(p:&mut i32) -> i32 {
    *p = *p + 1;
    return *p;
}
SRC

cat >"$WORKDIR/driver.c" <<'SRC'
#include <stdio.h>

extern int add(int a, int b);
extern int inc_ref(int *p);

int main(void) {
    int value = 41;
    int sum = add(20, 22);
    int updated = inc_ref(&value);
    printf("%d %d %d\n", sum, updated, value);
    return (sum == 42 && updated == 42 && value == 42) ? 0 : 1;
}
SRC

(
  cd "$ROOT"
  cargo run -- "$WORKDIR/lib.txt" -ll >/dev/null
)

"$CLANG" "$WORKDIR/lib.ll" "$WORKDIR/driver.c" -o "$WORKDIR/lib_driver"
OUTPUT="$("$WORKDIR/lib_driver")"
if [[ "$OUTPUT" != "42 42 42" ]]; then
  printf 'C interface test failed: expected "42 42 42", got "%s"\n' "$OUTPUT" >&2
  exit 1
fi

printf 'entry and C interface tests passed\n'
