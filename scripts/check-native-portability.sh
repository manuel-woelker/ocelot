#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
native_check_dir="$(mktemp -d)"
trap 'rm -rf -- "$native_check_dir"' EXIT
native_cc="${NATIVE_CC:-clang}"

"$project_root/target/debug/ocelot" build "$project_root/examples/native-smoke.ocelot-script" \
  --emit=c -o "$native_check_dir/program.c"
"$native_cc" -std=c99 -O2 -Wall -Wextra -Werror \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  "$native_check_dir/program.c" "$project_root/target/debug/libocelot_native_runtime.a" \
  -lpthread -ldl -lm -o "$native_check_dir/program"
ASAN_OPTIONS=detect_leaks=1 "$native_check_dir/program" > "$native_check_dir/output"
printf 'hello native\nhello\n' > "$native_check_dir/expected"
cmp "$native_check_dir/expected" "$native_check_dir/output"
printf 'Portable C and sanitizer smoke test passed.\n'
