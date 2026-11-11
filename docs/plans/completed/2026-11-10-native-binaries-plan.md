# What will this plan deliver?

Historical implementation account: the Rust runtime described below was replaced
by the self-contained C runtime in [DR-021](<../../decisions/DR-021 Use a C runtime in generated native programs.md>)
on 2026-11-11. See the [native guide](../../NATIVE.md) for current build requirements.

Add a C backend and an external TCC toolchain for native x86-64 Linux executables.
The same generated C must remain usable by later Clang/GCC adapters without
changing language lowering. Verify every executable spec example through the
native path as well as the existing interpreter path.

This replaces the earlier Cranelift implementation proposal in this plan.
[DR-020](<../../decisions/DR-020 Generate C with replaceable native toolchains.md>)
records the user-selected direction; the unimplemented Cranelift proposal,
[DR-019](<../../decisions/DR-019 Compile native executables with Cranelift and a C ABI runtime.md>),
is rejected. The backend, runtime, CLI, and spec matrix are implemented; final
verification and completion tracking are recorded below.

# What is the initial scope?

- Baseline x86-64 Linux, System V AMD64 ABI, ELF64, GNU libc; no host-specific ISA.
- One external TCC executable, pinned/tested in development and CI. Invoke it as
  a child process through PAL, not through libtcc or `tcc -run`.
- Generate a real executable and launch that artifact in tests. Execution requires
  compatible system libraries, but no Ocelot, TCC, Rust, or source installation.
- Support the complete currently implemented language, including modules, typed
  calls/results, strings/templates, booleans, unit, bindings, blocks, conditionals,
  negation, equality, effects, and early returns.
- Keep interpretation, the public embedded API, and the REPL working. Native spec
  scenario drivers are test artifacts, not a new public JIT or embedded API.
- Other architectures, musl/static output, user FFI, shared libraries, full DWARF
  debugging, and an optimizing compiler mode are later work.

The existing release ships a musl-linked compiler. That does not select the
generated program's ABI: package the GNU-target runtime separately and document
the supported glibc baseline. Test TCC's actual linker/relocation behavior before
choosing PIE; do not carry over an unverified PIE requirement from the old plan.

# How will the backend and toolchain stay separate?

Use three responsibilities, with the engine orchestrating them:

| Responsibility | Input | Output |
| --- | --- | --- |
| C backend, proposed `ocelot-codegen-c` crate | Canonical `ResolvedProgram` and entry mode | Deterministic C translation unit and source metadata |
| Native toolchain adapter | C source paths, runtime artifact, target, output path | Executable or a structured tool failure |
| Native execution/spec runner | Executable, isolated workspace, input scenario | Exit status, stdout, stderr, and optional eval-result artifact |

Consume [ResolvedProgram](../../../crates/semantic/src/resolved_program.rs) from
[compile_program](../../../crates/engine/src/compile_program.rs). Preserve the canonical
semantic model from [DR-003](<../../decisions/DR-003 Resolve programs into one canonical semantic model.md>):
do not duplicate name resolution, label/type/effect checking, or module lookup.

Define a small toolchain request/result contract and an explicit TCC adapter.
Keep executable paths, arguments, libraries, diagnostics, and target capabilities
in that adapter; the emitter must not know TCC flags. Model compile/link commands
as argument vectors, never shell strings. Permit a configured compiler path
containing spaces without treating it as a shell command.

Initially implement only TCC. A later Clang/GCC adapter changes driver arguments
and capabilities, not the emitter or runtime ABI. A same-source smoke test with
system Clang/GCC verifies portability without requiring a second production
adapter. Do not create a plugin registry, optimization framework, or silently
fall back to another compiler when TCC fails.

Implemented CLI: `ocelot build <source-file> -o <executable>`, with `--emit=c`
for inspecting generated source. Keep future toolchain selection explicit;
changing an executable path must not pretend to translate incompatible flags.

# How will portable C preserve Ocelot semantics?

Emit a conservative C99 subset supported by the selected TCC and checked with
Clang/GCC. Avoid GNU statement expressions, nested functions, computed gotos,
compiler-specific cleanup attributes, or dependence on C expression order.

- Declare all user-function prototypes before bodies; mangle canonical function
  identities into deterministic C identifiers independent of source spellings.
- Lower expressions into explicit statements and fresh temporaries. Evaluate
  callees/arguments, comparison operands, and interpolations in Ocelot order.
- Resolve lexical names to generated locals; shadowed names must not collide.
  Script root values remain unavailable to user functions as in the interpreter.
- Lower value-producing blocks/conditionals using result temporaries and branches.
  A missing else is unit; the internal non-fallthrough type produces control flow,
  not a C value. Unit expression results must still preserve their effects.
- Lower explicit returns through cleanup paths. A return inside an operand,
  initializer, condition, template, or argument skips every subsequent effect.
- Give strings explicit ownership. Generate retain/release and transfer code for
  bindings, temporary values, parameters, branch joins, and returns.
- Store literal UTF-8 as byte arrays plus lengths, including embedded NUL; never
  use NUL termination as string length or let C string equality compare pointers.
- Preserve static checking of unused bodies and unselected branches. Static effect
  annotations remain checked by the resolver, not implemented as runtime flags.
- Script entry executes statements in order; module entry calls zero-argument
  `main()` and discards its result. Ordinary execution does not run test bodies.

# What runtime will generated code use?

The implementation provides a small Rust runtime as a prebuilt `staticlib`
with a C header and versioned ABI. Generated programs must not compile Rust or
invoke Cargo per build. The runtime string/print probe established the explicit TCC-object/system-cc
linking policy documented in the implementation results below.

Use normalized scalar booleans, a documented unit representation where a C value
is necessary, and opaque reference-counted string handles. String parameters are
borrowed during calls; string results are owned. Returning a parameter/local must
retain or transfer ownership before scope cleanup. Avoid process-lifetime arenas
and leaks as the final string strategy.

Specialize built-ins from known Ocelot types rather than boxing every value.
Preserve mixed-type `assert_eq`, exact string equality, display/eval rendering,
and current runtime errors such as printing unit. Map built-ins by canonical
identity, not unqualified spelling. Diagnose unsupported custom native functions;
do not serialize Rust trait objects or compiler-process function pointers.

Share pure value/diagnostic rendering with interpretation where practical.
Embed source paths, spans, and excerpts required by assertion snapshots so binaries
work without source files. Language failures emit stable diagnostics and nonzero
status after preserving earlier output; normal completion exits zero. Rust panics
must not unwind through generated C frames.

All platform operations remain behind [PAL](../../../crates/pal/src/pal.rs).
Add a focused stderr/report operation if the runtime needs one rather than
bypassing PAL to emit language diagnostics.
Measure the existing PalReal/Tokio startup and dependency footprint; make runtime
initialization lazy or factor a focused PAL adapter if necessary. Test runtime
lookup/version errors and installed layouts outside the repository.

The validated adapter compiles with TCC and links with system cc. This is an
explicit toolchain requirement, not a fallback. The same ABI and C output remain
independent of that adapter.

# How will every spec example be verified natively?

Extend the existing
[loader](../../../crates/spec_validation/src/load_spec_chapters.rs),
[executor](../../../crates/spec_validation/src/execute_spec_example.rs), and
[validator](../../../crates/spec_validation/src/validate_spec_directory.rs).
Reuse the same examples, formatting checks, expectations, and normalization.
No duplicated Markdown parser, native-only copied examples, broad snapshot
rewrites, or hidden exclusions.

Run an explicit matrix of interpreter and C/TCC modes. Compare each mode with the
written expectation independently; interpreter agreement alone cannot establish
correctness. Report chapter, example, mode, failure stage, and coverage counts.
Every loaded example must have a result in both modes. Missing TCC is a setup
failure in required native validation, never a skip or interpreter fallback.

| Example kind | Native validation |
| --- | --- |
| Successful file program | Resolve, emit C, compile/link with TCC, execute the binary, require exit zero, compare stdout |
| Frontend/entrypoint error | Attempt the native build through the same language pipeline, compare the expected diagnostic; no executable is expected |
| Runtime error | Require a successful native build, run it, require a language failure and matching diagnostic |
| Embedded execute/eval sequence | Validate each step and compile a native scenario driver, execute it, compare the final typed eval rendering or first language error |

TCC errors, missing libraries, crashes/signals, timeouts, missing output artifacts,
and driver/protocol errors are infrastructure/backend failures. They cannot
satisfy `### Error`, even if the process exits nonzero. Keep language error
classification separate from toolchain errors and process termination.
Use a documented runtime failure status/report to identify language errors;
an arbitrary nonzero status and plausible stderr text are insufficient.

For file errors, preserve stdout separately even when only the diagnostic is
asserted by the current spec; add regressions for effects before failure.
Keep existing full assertion/error snapshots, normalizing only the same temporary
path prefixes, ANSI escapes, and line endings as the interpreter runner.

# How will embedded examples compile without changing their meaning?

Do not omit `execute:`/`eval:` examples or naïvely concatenate their snippets.
The existing harness uses sequential, source-backed sessions, retains successful
root values, executes new statements once, and does not retain eval-local bindings.

Build a test-only finite scenario using the same frontend preparation rules:
validate execute submissions against retained source; validate eval as exactly
one expression; preserve synthetic source offsets and `<embedded>` diagnostics.
Generate step functions and a driver sharing explicit root-binding storage.
Namespace code emitted from different source snapshots to prevent C symbol
collisions; execute only each submission's new statements. User functions keep
their isolated lexical scopes.

Each successful execute commits its root values; eval borrows the session values
and releases its locals. Run steps in order inside one generated process, stopping
at the first runtime failure. Return the last successful eval's bool/string/unit
rendering through a dedicated structured result artifact, separate from ordinary
stdout/stderr. This prevents printed text from masquerading as the eval result.

A later compile-time error must not mask an earlier runtime failure. Prepare only
the valid prefix up to the first static error, compile/run that prefix, and report
the static error only if the prefix succeeds. Do not execute prefix steps in the
interpreter. This ordering needs a dedicated harness regression.

The scenario adapter is not a substitute for transactional embedded-engine tests.
Retain existing interpreter session tests, and add native scenario cases proving
one-time initialization, persistent roots, eval-local isolation, result quoting,
module lookup, and first-error ordering. If recovery-after-failure scenarios are
added, represent candidate roots and commit/rollback explicitly before claiming
that coverage.

# What test infrastructure is required?

Native executables need real files. The current CapturingPal can hold virtual
sources, so explicitly materialize each example's files and build artifacts under
a unique real directory, preserving module-relative layout. Share setup helpers
where practical; virtual files alone cannot be passed to TCC.

Use PAL for materialization, subprocesses, output capture, and cleanup. Cover
command construction/tool failures with PalMock, and require real TCC process tests
with PalReal. Bound compile/run durations; extend PAL cancellation/timeout support
if needed, ensuring hung processes are terminated and reaped rather than merely
abandoning a waiting thread. Isolate concurrent runs and retain generated C,
command metadata, and stderr for failures; clean successful artifacts.

The spec runner accepts `--backend=interpreter|c-tcc|all`. A required
`validate-native` task runs in `nao check` on the supported Linux development/CI
environment alongside existing interpreter validation.
Provision pinned TCC and its headers/runtime support files in the project's
development environment and CI. Check tool availability/version before the matrix.
Do not require or claim coverage on unsupported hosts; report that distinction
explicitly instead of presenting native validation as passed.

As a portability guard, compile/run representative emitted C with system
Clang/GCC using strict warnings and an optimized build. Run sanitizer or memory
checker cases for string ownership where supported; TCC success alone does not
prove the emitted C is free from undefined behavior.

# In what order will work land?

- [x] Prove x86-64 TCC compilation/linking with the real runtime ABI, string return,
  assertion, and target dependencies. Select executable type and record versions.
- [x] Define/test the runtime ABI, ownership rules, failure reporting, source
  metadata, packaging lookup, and archive compatibility.
- [x] Add the C emitter for the checked language, with deterministic output,
  explicit sequencing, lexical locals, and cleanup. Add source-driven regressions.
- [x] Add the replaceable toolchain request/result boundary and TCC adapter;
  integrate the build CLI, emitted-C inspection, staging, and atomic publication.
  Preserve an existing executable if any stage fails.
- [x] Add native file-example execution and negative-error classification to the
  shared spec harness. Require real binaries and exact existing expectations.
- [x] Add compiled embedded-scenario drivers and first-error sequencing. Account
  for every loaded example in native and interpreter reports.
- [x] Provision TCC and wire mandatory native validation into nao/CI. Test missing
  tools, linker failures, paths with spaces, concurrency, timeouts, and cleanup.
- [x] Run same-source Clang/GCC portability checks, memory checks, installed-layout
  tests, and source-free executable smoke tests. Measure compile/link latency,
  startup, runtime allocations, and binary size without promising speedup ratios.
- [x] Update CLI/spec/testing/release documentation as features land, run
  `nao check`, commit coherent verified units, and move this plan to completed
  only after the full interpreter/native spec matrix passes without exclusions.

# What assumptions and decisions remain?

GNU libc output is the initial assumption; choose/document its oldest supported
baseline before distributing runtime archives. TCC version, runtime archive
linkability, PIE capability, and PAL timeout support require concrete validation.
A runtime implementation/linker change must be reflected in DR-020.

Do not claim native support in the implemented-language overview or spec until
it exists. The current language lacks numeric loops, so its spec cannot establish
future numerical optimization quality. Replacing TCC later should be driven by
measured workloads, with the same C ABI and spec matrix protecting semantics.

# Which references guide implementation?

- [TCC documentation](https://bellard.org/tcc/tcc-doc.html): external compilation,
  object/executable emission, linker inputs, and supported C subset. This manual
  documents 0.9.27; validate the actual pinned build rather than assuming all
  current platform capabilities from its older examples.
- [Rust linkage](https://doc.rust-lang.org/reference/linkage.html): staticlib/native
  library dependencies and mixed-language linking.
- [Spec authoring and execution contract](../../spec/README.md).
- [Existing embedded session](../../../crates/engine/src/embedded_engine.rs).
- [Testing guidance](../../TESTING.md) and [plan workflow](../../PLANS.md).

# What was reviewed before implementation?

Reviewed the current frontend, native-function interface, spec loader/executor,
error rendering, PAL process interface, and nao task graph. The implementation results and final verification below now establish the
completed native path.

# What did implementation establish?

- TCC is obtained through the checksum-pinned tool-tool bootstrap (`./t tcc`),
  using vlang/tccbin commit 60d606e37e6afe1ac5801fd88c55036a56b7caae, reporting
  0.9.28rc 2026-07-24 HEAD@85ba3ae8 on x86-64 Linux.
- The explicit adapter uses TCC for object compilation and system cc for linking
  the prebuilt Rust runtime, with `--gc-sections` and `--no-eh-frame-hdr`. This is
  the planned explicit linker-policy revision, not an automatic fallback.
- ABI v1 uses an unboxed tag/payload pair, with Arc-owned SharedString handles
  only for strings. Generated frames track owned strings; booleans/unit require
  no cleanup allocation. Calls borrow parameters and return owned values.
- The generated artifact was verified as x86-64 ELF PIE. A release-runtime probe
  needed libc, libm, libgcc_s, and the GNU loader. Native runtime I/O uses a small
  synchronous PAL boundary and does not initialize Tokio.
- All 90 examples in 22 chapters pass independently in interpreter and C/TCC
  modes, including embedded execute/eval scenarios; no examples are excluded.
- Regressions verify return/argument ordering, shadowing, exact NUL/Unicode bytes,
  persistent roots without replay, first-error precedence, deterministic C, missing
  tools/runtime, linker errors, output preservation, and child-process deadlines.
- Strict optimized Clang compilation, ASan, UBSan, and leak detection pass the
  same-source ownership smoke test. LeakSanitizer required unsandboxed process
  inspection on this host; no sanitizer failure was suppressed.
- An installed-layout test copied the compiler and release archive outside the
  checkout, built a program there, then removed compiler/archive/source and ran
  the resulting executable from another directory.
- One illustrative local sample took 12 ms for TCC compilation and 165 ms for
  linking with section GC, producing a 4,528,888-byte binary. A separate installed
  hello-world sample built in 353 ms and ran in about 1.4 ms. These are single-run
  observations with Rust 1.98.0, not comparative benchmarks or throughput claims.
- CI and release jobs now pin Ubuntu 24.04 for GNU runtime builds and provision
  TCC through tool-tool; workflow configuration is checked locally but has not
  been executed on GitHub during this task.

The [linking learning](<../../learnings/2026-11-10 TCC objects need an explicit Rust runtime linking policy.md>)
and [native guide](../../NATIVE.md) document the tested policy and scope limits.

# What final verification passed?

`nao check` passes its seven-task graph, including all 381 Rust tests and both
90-example spec passes. `cargo test --workspace`, direct native regressions,
strict Clang/sanitizer checks, source-free installed execution, and local
documentation link checks passed. `git diff --check` is clean. No planned
implementation work remains; public JIT, alternate adapters, and non-GNU targets
remain explicitly outside this milestone. GitHub workflows were configured but
not run remotely in this session.
