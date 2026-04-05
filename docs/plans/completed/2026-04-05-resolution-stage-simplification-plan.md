# Why should the resolution stage be simplified?

[`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) currently owns too much of the resolver pipeline.
It parses modules, validates them, creates the compilation session, creates the symbol table, creates per-module environments, and then calls several low-level resolver functions in the right order before rebuilding a [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs).

That split is awkward for a few reasons:

- the engine crate knows resolver internals it should not need to know
- the resolver crate already owns the actual registration and resolution passes, but not the orchestration boundary
- the current API makes it easy to accidentally duplicate pipeline logic or construct an invalid partial resolution flow
- `CompilationContext` currently carries both diagnostics and a symbol table, but `EngineWorker` bypasses that state and threads a separate local symbol table through the pipeline

The next step should be a clean boundary where the engine hands parsed modules to the resolver crate and receives one resolved program artifact back.

# What should the new boundary look like?

The target boundary for this slice is:

- [`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) should call one resolver entry point
- that entry point should live in the resolver crate
- the resolver crate should take `Vec<ParsedModule>` as input
- the resolver crate should take `CompilationSession` as input for compiler-provided native implementations
- the resolver crate should return a `ResolvedProgram` as output
- `ResolvedProgram` should contain:
  - the resolved modules needed by the engine
  - `SourceDiagnostics`
  - `SymbolTable`

This plan assumes `ResolvedProgram` becomes the canonical result of multi-module resolution, while single-file helpers can either wrap it or remain as convenience APIs built on top of the same implementation.

# What should stay inside the resolver crate?

The resolver crate should own all orchestration for:

- module validation currently performed before resolution
- builtin/core module registration needed for semantic analysis
- symbol table creation and mutation
- module environment creation and lookup
- registration passes for effects, functions, and imports
- item resolution and function-body resolution
- construction of the final program-level semantic result

The engine crate should stop knowing about the ordering of `register_module_effects`, `register_module_functions`, `register_module_imports`, `resolve_module_items`, and `resolve_user_defined_function_definitions`.
If the engine still needs to call multiple resolver internals after this change, the boundary is still too leaky.

`CompilationSession` should remain part of this boundary.
Parsed modules include parsed `native fn` declarations, but they do not contain the boxed native implementations used to build native semantic function definitions.

# What data shape should the resolver return?

Add a new `ResolvedProgram` type with a focused API.

Recommended initial shape:

- `modules: Vec<ParsedModule>` for the resolved compilation units
- `source_diagnostics: SourceDiagnostics` so the engine can render compilation errors without borrowing resolver internals
- `symbol_table: SymbolTable` as the semantic output promised by this slice

The implemented version does not need `entry_module_index`.
[`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) already knows the entry path from its command and can look up the entry module without introducing another index to keep in sync.

This deliberately keeps the return type smaller than today’s engine state.
If runtime execution still needs a [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs), the engine can derive it from the returned `SymbolTable` or the resolver can expose a helper for that conversion.

That tradeoff matters: the requested boundary says the resolver should return source diagnostics and a symbol table.
Returning only `ProgramEnvironment` would preserve the old design under a different name and miss the actual simplification goal.

# How should EngineWorker change?

`EngineWorker` should keep ownership of loading and parsing source files, because that is still engine-facing IO orchestration.
After parsing succeeds, it should hand off the parsed modules to a single resolver function and store the returned `ResolvedProgram`.

Recommended engine changes:

1. Keep `parse_modules()` focused on collecting `Vec<ParsedModule>`.
2. Keep `resolve_modules()` as a thin wrapper around one call into the resolver crate.
3. Store the returned `ResolvedProgram` rather than separate `parsed_modules` and `program_environment` fields, or at least derive those fields from the resolved result in one place.
4. Read diagnostics from `ResolvedProgram.source_diagnostics`.
5. Build any interpreter-facing environment from `ResolvedProgram.symbol_table` in a narrow adapter layer.

This is also a good moment to remove duplicated state.
Keeping `parsed_modules`, `program_environment`, and `compilation_context` alive after introducing `ResolvedProgram` would likely leave the worker with two sources of truth.

# What resolver API should be introduced?

Introduce one multi-module resolver entry point with a signature in this shape:

```rust
pub fn resolve_program(
    modules: Vec<ParsedModule>,
    compilation_session: &CompilationSession,
) -> OcelotResult<ResolvedProgram>
```

The exact arguments may shift, but the important invariant is that the engine passes parsed modules in once and receives one resolved result back.
`CompilationSession` should stay because declaration registration resolves boxed native implementations from its registry.

The implementation should:

1. construct the symbol table
2. create per-module environments
3. validate parsed modules
4. register declarations across all modules
5. resolve module items and user-defined function bodies
6. package resolved modules, diagnostics, and symbol table into `ResolvedProgram`
7. return resolver diagnostics in `ResolvedProgram` and let the engine wrapper raise `CompilationStage::Resolver` when errors are present

Builtin modules should not be a separate resolver input for this slice.
If the engine already parses builtin modules into `Vec<ParsedModule>`, the resolver should treat them like any other parsed module.

# How should diagnostics and failure flow work?

The resolver entry point should own its own `CompilationContext` internally and use it to build `ResolvedProgram.source_diagnostics`.

Recommended behavior:

- parser diagnostics remain produced during parsing in the engine
- resolver diagnostics are produced during `resolve_program`
- resolver compilation errors should still be reported as `CompilationStage::Resolver`
- if resolution fails, the engine should still be able to read and render the accumulated `SourceDiagnostics`

This landed with `ResolvedProgram` returned even when resolver diagnostics contain errors.
[`EngineWorker::resolve_modules()`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) stores that result and then raises a resolver-stage compilation error if `source_diagnostics` contains errors, so diagnostics stay available for rendering without letting execution continue.

# What implementation order keeps this change controlled?

1. Add an active plan for the resolver-boundary simplification.
2. Introduce `ResolvedProgram` in a crate that both the engine and resolver can use without creating a dependency cycle.
3. Move multi-module resolution orchestration out of [`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) into one resolver entry point.
4. Keep [`EngineWorker::resolve_modules()`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) as a thin wrapper that delegates to the resolver entry point.
5. Make the resolver entry point own `CompilationContext` and produce `SourceDiagnostics` plus `SymbolTable`.
6. Update the engine to call only the new resolver function after parsing.
7. Replace direct engine use of `ProgramEnvironment` construction with a narrow adapter from the resolved result.
8. Update tests to cover both the new resolver API and the slimmer engine boundary.
9. Run `nao check`.

# What verification should this work include?

Verification should include:

- resolver tests for successful multi-module resolution through the new public entry point
- resolver tests proving `ResolvedProgram` includes source diagnostics and a populated symbol table
- resolver tests for declaration registration across multiple modules and core module seeding
- engine tests proving `EngineWorker` no longer depends on low-level resolver steps
- engine tests for resolver failures still surfacing user-facing diagnostics correctly
- running `nao check`

Where possible, the new tests should assert the boundary, not the implementation details.
If a test in the engine crate still needs to know about per-module environments or pass ordering, it is probably testing the wrong layer.

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes the resolver crate can depend on `ParsedModule` without introducing a crate cycle. If that is not currently possible, `ParsedModule` may need to move into a shared crate before the simplified API can exist cleanly.
- `ResolvedProgram` returning `SymbolTable` instead of `ProgramEnvironment` is intentional. Runtime execution can derive what it needs later, but the semantic boundary should expose semantic data first.
- `CompilationContext.symbol_table` currently looks underused relative to the local symbol table in `EngineWorker`. This refactor is a good opportunity to decide whether that field should become the single symbol-table owner or be removed.
- The existing single-compilation-unit `resolve()` helper in [`crates/resolver/src/resolution.rs`](/data/projects/ocelot/crates/resolver/src/resolution.rs) may become a thin wrapper over `resolve_program` or may need to be deprecated if it creates duplicate logic.
- Builtin modules are expected to already be present in `Vec<ParsedModule>`. Adding a separate `builtin_modules` parameter would duplicate the same information and make the boundary worse, not better.
- `CompilationSession` should remain an explicit resolver input. `ParsedModule` contains parsed syntax, including `native fn` declarations from builtin modules, but the boxed native implementations still come from the compilation session registry.
- The engine currently uses [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs) directly during interpretation. If that remains true, introduce an explicit conversion step from `SymbolTable` to `ProgramEnvironment` rather than quietly rebuilding the old flow inside `EngineWorker`.

# What landed from this plan?

This refactor landed the simplified multi-module resolver boundary:

- [`ParsedModule`](/data/projects/ocelot/crates/semantic/src/parsed_module.rs), [`SourceFileKind`](/data/projects/ocelot/crates/semantic/src/source_file_kind.rs), and [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs) now live in [`ocelot-semantic`](/data/projects/ocelot/crates/semantic/src/lib.rs), so both the engine and resolver can share the same program-boundary types without a crate cycle
- [`ocelot_resolver::resolution::resolve_program()`](/data/projects/ocelot/crates/resolver/src/resolution.rs) now owns multi-module resolution orchestration and returns a [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs)
- module validation moved out of [`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) and into the resolver boundary, including duplicate builtin-module name diagnostics based on the parsed module set
- [`EngineWorker::resolve_modules()`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) is now a thin wrapper around `resolve_program`
- the engine now stores `ResolvedProgram`, reads diagnostics from it, and derives [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs) through [`ResolvedProgram::program_environment()`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs)
- `CompilationContext.symbol_table` did not become the working mutable symbol table for multi-module resolution; instead, the resolver keeps a local symbol table during orchestration and writes the final semantic table into the returned result
- resolver tests now cover the public `resolve_program()` boundary directly, and engine tests still cover resolver-error surfacing through the worker
- `cargo test -p ocelot-resolver -p ocelot-engine` and `nao check` pass

# What concrete tasks should track this plan?

- [x] Add `ResolvedProgram` with resolved modules, `SourceDiagnostics`, and `SymbolTable`.
- [x] Place `ResolvedProgram` in a crate shared cleanly by the engine and resolver without introducing a dependency cycle.
- [x] Add one public resolver entry point that accepts `Vec<ParsedModule>` plus `CompilationSession` and returns `ResolvedProgram`.
- [x] Move multi-module resolution orchestration out of `EngineWorker` and into that resolver entry point.
- [x] Keep `EngineWorker::resolve_modules()` as a thin wrapper around the new resolver entry point.
- [x] Decide whether `CompilationContext.symbol_table` becomes authoritative or is removed from this workflow.
- [x] Update `EngineWorker` to call only the new resolver API after parsing.
- [x] Keep interpreter setup working through an explicit adapter from the resolved semantic output.
- [x] Add resolver and engine tests for the new boundary and error-reporting behavior.
- [x] Run `nao check`.
