# Why is a dedicated embedded execution API needed now?

The current repository has enough pieces to execute Ocelot code, but they are split across layers that are awkward for embedding:

- [`Engine`](/data/projects/ocelot/crates/engine/src/engine.rs) is file-system oriented and assumes execution starts from a real entry path
- [`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) owns the useful parse-resolve-execute pipeline, but it is shaped around CLI commands rather than host application use
- [`Interpreter`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) can evaluate expressions internally, but only after parsing and resolution have already happened

That leaves embedding use cases in an annoying middle state:

- they cannot evaluate an arbitrary expression string directly
- they cannot easily reuse a prepared execution environment without re-creating engine plumbing
- they would have to know too much about parser, resolver, synthetic source files, and symbol tables to do something that should feel boring

This work should introduce one self-contained `EmbeddedEngine` that owns those details and exposes an embedding-first API.

# How should the language spec describe embedded eval behavior?

This feature needed explicit spec coverage rather than hiding inside engine RustDoc or tests.
Embedded eval is not ordinary script execution, and pretending otherwise would blur the runtime model.

Recommended documentation work:

- add one runtime-behavior chapter that explains what an embedded eval session is, how it retains state, and how `base_path` affects module loading
- add one expressions or runtime-results chapter that explains what it means for eval to return a [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs) instead of only producing printed output
- update [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md) so spec examples can describe eval-driven expectations in addition to ordinary `### Output` and `### Error` script expectations

The spec should answer at least these questions:

- what operation embedded eval performs
- whether eval executes in a persistent session or in isolation
- what prior definitions remain visible across calls
- how modules are resolved relative to `base_path`
- how successful eval results are rendered for spec comparison
- how eval failures are reported

# What spec chapters should be added for this feature?

Recommended first chapters:

- a new chapter under runtime behavior, for example `28.03 Runtime behavior - Embedded eval.md`
- a second chapter focused on eval results, for example `28.04 Runtime behavior - Eval results.md`

The first chapter should document session semantics:

- `EmbeddedEngine` owns a persistent evaluation session
- calls execute against the current session state in source order
- successful state-changing calls become visible to later eval calls
- failed calls should not commit partial session changes
- module loading happens relative to the configured `base_path`

The second chapter should document observable results:

- successful eval returns one runtime value
- result rendering for spec purposes should be stable and human-readable
- result values may include at least strings, booleans, and unit once unit needs representation
- eval errors use the same stable error rendering approach as other spec-validation failures

If the implementation ends up with a cleaner chapter split elsewhere, that is fine, but the spec should keep session behavior and result behavior as separate concerns.

# What should `EmbeddedEngine` be responsible for?

`EmbeddedEngine` should live in the engine crate and provide a small public API for host applications that want to run Ocelot code without going through CLI-style file execution.

Recommended responsibilities:

- own a [`PalHandle`](/data/projects/ocelot/crates/engine/src/engine.rs) and compilation inputs needed for execution
- own a `base_path` used to discover and load real `.ocelot` modules from the host project
- register builtin modules the same way [`Engine`](/data/projects/ocelot/crates/engine/src/engine.rs) does, so embedding behavior matches normal engine behavior
- maintain persistent in-memory embedded source state across calls
- compile that state together with disk-backed modules under `base_path` into a resolved program artifact whenever new embedded input is added
- evaluate one expression string and return [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs)
- retain embedded declarations between evaluations so later expressions can refer to earlier definitions

It should not require embedding callers to manually construct ASTs, symbol tables, or temporary files.

# What API shape keeps the first version useful without overengineering it?

The first version should stay intentionally small.

Recommended public shape:

- `EmbeddedEngine::new(pal: PalHandle, base_path: impl Into<FilePath>) -> Self`
- builtin module registration equivalent to [`Engine::register_builtin_module()`](/data/projects/ocelot/crates/engine/src/engine.rs)
- one method for adding persistent embedded code, such as `EmbeddedEngine::execute(source: &str) -> OcelotResult<()>`
- `EmbeddedEngine::eval(expression_source: &str) -> OcelotResult<RuntimeValue>`

One internal concept should be explicit from the start:

- a persistent embedded session state that stores synthetic source buffers, the latest compiled [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs), and stable synthetic file paths

That keeps the public API small while giving the implementation one honest place to preserve state between calls.

# How should persistent embedded state work across multiple calls?

The embedded engine should behave more like a REPL session than like a one-shot eval helper.

Recommended direction:

- collect real project modules from `base_path` the same way file-based engine execution already does
- keep one synthetic script file for embedded top-level statements and expression history
- place that synthetic script under the same `base_path` namespace with a stable path such as `<base_path>/.embedded/main.ocelot-script`
- append host-provided persistent snippets to that synthetic source in call order
- reparse and reresolve the full module set after each state-changing call
- let later expressions see declarations introduced by earlier snippets
- let embedded code import and call real modules loaded from `base_path`
- treat builtin modules and native functions as part of the stable session configuration

Because the current language has no assignment syntax or mutable local declarations, “retaining state” currently means:

- preserving top-level function declarations
- preserving the ordering of previously executed top-level statements
- preserving access to real modules under `base_path`
- preserving any observable host effects produced by earlier executions through the PAL

It does not yet mean arbitrary mutable variable state inside Ocelot itself.

# How should spec validation change to verify eval and eval results?

The current spec-validation crate only knows how to write example files, run `Engine::run_file()`, and compare either printed output or rendered error text.
That is too narrow for embedded eval.

Recommended direction:

- extend the spec-example model with an explicit execution mode instead of assuming every example is a file execution case
- add one eval-oriented mode that drives [`EmbeddedEngine`](/data/projects/ocelot/crates/engine/src/lib.rs) rather than [`Engine`](/data/projects/ocelot/crates/engine/src/engine.rs)
- allow one example to describe a sequence of embedded steps, not just one script entry file
- add one expected-outcome kind for successful eval results so the harness can compare returned values directly instead of forcing everything through `println`

One practical Markdown contract worth targeting:

- keep `## Example: ...` headings
- keep filename-labeled `ocelot` blocks for project files loaded from `base_path`
- add visible eval-step labels such as `eval:` or `execute:` before fenced `ocelot` blocks for embedded session actions
- add a dedicated expectation heading such as `### Eval result` for successful returned values
- continue to support `### Error` for failing eval examples

That would let the harness represent examples like:

- preload helper modules from `base_path`
- execute one embedded snippet that defines `fun greet() { ... }`
- evaluate `greet()` in a later step
- compare the returned rendered value against `### Eval result`

This is much cleaner than trying to encode embedded eval as a fake `main.ocelot-script` program.

# How should arbitrary expression strings be compiled without creating a parallel language pipeline?

The simplest correct approach is to reuse the existing compilation pipeline and wrap the requested expression in synthetic script source.

Recommended direction:

- create one stable synthetic source file path under `base_path`, such as `<base_path>/.embedded/main.ocelot-script`
- keep accumulated embedded declarations and statements in that synthetic source file
- build eval requests around the requested expression in a temporary wrapper such as `<expression>;`
- parse and resolve eval requests against the persistent embedded source, real modules discovered from `base_path`, and builtin modules
- extract the single top-level expression statement from the temporary eval unit
- evaluate that resolved expression with the interpreter and return its resulting [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs)

This is preferable to exposing a new public parser entry point immediately because:

- it reuses the exact grammar and diagnostics path the language already has
- it avoids introducing a second “expression-only” compilation mode too early
- it keeps parser API churn out of scope unless the wrapper approach proves insufficient

If the wrapper forces bad diagnostics or awkward AST extraction, that is the moment to add a dedicated public expression parser. Doing it preemptively would be architecture cosplay.

# How should the interpreter boundary change to support expression results?

[`Interpreter`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) already has the core behavior needed, but the useful part is trapped in a private helper.

Recommended direction:

- add a public interpreter entry point that evaluates one already-resolved [`Expression`](/data/projects/ocelot/crates/ast/src/expression.rs) and returns [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs)
- keep statement and script execution entry points for existing engine behavior
- avoid duplicating evaluation logic in the engine crate

That keeps the layering clean:

- engine handles source preparation, parsing, resolution, and user-facing embedding API
- interpreter handles runtime execution of resolved AST nodes

# How should the engine crate be refactored so `EmbeddedEngine` does not duplicate `EngineWorker`?

The current `EngineWorker` contains most of the right behavior, but it is glued to file discovery and command dispatch.

Recommended refactor:

- extract a smaller compilation helper in the engine crate that can:
  - parse provided source files or synthetic in-memory modules
  - discover disk-backed modules relative to a caller-provided base path
  - include builtin modules
  - resolve to one [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs)
  - preserve parser and resolver diagnostics in one obvious place
- keep [`Engine`](/data/projects/ocelot/crates/engine/src/engine.rs) using that helper for file-based execution
- let `EmbeddedEngine` use the same helper for synthetic persistent-session compilation

The goal is not to make embedding reuse `EngineWorker` directly.
The goal is to extract the real shared compilation boundary so both APIs stay readable.

# What execution context should embedding reuse or preserve?

The first implementation should reuse the existing resolver and interpreter environment model rather than inventing a new runtime context type.

That means:

- reuse [`CompilationInputs`](/data/projects/ocelot/crates/semantic/src/compilation_inputs.rs) for native-function registration
- reuse [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs) and [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) as the compiled semantic artifact
- reuse [`Interpreter`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) for runtime evaluation

The embedded session itself should become the persistent context boundary.

That session should keep:

- the configured `base_path`
- the accumulated embedded source text
- stable synthetic source-file metadata used for diagnostics
- the most recent successfully resolved program, if caching it makes eval simpler

It should not invent a second runtime environment separate from the existing interpreter and resolver models.

# What implementation order keeps this manageable?

1. Add an active plan for the embedding work.
2. Introduce a new public [`EmbeddedEngine`](/data/projects/ocelot/crates/engine/src/lib.rs) entry point in the engine crate.
3. Define persistent embedded-session state for `base_path`, synthetic sources, and stable diagnostic paths.
4. Extract engine-internal compilation helpers so file-based engine execution and embedded evaluation can share parse/resolve logic, including module discovery from `base_path`.
5. Add an API for applying persistent embedded source snippets to the session and recompiling the accumulated program plus disk-backed modules.
6. Add a public interpreter method for evaluating one resolved expression and returning [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs).
7. Define the spec-facing rendering rules for successful eval results so the harness can compare `RuntimeValue` results deterministically.
8. Update `docs/spec/README.md` and the spec-validation domain model to support embedded-eval example modes and `### Eval result` expectations.
9. Add new spec chapters for embedded eval session semantics and eval-result behavior.
10. Implement `EmbeddedEngine::eval()` by compiling the current persistent session plus one temporary expression statement against modules loaded from `base_path`.
11. Ensure builtin module registration and default native-function setup match normal engine behavior.
12. Add colocated tests for persistent state across calls, module loading from `base_path`, builtin usage, type/runtime failures, parser failures, resolver failures, and spec-validation eval examples.
13. Run targeted crate tests, `cargo run -p ocelot-spec-validation`, and `nao check`.

# How should this work be verified?

Verification proved both API usefulness and pipeline reuse.

- [x] Add engine tests proving `EmbeddedEngine` retains earlier definitions across multiple calls, such as defining `fun greet() {}` in one call and evaluating `greet()` or a dependent expression later.
- [x] Add engine tests proving `EmbeddedEngine` can import or call project modules loaded from its configured `base_path`.
- [x] Add engine tests proving failed `execute()` or `eval()` calls do not corrupt the last known good embedded session state.
- [x] Add engine tests for `EmbeddedEngine::eval()` returning booleans, strings, and template-string results as [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs).
- [x] Add engine tests showing embedded eval can call builtin functions and functions from registered builtin modules.
- [x] Add engine tests covering parser diagnostics for malformed expressions.
- [x] Add engine tests covering resolver diagnostics for unresolved identifiers or invalid calls.
- [x] Add interpreter tests for the new public single-expression evaluation entry point.
- [x] Add tests proving embedded evaluation does not require real on-disk source files.
- [x] Add spec-validation tests for embedded eval example parsing, malformed eval examples, successful eval-result comparison, and eval-error comparison.
- [x] Add spec chapters with executable eval examples that run through the spec-validation harness.
- [x] Run `cargo test -p ocelot-engine -p ocelot-interpreter -p ocelot-spec-validation`.
- [x] Run `cargo run -p ocelot-spec-validation`.
- [x] Run `nao check`.

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes wrapping an expression in synthetic script source preserves acceptable diagnostics. If error spans or messages become misleading, a dedicated expression parser entry point may be required.
- This plan now assumes session persistence is source-based and recompilation-based rather than based on a long-lived mutable interpreter environment.
- The current language has no return statements or general expression-oriented script result model, so expression evaluation should operate on the extracted AST node directly rather than on “run script and read the last value.”
- The current language also has no mutable locals or assignment syntax, so retained state is currently limited to declarations and accumulated executed script history. If real mutable embedding variables are needed, the language or runtime model will need another design pass.
- `base_path` semantics need to be precise. `EmbeddedEngine` should most likely behave like `Engine` and discover modules relative to that root rather than relative to the synthetic embedded file path alone.
- The spec-validation format needs an explicit embedded-eval mode. Reusing `### Output` for returned values would be ambiguous and would make it harder to tell whether an example is asserting printed side effects or expression results.
- Synthetic file paths for embedded code need a stable convention so diagnostics stay understandable and tests are deterministic.
- If builtin module registration on [`EmbeddedEngine`](/data/projects/ocelot/crates/engine/src/engine.rs) diverges from [`Engine`](/data/projects/ocelot/crates/engine/src/engine.rs), the embedding API will become confusing fast. Shared storage or shared helper construction is preferable.
- The engine crate may need a small in-memory source representation. That is fine if it removes duplicated parse/resolve code, but it should stay narrow and not become a second filesystem abstraction.
- Re-resolving the full embedded source after each change is the simplest correct first implementation, but it may become a performance problem for long sessions. The plan should prioritize correctness and clarity first, then optimize if real embedding workloads justify it.
- The transaction boundary needs to be explicit. If a snippet fails to parse or resolve, `EmbeddedEngine` should most likely keep the previous valid session instead of half-applying broken source.
- Result rendering needs to be nailed down early. The spec harness needs a stable textual form for returned [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs) values, and that rendering should not drift from one test to the next.

# What concrete tasks should track this plan?

- [x] Create `EmbeddedEngine` as a self-contained embedding API in the engine crate.
- [x] Make `EmbeddedEngine` accept and retain a `base_path` for loading project modules.
- [x] Add persistent embedded-session state that survives multiple `execute()` and `eval()` calls.
- [x] Extract shared engine compilation helpers so embedded and file-based execution reuse one parse/resolve path.
- [x] Add a public interpreter API for evaluating one resolved expression into [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs).
- [x] Implement persistent snippet execution and transactional recompilation of accumulated embedded source.
- [x] Implement `EmbeddedEngine::eval()` against the persistent session using synthetic source instead of a separate expression-only pipeline.
- [x] Define stable spec-facing rendering for successful eval results.
- [x] Extend the spec-validation crate with an embedded-eval execution mode and `### Eval result` expectations.
- [x] Add spec chapters documenting embedded eval session behavior and eval-result behavior.
- [x] Keep builtin module registration and default native-function behavior aligned with `Engine`.
- [x] Add colocated engine and interpreter coverage for success and failure cases.
- [x] Add spec-validation coverage and executable spec examples for eval.
- [x] Run `cargo test -p ocelot-engine -p ocelot-interpreter -p ocelot-spec-validation`.
- [x] Run `cargo run -p ocelot-spec-validation`.
- [x] Run `nao check`.

# What landed from this plan?

This work added a persistent embedding API and the matching spec-validation surface:

- [`EmbeddedEngine`](/data/projects/ocelot/crates/engine/src/embedded_engine.rs) now provides persistent `execute()` and `eval()` calls rooted at a configured `base_path`
- engine compilation logic was extracted into [`compile_program()`](/data/projects/ocelot/crates/engine/src/compile_program.rs) so file execution and embedded evaluation share one parse/resolve path
- [`Interpreter::evaluate_expression()`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) is now public for the resolved-expression eval path
- [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs) now exposes stable eval-result rendering
- the spec-validation crate now supports embedded `execute:` and `eval:` steps plus `### Eval result` expectations
- [`docs/spec/28.03 Runtime behavior - Embedded eval.md`](/data/projects/ocelot/docs/spec/28.03%20Runtime%20behavior%20-%20Embedded%20eval.md) and [`docs/spec/28.04 Runtime behavior - Eval results.md`](/data/projects/ocelot/docs/spec/28.04%20Runtime%20behavior%20-%20Eval%20results.md) document the feature
- `cargo test -p ocelot-engine -p ocelot-interpreter -p ocelot-spec-validation`, `cargo run -p ocelot-spec-validation`, and `nao check` pass
