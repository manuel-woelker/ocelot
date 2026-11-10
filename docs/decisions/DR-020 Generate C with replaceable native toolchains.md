# DR-020: Generate C with replaceable native toolchains

- Status: Accepted
- Date: 2026-11-10
- Scope: Backend/toolchain direction agreed by the user's planning request;
  implementation and detailed runtime ABI remain pending.

## What is the decision?

Generate portable C from the existing canonical resolved program and use an
external TCC toolchain for initial x86-64 Linux native builds. Keep C lowering
independent of toolchain commands through a small build request/result boundary.
Later Clang/GCC adapters should compile the same emitted C without duplicating
language semantics. Start with a child-process TCC invocation, not libtcc.

Require native verification of all executable spec examples alongside interpreter
verification. File programs compile to executables; embedded execute/eval examples
use compiled scenario drivers preserving session ordering and root values.
Expected language errors must match the spec; toolchain failures cannot count as
successful negative examples. Frontend-invalid examples correctly produce no binary.

The initial target assumes GNU libc. Runtime implementation, TCC archive linking,
and executable type must be validated before building the full backend. These
are explicitly pending, not capabilities established by accepting this direction.

## Why was this chosen?

The user selected a C backend with TCC for starters and requested replaceable
toolchains and compilation/execution of spec examples. This minimizes initial
lowering complexity while leaving a route to optimizing C compilers through the
same source output. Explicit sequencing and ownership keep Ocelot semantics out
of toolchain-specific behavior.

A shared spec matrix protects later toolchain replacement and catches differences
in effects, control flow, diagnostics, and string lifetimes. It must account for
embedded examples rather than testing only the convenient file examples.

## What is the context?

The earlier [DR-019](<DR-019 Compile native executables with Cranelift and a C ABI runtime.md>)
proposed Cranelift but was never accepted or implemented. The user subsequently
selected C/TCC. The [revised implementation plan](../plans/2026-11-10-native-binaries-plan.md)
defines lowering, runtime feasibility, toolchain isolation, and full spec coverage.
This adds a backend without replacing the canonical semantic model in
[DR-003](<DR-003 Resolve programs into one canonical semantic model.md>) or the
platform boundary in [DR-001](<DR-001 Isolate platform interactions behind the PAL.md>).

## What are the consequences?

- Generated C becomes inspectable and independently compilable; emission must
  avoid TCC/GNU extensions and C's unspecified argument evaluation order.
- TCC is a build prerequisite; standalone programs need only their documented
  system libraries, not Ocelot or TCC installed.
- Runtime ownership, built-in mappings, ABI versions, and source diagnostics need
  explicit contracts independent of compiler-process Rust layouts.
- Native spec tests need materialized files, real subprocesses, isolated workspaces,
  timeouts, exact expectations, and a compiled embedded-scenario adapter.
- The current interpreter remains supported; native capabilities are documented
  as implemented only after the plan's acceptance checks pass.
- TCC's limited optimization is an initial tradeoff, not a performance guarantee.

## What alternatives were considered?

- **Cranelift first:** Good Rust integration but more initial lowering/toolchain
  work than the chosen generated-C approach; the unimplemented proposal is rejected.
- **Clang/GCC first:** Fits the same C backend but prioritizes optimization over
  the selected lightweight initial toolchain. Keep same-source portability checks.
- **Embed libtcc:** Adds binding/lifetime/distribution work without being needed
  to compile independent executable files.
- **Only compile file examples:** Omits persistent values, typed eval rendering,
  and embedded diagnostics; fails the requested full-spec verification scope.
- **Replay accumulated source for every eval:** Repeats initializer effects and
  can mask earlier runtime failures; use compiled ordered scenario steps instead.
