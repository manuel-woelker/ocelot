# Why does the current engine, resolver, and execution structure still feel too hard to follow?

The recent resolver-boundary cleanup moved multi-module orchestration out of the engine, but the overall compilation and execution model is still carrying too many overlapping concepts.

Today the same logical program state is represented in several different ways:

- [`CompilationContext`](/data/projects/ocelot/crates/semantic/src/compilation_context.rs) stores diagnostics and a symbol table, but the resolver mostly treats it as a diagnostics bag
- [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) is the resolver’s real semantic index during orchestration
- [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs) duplicates most of the symbol table shape for the interpreter
- [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs) stores `modules`, `source_diagnostics`, and `symbol_table`, then rebuilds a `ProgramEnvironment` on demand
- [`ModuleEnvironment`](/data/projects/ocelot/crates/semantic/src/module_environment.rs) is a very small file-local import map, but its generic name makes it sound more important and broader than it is
- [`CompilationSession`](/data/projects/ocelot/crates/semantic/src/compilation_session.rs) carries only the native function registry, which is real input state, but its name suggests a larger lifetime and responsibility than it currently has

That layering creates avoidable cognitive load:

- there is no obvious single source of truth for “the resolved program”
- the reader has to learn which structs are persistent artifacts and which are scratch state
- the engine still owns transient pipeline fields like parsed modules, parser diagnostics, resolved program, and test summaries instead of working with one explicit compilation result
- interpreter entry points accept a different semantic representation than the resolver returns

This plan should simplify the model so the architecture reads more like:

1. load and parse source files
2. resolve them into one runtime-ready program artifact plus diagnostics
3. execute a selected entrypoint or test against that artifact

# What should the simplified architecture look like?

The target model should separate three responsibilities cleanly:

- stable compiler inputs
- resolver scratch state
- stable compiled program output

Recommended target types:

- keep one small compiler input type, renamed from [`CompilationSession`](/data/projects/ocelot/crates/semantic/src/compilation_session.rs) to something like `CompilationInputs` or `ResolverInputs`
- replace [`CompilationContext`](/data/projects/ocelot/crates/semantic/src/compilation_context.rs) with a narrower diagnostics-focused type if it no longer owns authoritative semantic state
- rename [`ModuleEnvironment`](/data/projects/ocelot/crates/semantic/src/module_environment.rs) to something like `ModuleImports` or `ModuleScope` to reflect that it is scratch data for imported bindings, not a general environment
- make [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs) the single canonical compiled program artifact
- remove the semantic duplication between [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) and [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs) so only one program-wide table survives long term

The important simplification is not mostly naming.
The important simplification is reducing persistent semantic representations from two program-wide tables to one.

# Which structures should remain, be renamed, or be removed?

Recommended direction:

- keep [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs), but make it runtime-ready instead of making callers reconstruct another program representation
- keep the [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) name as the canonical program-wide semantic model and remove [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs) as a long-term peer model
- remove `CompilationContext.symbol_table`, because it currently advertises ownership that the resolver workflow does not actually respect
- rename [`ModuleEnvironment`](/data/projects/ocelot/crates/semantic/src/module_environment.rs) to reflect its true scope
- rename [`CompilationSession`](/data/projects/ocelot/crates/semantic/src/compilation_session.rs) to reflect that it is immutable resolver input rather than a long-lived mutable session

The chosen consolidation for this plan is:

- treat [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) as the final canonical program model
- move any remaining execution-facing helper APIs from [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs) onto [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) or adjacent helper modules
- have [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs) store that canonical symbol table directly

Why this direction is preferable:

- `SymbolTable` is already the dominant name on the resolver side
- keeping the name avoids a churn-heavy rename that does not buy much architectural clarity
- the real simplification is deleting the duplicate model, not rebranding the surviving one
- interpreter-facing lookups can use the same canonical table once the missing runtime helpers move over

# How should the engine boundary become easier to understand?

[`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) is still acting as a stateful pipeline object with several partially overlapping fields:

- parser diagnostics
- parsed modules
- resolved program
- test run summary

That makes the happy path harder to read than it needs to be.

The engine should move toward a data flow like this:

1. create one execution request from the CLI-facing command
2. load and parse modules into one `ParsedProgram` or equivalent temporary value
3. ask the resolver for one `ResolvedProgram`
4. hand `ResolvedProgram` plus the selected entry target to execution helpers

Recommended engine refactor:

- shrink [`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) into a thin orchestration object or replace it with pure helper functions
- stop storing `parsed_modules` as long-lived worker state once parsing is complete
- replace `parser_source_diagnostics` plus `resolved_program` with one enum or result object that can expose diagnostics from either stage consistently
- keep test aggregation separate from compilation state so test summary handling does not live on the same object as parser and resolver pipeline state

One concrete shape worth targeting is to store the execution target directly in [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs):

```rust
pub struct ResolvedProgram {
    pub entry_path: FilePath,
    pub modules: Vec<ParsedModule>,
    pub source_diagnostics: SourceDiagnostics,
    pub symbol_table: SymbolTable,
}
```

Then execution helpers can take `&ResolvedProgram` and a run mode instead of relying on mutable worker fields or an extra wrapper type.

# How should resolution and execution interact after the simplification?

The resolver should return a program artifact that is immediately usable by execution.
Execution should not need to know how to convert between semantic table formats.

Recommended boundary:

- resolver input: parsed modules plus immutable compilation inputs
- resolver output: `ResolvedProgram { entry_path, modules, diagnostics, symbol_table }`
- execution input: `ResolvedProgram` plus an entry selection such as run-file, run-test, or run-tests

Execution helpers should look up:

- the entry module from `ResolvedProgram.entry_path` plus `ResolvedProgram.modules`
- the entry function from `ResolvedProgram.symbol_table`
- test bodies from the resolved module AST

That means the engine no longer needs helper methods like `create_compilation_session()` and `program_environment()` on the worker.
Those methods are currently adapter noise caused by the split between resolver output and interpreter input.

# What implementation order should be used?

1. Introduce an active plan for the semantic-model simplification.
2. Keep [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) as the canonical program-wide semantic structure.
3. Update [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs) to store `entry_path` and the canonical symbol table directly.
4. Remove the duplicate conversion path between [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs), [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs), and [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs).
5. Narrow [`CompilationContext`](/data/projects/ocelot/crates/semantic/src/compilation_context.rs) so it only owns diagnostics, or replace it with a more explicit diagnostics accumulator type.
6. Rename [`ModuleEnvironment`](/data/projects/ocelot/crates/semantic/src/module_environment.rs) to match its actual role, and update resolver APIs accordingly.
7. Rename [`CompilationSession`](/data/projects/ocelot/crates/semantic/src/compilation_session.rs) to a name that reflects immutable compiler inputs.
8. Simplify [`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) so it passes through explicit artifacts instead of storing every intermediate stage as mutable fields.
9. Extract execution helpers that take `ResolvedProgram` directly for script execution, module entrypoint execution, and test execution.
10. Remove any remaining semantic adapters that only exist because of the old split representations.
11. Run targeted crate tests, then run `nao check`.

# What concrete refactors should happen in the semantic crate?

- [ ] Keep [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) as the canonical program-wide semantic representation and document that decision in the code.
- [ ] Update [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs) to store `entry_path` and the canonical symbol table directly.
- [ ] Remove `ResolvedProgram::program_environment()` if it becomes a transitional conversion helper.
- [ ] Delete [`ProgramEnvironment`](/data/projects/ocelot/crates/semantic/src/program_environment.rs) as a persistent peer model once its remaining APIs have moved.
- [ ] Remove `CompilationContext.symbol_table` from [`CompilationContext`](/data/projects/ocelot/crates/semantic/src/compilation_context.rs) if diagnostics are its only lasting job.
- [ ] Rename [`ModuleEnvironment`](/data/projects/ocelot/crates/semantic/src/module_environment.rs) to a name that communicates “module-local imported bindings”.
- [ ] Rename [`CompilationSession`](/data/projects/ocelot/crates/semantic/src/compilation_session.rs) to a name that communicates “immutable compilation inputs”.
- [ ] Update RustDoc comments so readers can tell which types are compiler inputs, scratch state, and final artifacts.

# What concrete refactors should happen in the resolver crate?

- [ ] Change resolver orchestration to build the canonical [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) directly instead of building one table and converting it into another.
- [ ] Replace generic “context” and “environment” parameter names with names tied to their actual role after the type renames land.
- [ ] Make the resolver use one clearly named diagnostics accumulator instead of a pseudo-context that appears broader than it is.
- [ ] Keep module-local import state as resolver scratch data only; do not leak it into the final compiled program artifact.
- [ ] Collapse any helper APIs that exist only to support both the old single-file and multi-file semantic representations.
- [ ] Update resolver tests to assert the new canonical `ResolvedProgram` shape and remove assertions that depend on transitional conversion helpers.

# What concrete refactors should happen in the engine and interpreter crates?

- [ ] Replace worker-local semantic adapters in [`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) with direct use of `ResolvedProgram.entry_path`, `ResolvedProgram.modules`, and `ResolvedProgram.symbol_table`.
- [ ] Move parsing results through the engine as local variables or a dedicated temporary artifact rather than persistent mutable worker fields where possible.
- [ ] Extract execution helpers that take a resolved program and an entry selection rather than reading from worker state.
- [ ] Keep parser diagnostics and resolver diagnostics available through one obvious access path for user-facing error rendering.
- [ ] Update interpreter entry points to consume the canonical semantic program type directly.
- [ ] Remove any now-redundant helper methods that only exist to convert between semantic representations.

# How should this work be verified?

Verification should prove both behavioral correctness and architectural simplification.

- [ ] Add semantic or resolver tests proving the chosen canonical program representation contains everything needed for execution without a secondary conversion step.
- [ ] Add resolver tests covering multi-module imports, builtin registration, function resolution, and effect propagation through the simplified artifact.
- [ ] Add engine tests covering script execution, module `main()` execution, targeted test execution, and all-tests execution through the new boundary.
- [ ] Add tests that compilation failures still preserve diagnostics for parser and resolver stages.
- [ ] Remove or rewrite tests that lock in the old duplicated table model.
- [ ] Run `cargo test -p ocelot-semantic -p ocelot-resolver -p ocelot-engine -p ocelot-interpreter`.
- [ ] Run `nao check`.

# What risks, assumptions, and open questions should stay explicit?

- This plan now assumes [`SymbolTable`](/data/projects/ocelot/crates/semantic/src/symbol_table.rs) remains the canonical name for the single surviving program-wide semantic model.
- Adding `entry_path` to [`ResolvedProgram`](/data/projects/ocelot/crates/semantic/src/resolved_program.rs) is a good trade if one resolved program always corresponds to one engine command target. If the same resolved artifact later needs to support multiple entrypoints without recompilation, `entry_path` may need to move back out into a thinner execution request type.
- If future compiler stages need both a declaration-phase table and a runtime-ready program model, that split should be made explicit with narrower names such as `DeclarationTable` and `ResolvedProgramState`, not left as two near-identical structs.
- [`resolve()`](/data/projects/ocelot/crates/resolver/src/resolution.rs) for single-compilation-unit resolution may be worth rewriting as a thin wrapper around the multi-module boundary once the canonical model is chosen. Keeping two separate resolution flows would reintroduce the same confusion.
- The engine simplification should avoid prematurely deleting [`EngineWorker`](/data/projects/ocelot/crates/engine/src/engine_worker.rs) if it still provides a useful seam for test execution orchestration. The real goal is removing hidden mutable pipeline state, not forcing everything into free functions.
- If `CompilationSession` is renamed, the new name should be chosen carefully. `Inputs` is clearer than `Session`, but it may still be too broad if native functions remain the only payload.
- This plan intentionally prioritizes understandability over micro-optimizations. If the canonical program model allows both resolution and execution without cloning whole tables, that is a bonus, but not the primary goal.
