# What outcome are we planning?

Compile Ocelot file programs into native x86-64 Linux ELF executables. Generated
programs must run without Ocelot, Rust, source files, or a compiler installed.
Reuse the current resolver, type checking, effect checking, and module loading.
Keep the interpreter and embedded sessions working as they do today.

This is an implementation plan, not an implemented feature. The recommended
backend is Cranelift AOT, subject to a small feasibility and timing experiment.
[DR-019](<../decisions/DR-019 Compile native executables with Cranelift and a C ABI runtime.md>)
is Proposed until that experiment and implementation review establish the choice.

# What are the options and tradeoffs?

The following is an engineering assessment of integration cost and the projects'
stated designs, not measured Ocelot performance. Compiler implementation time,
building the Ocelot compiler itself, compiling user programs, and executing those
programs are separate costs.

| Option | Implementation and distribution | User-program compilation | Generated runtime performance |
| --- | --- | --- | --- |
| Generate C, invoke Clang/GCC | Simplest route to a first compiler; inspectable output; needs a C compiler on the build machine | Starts another compiler, parses generated C, optimizes, and links | Strong optimization potential; depends on generated C and flags |
| Cranelift AOT | Moderate lowering work; Rust library integration; still needs a linker driver and runtime archive | Designed for fast code generation; emits objects directly | Deliberately trades some optimization opportunities for compiler speed and simplicity |
| LLVM | Larger API/toolchain integration and versioning burden; textual IR plus Clang can avoid linking LLVM into Ocelot | Optimization pipelines can cost more, especially at higher optimization levels | Best candidate here when aggressive optimization becomes the priority; no guaranteed win for every workload |
| QBE | Compact textual IR and straightforward C ABI; an additional external backend executable | Lightweight design; commonly emits assembly before assembly/linking | Intentionally narrower optimization scope than industrial backends |
| Handwritten x86-64 | Small demonstrations are easy; register allocation, relocations, ABI, and debugging become our responsibility | Potentially fast, but requires building and maintaining the backend | Depends entirely on optimizations we implement; poor use of project effort now |

**Recommendation:** use Cranelift as the first maintained native backend. It fits
the Rust implementation and gives a useful balance without an LLVM dependency.
If minimum implementation effort outweighs compiler latency, choose C generation
instead. LLVM remains a future option if measurements establish a runtime need.
Do not maintain two production backends or introduce a backend plugin framework
as part of this milestone.

Generating C is a legitimate long-term choice, not merely a throwaway approach.
It must emit explicit temporaries and sequencing: C argument evaluation order
cannot be used to implement Ocelot's left-to-right effects and early returns.
Source-level debugging also needs deliberate mapping, whichever backend is chosen.

# What does x86-64 Linux mean for the first target?

Propose `x86_64-unknown-linux-gnu`, System V AMD64 calling conventions, ELF64,
little endian, and baseline x86-64 instructions without host-specific CPU tuning.
Use PIC objects and the system `cc` driver to produce a PIE executable, linking
the Ocelot runtime statically and the system C runtime dynamically. Verify this
combination in the feasibility experiment rather than assuming relocation support.
Choose and document the oldest supported glibc environment in CI before packaging.

The build machine needs a compatible linker driver and development libraries.
The execution machine needs compatible glibc/system libraries, but no Ocelot runtime
installation. "Native" does not imply "fully static" or "runs on every Linux".
Other architectures, cross-compilation, musl output, shared libraries, FFI for user
programs, JIT, and compiling persistent REPL state are outside the first milestone.

The existing release ships a musl-linked compiler; its ABI is independent of the
ABI of generated programs. Packaging must supply the GNU-target runtime archive
and linker requirements explicitly. If a single static distributable executable
is the actual first requirement, switch this proposal to musl before implementation
and budget for its linker/sysroot setup. Do not silently inherit the compiler's host
target or treat both libc targets as supported.

# How does this fit the current implementation?

[compile_program](../../crates/engine/src/compile_program.rs) already returns a
[ResolvedProgram](../../crates/semantic/src/resolved_program.rs) with canonical
types, resolved function indices, modules, and diagnostics. Lower this checked
representation directly into Cranelift functions and control-flow blocks. Use
backend-local lexical environments for values and explicit block parameters for
conditional results. No second resolver, custom optimizer, or general-purpose
middle IR is required for the initial language.

Keep this consistent with [DR-003](<../decisions/DR-003 Resolve programs into one canonical semantic model.md>).
Resolve/check the whole program before generating code, including unused bodies
and unselected branches. Declare function symbols before compiling bodies to
support forward calls and recursion. A script gets a synthetic entry function;
a module invokes its zero-argument `main()` and discards its result as today.
Test declarations are checked but do not execute in ordinary binaries.

The current native-function registry holds Rust trait objects. These objects,
Rust enum layouts, and compiler-process function pointers cannot be embedded as
a usable ABI in an independent executable. Map supported built-ins by canonical
identity to explicit runtime symbols, and diagnose unsupported registrations.

# What runtime and ownership rules are needed?

Add a small Rust runtime crate built as a versioned `staticlib`, with a narrow C
ABI. Build it once per compiler distribution/target; compiling an Ocelot program
must not invoke Cargo or rebuild the runtime. Pin matching Cranelift crate versions
together when implementing, based on the actual Rust toolchain requirements.

Proposed initial representation:

- Booleans use a normalized scalar value; unit has no machine-level payload.
- Strings use opaque runtime handles, with literal UTF-8 bytes in read-only data.
  Construct handles through runtime calls; do not expose `SharedString` layout.
- Function string parameters are borrowed for the call; string results are owned.
  Returning a borrowed parameter/local requires a retained reference.
- Runtime helpers borrow inputs unless explicitly documented as consuming them.
  Bindings and owned temporaries are released at scope exit, including early
  returns; branch results transfer ownership to the join. Immutable strings need
  reference counting, not a garbage collector. Do not use a process-lifetime arena
  as the final ownership scheme or silently leak temporary strings.
- Specialize built-in calls from known operand types. `any` in built-in signatures
  does not require boxing every Ocelot value. Preserve mixed-type `assert_eq`
  behavior and the existing rejection of `println` on unit values.

Support exact string equality, interpolation, bool/string printing, and assertions.
Refactor/share pure display and assertion logic where practical; do not pull the
parser/resolver/interpreter into generated binaries. Define an ABI version and
reject mismatched runtime archives.

Runtime failures must preserve effects already performed, print a useful message
and source location, and exit nonzero. Embed the diagnostic source metadata needed
to operate without source files. Use explicit failure paths; do not unwind Rust
panics through generated frames or expose a raw machine trap as an assertion message.
Success exits zero even when a module's `main()` returns a non-unit value.

Compiler I/O and linker execution use the existing PAL, argument vectors, and
process events. Runtime platform operations also stay behind the PAL. Initially
reuse its adapters, but measure their footprint: `PalReal` currently initializes
Tokio, and that startup/dependency cost may dominate a tiny native program. If
material, make that initialization lazy or introduce a focused PAL capability
without duplicating direct OS I/O throughout the runtime.

# In what order should we implement this?

- [ ] **Validate the backend choice.** Create a small Cranelift object containing a
  branch, typed call, and runtime string/print call; link and run it as PIE. Compile
  an equivalent small C program for comparison. Check toolchain compatibility,
  native library dependencies, relocations, and runtime packaging. Record measured
  results and accept or revise DR-019; do not build two complete backends.
- [ ] **Define the runtime ABI.** Document scalar and string representations,
  borrowing/ownership, failure paths, symbol names, and runtime lookup/versioning.
  Add the runtime crate and ABI tests, including returning borrowed strings and
  embedded NUL/Unicode content. Verify libc and Rust staticlib link dependencies.
- [ ] **Add the code generator.** Create `ocelot-codegen` consuming `ResolvedProgram`.
  Declare symbols, lower function parameters/calls, literals, bindings, blocks,
  returns, conditionals, negation, equality, templates, and built-ins. Maintain
  deterministic names, source spans, lexical scopes, and ownership cleanup. Verify
  Cranelift IR; reject unsupported inputs explicitly instead of interpreting them.
- [ ] **Integrate builds.** Propose `ocelot build <source-file> -o <executable>` and
  `--emit=obj` for inspection, with one target initially. Use the existing module
  root and entrypoint rules. Produce objects in an isolated staging directory;
  link to a temporary sibling output and publish only on success. Preserve an
  existing output on failure. Extend PAL cleanup only as required, with real and
  mock coverage. Report missing `cc`, runtime mismatch, and linker failure clearly.
- [ ] **Establish parity.** Add native file-execution support to the spec harness
  alongside interpretation. Run file examples both ways and compare output,
  compile diagnostics, assertion details/locations, exit success/failure, and
  effect ordering. Embedded eval examples remain interpreter-only. Add regressions
  for forward/recursive calls, shadowing, branch joins, returns in either operand,
  argument/initializer early returns, returned strings, and unit built-in errors.
- [ ] **Measure and package.** Add release/CI jobs for the selected target and ship
  its runtime archive and metadata beside the compiler. Test an installed layout
  outside the checkout and execution without compiler/source files. Inspect ELF
  architecture, PIE type, CPU baseline, and dynamic dependencies. Measure runtime
  allocations/leaks and the PAL startup cost. Document linker prerequisites.
- [ ] **Finish verification.** Run `nao check` with native parity required on the
  supported CI runner; mock linker errors and exercise the real linker too.
  Run memory checks on returned/branch/temporary strings and error paths. Update
  spec, CLI help, release instructions, and overview only as features land. Commit
  verified logical units; complete and move this plan after native support works.

# How will performance claims be tested?

Measure separately: compiler clean-build time/size; user-program frontend time;
code-generation time; linking time with the prebuilt runtime; total build latency;
generated binary size; process startup; and steady execution time/allocations.
Report toolchain versions, optimization flags, CPU, workload, repeated runs, and
cold versus warm caches. Compare the same programs and avoid constant-folded or
stdout-dominated workloads when measuring computation. Prototype C timings guide
the initial choice; maintaining a second backend is not a benchmark requirement.

The current language has no numeric loops, so its examples cannot establish
future numerical throughput. Use generated call/branch graphs and bounded string
workloads now, and revisit runtime optimization with representative applications.
Cranelift/LLVM/QBE project performance statements are not Ocelot benchmarks.
Do not promise percentage speedups or a second optimizing backend before evidence.

# What remains to confirm before implementation?

- GNU dynamic output versus musl static output, especially given current releases.
- Required deployment glibc baseline and acceptable external build prerequisites.
- Whether initial source locations/assertion diagnostics suffice, or full DWARF
  debugger support is required; full debug information is a separate milestone here.
- Baseline timing results and whether developer iteration or peak runtime speed
  should win if the experiment exposes a material tradeoff.

# Which sources support the comparison?

Reviewed on 2026-11-10; dependency versions and measured results must be captured
again when implementation begins.

- [Cranelift project](https://cranelift.dev/): Rust embedding, AOT support, targets,
  and explicit compilation-speed versus optimization tradeoffs.
- [Cranelift object API](https://docs.rs/cranelift-object/latest/cranelift_object/):
  object-file emission through `ObjectModule`.
- [LLVM code generator](https://llvm.org/docs/CodeGenerator.html): native code
  generation infrastructure and optimization stages.
- [Clang driver](https://clang.llvm.org/docs/CommandGuide/clang.html): compilation,
  optimization, assembly, and linking stages for generated C or LLVM IR.
- [QBE project](https://c9x.me/compile/) and
  [author's comparison with LLVM](https://c9x.me/compile/doc/llvm.html): compact
  backend scope, C ABI support, and intentionally limited optimization goals.
- [Rust linkage reference](https://doc.rust-lang.org/reference/linkage.html):
  staticlib packaging, native dependencies, and mixed-language linking constraints.
