# DR-021: Use a C runtime in generated native programs

- Status: Accepted
- Date: 2026-11-11
- Supersedes: [DR-020](<DR-020 Generate C with replaceable native toolchains.md>)
  for the runtime and packaging choices; C emission and replaceable toolchains remain.

## What is the decision?

Implement the native runtime in C99 and include its source with the generated
translation unit. Remove the Rust runtime crate, static archive lookup, and
`--runtime` option. Native compilation must neither link Rust libraries nor
require a separately installed runtime artifact.

Keep the existing unboxed tag/payload ABI, borrowed parameters, owned string
results, explicit cleanup, and versioned spec reports. Strings use a C allocation
with a reference count, byte length, and immutable contents, including embedded
NUL bytes. Reference counts are non-atomic because native execution currently
has no threading. Check allocation arithmetic and reference count overflow.

Keep runtime libc I/O behind a focused C boundary. The Rust compiler continues
using PAL; generated binaries do not link the Rust PAL. Retain the explicit TCC
compiler/system linker adapter and its tested object compatibility flags.

## Why was this chosen?

The user explicitly requested a C runtime after examining a 45 MB debug
hello-world executable. It inherited Rust dependency-tree debug information from
the runtime's base/PAL dependencies. Stripping hid that symptom but left an
unnecessary runtime architecture and distribution dependency.

A self-contained C translation unit is inspectable, works with other C compilers,
and removes archive version/profile/target lookup from native builds. The runtime
needs strings, display, assertions, and synchronous I/O, not compiler dependencies.

## What is the context?

[DR-020](<DR-020 Generate C with replaceable native toolchains.md>) established
C emission with TCC, initially with a Rust runtime. That implementation is
preserved in repository history; this decision changes its runtime choice rather
than rewriting that history.

- [Runtime API](../../crates/codegen_c/src/native_api.c)
- [String storage](../../crates/codegen_c/src/native_string.h)
- [Runtime I/O boundary](../../crates/codegen_c/src/native_io.c)
- [Direct C contract tests](../../crates/engine/src/native_runtime_tests.c)
- [Native guide](../NATIVE.md)

## What are the consequences?

- Native executables link only the C/system libraries their toolchain requires.
- Emitted C contains its runtime; packaging ships only the compiler and instructions.
- Runtime memory safety is now a C implementation responsibility. Native spec
  parity, ownership/byte/overflow tests, ASan/UBSan/leak checks, and a minimal-binary
  size budget are required regression coverage.
- Assertion diagnostics remain rendered by the Rust frontend and embedded as
  data, without placing that renderer in the generated executable.
- Adding concurrency would require reconsidering reference counts and report state.
- Rust interpretation and its runtime values remain unchanged.

## What alternatives were considered?

- **Strip the Rust runtime:** Reduces disk size but retains unwanted runtime and
  archive/toolchain coupling; does not satisfy the requested architecture.
- **Create a smaller Rust runtime crate:** Still adds a Rust runtime distribution
  dependency to otherwise ordinary generated C.
- **Ship a prebuilt C archive:** Avoids Rust but keeps target/profile/runtime lookup
  and packaging complexity unnecessary for this small runtime.
- **Use process-lifetime allocations:** Simpler initially but leaks temporary
  strings; retain explicit ownership and cleanup instead.
