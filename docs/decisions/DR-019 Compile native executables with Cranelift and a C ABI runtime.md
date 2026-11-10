# DR-019: Compile native executables with Cranelift and a C ABI runtime

- Status: Proposed
- Date: 2026-11-10

## What is the proposed decision?

Add ahead-of-time compilation with Cranelift object emission, a prebuilt Rust
runtime exposing a C ABI, and the system linker driver. Initially target baseline
x86-64 GNU/Linux ELF executables with the runtime linked statically and libc
dynamically. Confirm the libc choice before implementation; the existing compiler
release uses musl, which does not determine the ABI of generated programs.

Consume the canonical resolved program directly, with backend-local control flow
and ownership lowering. Keep the interpreter and embedding APIs. Do not introduce
a new resolver, optimizer framework, or second maintained native backend now.

Acceptance requires a successful object/runtime/linking experiment and review.
No native backend or runtime ABI is implemented by this record.

## Why is this recommended?

Cranelift's Rust integration and emphasis on fast code generation fit the desired
balance. A prebuilt runtime keeps Cargo/Rust compilation out of individual Ocelot
builds. A narrow ABI separates generated code from Rust layouts and trait objects.
String ownership and cleanup must be explicit to avoid leaks.

These are design-based expectations, not measured Ocelot performance. Compare a
small C baseline before committing to this recommendation.

## What is the context?

The user requested native binaries for x64 Linux and a comparison of simplicity,
compilation time, and runtime performance. Today typed ASTs are interpreted and
built-ins use Rust trait objects.
[DR-003](<DR-003 Resolve programs into one canonical semantic model.md>) permits
future backends without a parallel semantic environment;
[DR-001](<DR-001 Isolate platform interactions behind the PAL.md>) governs platform
operations. The [plan](../plans/2026-11-10-native-binaries-plan.md) contains the
comparison, ABI assumptions, implementation milestones, verification, and sources.

## What would the consequences be?

- Executables would run without Ocelot or source files, but GNU output still
  requires compatible system libraries.
- Builds require a linker driver and matching packaged runtime archive.
- Interpreter/native parity needs differential tests, including diagnostic sources,
  effects, assertions, early returns, and string lifetimes.
- Runtime PAL initialization and dependencies may affect startup time and size.
- ABI versioning and built-in mappings become explicit contracts.
- Full debugger information, musl output, cross-compilation, JIT, and additional
  optimizing backends remain separate milestones.

## What alternatives are under consideration?

- **Generate C:** Lowest initial implementation burden, inspectable output, and
  strong optimization potential. Adds C compilation latency and requires explicit
  sequencing to preserve Ocelot effects. Prefer it if implementation simplicity wins.
- **LLVM:** Strong candidate for aggressive optimization, with more integration
  and toolchain complexity. Reconsider if runtime measurements justify it.
- **QBE:** Compact textual IR and C ABI support; adds an external backend tool
  rather than using a Rust library. Useful if inspectability dominates.
- **Generate x86-64 directly:** Makes register allocation, ABI, relocations, and
  backend correctness project responsibilities without a demonstrated benefit.
- **Bundle the interpreter:** Launchable but does not compile user functions to
  machine code or address interpretation overhead.
