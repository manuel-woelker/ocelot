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
  "$native_check_dir/program.c" -o "$native_check_dir/program"
ASAN_OPTIONS=detect_leaks=1 "$native_check_dir/program" > "$native_check_dir/output"
printf 'hello native\nhello\n' > "$native_check_dir/expected"
cmp "$native_check_dir/expected" "$native_check_dir/output"

# Reuse the emitted runtime for direct handle, byte, and overflow checks.
sed '/^int main(/,$d' "$native_check_dir/program.c" > "$native_check_dir/runtime-test.c"
cat "$project_root/crates/engine/src/native_runtime_tests.c" >> "$native_check_dir/runtime-test.c"
"$native_cc" -std=c99 -O2 -Wall -Wextra -Werror \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  -DOCELOT_TEST_OVERFLOW_BUILTINS "$native_check_dir/runtime-test.c" -o "$native_check_dir/runtime-test"
ASAN_OPTIONS=detect_leaks=1 "$native_check_dir/runtime-test"
set +e
ASAN_OPTIONS=detect_leaks=1 "$native_check_dir/runtime-test" --overflow 2> "$native_check_dir/overflow-error"
overflow_status=$?
set -e
test "$overflow_status" -eq 102
grep -Fq 'string size overflow' "$native_check_dir/overflow-error"
for operation in add subtract multiply divide zero negate; do
  set +e
  ASAN_OPTIONS=detect_leaks=1 "$native_check_dir/runtime-test" "--int-$operation" 2> "$native_check_dir/integer-error"
  integer_status=$?
  set -e
  test "$integer_status" -eq 101
  grep -Fxq 'checked integer error' "$native_check_dir/integer-error"
done
printf 'Portable C and sanitizer smoke test passed.\n'
