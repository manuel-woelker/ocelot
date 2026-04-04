# Why is function resolution needed now?

Function calls currently parse as generic call expressions, but the interpreter still decides what to execute by matching the callee name at runtime in [`Interpreter::evaluate_call_expression()`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs).
That keeps native calls working for `println`, `assert`, and `assert_eq`, but it means the resolver crate is still a stub and the runtime is doing semantic lookup work that should happen earlier.

Adding a real function table plus a resolver pass now gives the language a cleaner execution model:

- function lookup happens once after parsing
- unresolved function names fail in the resolver stage instead of during interpretation
- the interpreter can dispatch by stable function index instead of by string matching
- later user-defined functions can reuse the same call-site representation and lookup pattern

# What should this slice include?

This slice should include:

- a function definition table containing the existing native functions only: `println`, `assert`, and `assert_eq`
- a symbol table that maps function names to indices in that function table
- a resolver pass that walks the parsed script and resolves call expressions to function indices
- storing the resolved function index directly in `CallExpression`
- interpreter dispatch through the resolved function index instead of the callee name
- engine integration so scripts and tests run the resolver step after parsing and before interpretation

This slice should not yet include:

- user-defined function declarations
- lexical scopes for function declarations
- first-class function values
- overload resolution or signature-based dispatch

# What is the current implementation gap?

Today:

- [`CallExpression`](/data/projects/ocelot/crates/ast/src/call_expression.rs) stores a callee expression and argument list, but no resolution result
- [`ocelot_resolver::resolve()`](/data/projects/ocelot/crates/resolver/src/lib.rs) returns `Ok(())` without traversing the AST
- [`Engine::run_script()`](/data/projects/ocelot/crates/engine/src/engine.rs), [`Engine::run_test()`](/data/projects/ocelot/crates/engine/src/engine.rs), and [`Engine::run_tests()`](/data/projects/ocelot/crates/engine/src/engine.rs) parse and then interpret directly
- [`Interpreter::evaluate_call_expression()`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) still extracts an identifier and matches on its name

That split is awkward now and will get worse once there is more than one category of callable thing.

# What data model should function resolution use?

The implementation should add an explicit native-function registry instead of leaving native function knowledge scattered across the interpreter.
That registry can live inside a small shared environment struct that owns execution-global data for one engine run.

Recommended shape:

- add a `ProgramEnvironment` struct that owns:
  - the function definition table
  - the function-name symbol table
  - future execution-global data as the interpreter grows
- add a `FunctionIndex` newtype wrapper around the underlying table index instead of passing around raw `usize`
- add a small `FunctionDefinition` type for native functions
- store the definitions in a `Vec<FunctionDefinition>` so resolved handles are cheap to copy into the AST
- build a symbol table from function name to function index, likely `HashMap<SharedString, FunctionIndex>` or `HashMap<String, FunctionIndex>`
- keep the initial registry centralized inside `ProgramEnvironment` so both the resolver and interpreter can rely on the same source of truth
- initialize `ProgramEnvironment` with the currently supported native functions: `println`, `assert`, and `assert_eq`

For call sites:

- extend `CallExpression` with a `resolved_function_index: Option<FunctionIndex>` field
- parser-created call expressions should start with `None`
- the resolver should replace that with `Some(index)` for successfully resolved function calls
- add a helper on `CallExpression` that returns `OcelotResult<FunctionIndex>` so downstream code does not need to unwrap the option manually

Using `Option<FunctionIndex>` keeps the parser simple, makes unresolved-vs-resolved state explicit in tests, and prevents unrelated table indices from being mixed up accidentally.

# How should the resolver pass work?

The resolver should become a real AST traversal that mutates the parsed script in place with resolved call metadata.

Recommended behavior:

1. Construct or receive a `ProgramEnvironment` at resolver entry.
2. Walk all top-level statements and all test bodies.
3. Recursively visit expressions so nested call expressions are also resolved.
4. For each call expression:
   - require the callee to remain an identifier for this slice
   - look up the identifier name in `ProgramEnvironment`'s function symbol table
   - write the resolved function index into the call expression
   - emit a resolver error for unknown function names
5. Leave non-call identifiers untouched so existing runtime behavior for unresolved value names remains unchanged.

The resolver signature should change to mutate in place:

- change `resolve(&Script)` to a mutable API
- thread `SourceFile` and `CompilationContext` through the resolver so it can report user-facing resolver diagnostics cleanly

That is the cleanest fit for the requested behavior and avoids needless AST cloning.

# How should diagnostics behave?

Unknown function names should stop being generic runtime errors and become resolver-stage failures.

Recommended diagnostics:

- unknown called identifier like `printline("x");` should produce a resolver diagnostic such as `unknown function \`printline\``
- source spans should point at the callee identifier, not the entire call
- non-call unresolved identifiers such as `println(missing_value);` should remain runtime diagnostics for now, because this plan is only resolving functions

This keeps the phase boundary clear without overreaching into general name resolution yet.

# How should the interpreter change?

The interpreter should stop doing function-name lookup and treat call expressions as already resolved.

Recommended changes:

- replace string-based dispatch with index-based dispatch
- centralize native function execution behind `ProgramEnvironment` and a helper that takes a function index
- use the `CallExpression` helper returning `OcelotResult<FunctionIndex>` when interpreter code needs the resolved index, so the invariant is checked in one place
- keep native argument validation where it already belongs today unless this refactor reveals an obvious cleanup

This removes repeated string matching from runtime execution and makes it obvious when the resolver step was skipped.

# How should the engine pipeline change?

The engine should treat resolution as a mandatory compilation step between parsing and interpretation.

Recommended order:

1. load source
2. parse into `Script`
3. resolve function calls
4. interpret the resolved script or discovered test bodies

Concretely, this likely means:

- adding an engine helper that parses and resolves before returning the script
- using that helper from `run_script`, `discover_tests`, `run_test`, and `run_tests`
- ensuring resolver failures render with the same user-facing quality as existing parser/runtime errors

`discover_tests` does not need resolved calls to list test names, but running the resolver there keeps the pipeline consistent and catches invalid test files early.

# What implementation order keeps the work manageable?

1. Add an active plan document for function resolution.
2. Introduce `ProgramEnvironment`, native function definition data, and a shared symbol-table builder.
3. Seed `ProgramEnvironment` with `println`, `assert`, and `assert_eq`.
4. Extend `CallExpression` so parser-created call nodes can carry an optional resolved function index.
5. Implement AST traversal in `ocelot-resolver` to resolve call expressions in statements and test bodies.
6. Add resolver tests for successful resolution, unknown functions, nested calls, and test-body calls.
7. Update the engine to run the resolver after parsing in every relevant execution path.
8. Update the interpreter to dispatch by resolved function index and fail loudly if a call was not resolved.
9. Refresh interpreter and engine tests so unknown native functions fail in the resolver stage instead of at runtime.
10. Run `nao check`.

# What verification should this work include?

Verification should include:

- colocated resolver tests for:
  - resolving `println("hello");`
  - resolving `println`, `assert`, and `assert_eq` to valid function indices
  - resolving nested calls if the current grammar allows them
  - resolving calls inside test bodies
  - reporting unknown function names with resolver-stage errors and correct spans
- interpreter tests proving that resolved indices drive dispatch correctly
- engine tests covering:
  - successful script execution through parse -> resolve -> interpret
  - resolver failures rendered to users for unknown function names
  - test execution still working after the new pipeline step
- running `nao check`

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes function indices are created and consumed only through `ProgramEnvironment` and `FunctionIndex`, not by tests or callers depending on specific numeric values.
- The initial native-function set should be fully migrated into `ProgramEnvironment` in this change. Leaving some functions in the old interpreter string match would create a half-resolved design that is worse than either approach on its own.
- This plan assumes `FunctionIndex` is the only public way to reference entries in the function table. Falling back to raw `usize` in APIs would weaken the benefit of the newtype almost immediately.
- The current `CallExpression` shape still stores the callee expression. That is fine for now, but if callable expressions stay restricted to identifiers for a while, the AST may be carrying more generality than the runtime needs.
- There is a design choice between storing only the resolved index on the call or also keeping the original callee identifier for diagnostics and future tooling. Keeping both is the pragmatic choice for now.
- If `discover_tests` starts running resolution, callers may now see resolver errors earlier than before. That is probably the right UX, but it is a behavior change worth making explicit.
- This plan intentionally does not resolve ordinary identifiers. Mixing function resolution and value resolution in one pass right now would be overengineered and muddy the semantics.
- The `CallExpression` helper should be the preferred access path for resolved indices. Reaching into the raw option directly outside AST code would spread invariant handling back across the codebase.

# What landed from this plan?

This change landed the first native-function resolution pass:

- [`CallExpression`](/data/projects/ocelot/crates/ast/src/call_expression.rs) now stores an optional resolved [`FunctionIndex`](/data/projects/ocelot/crates/ast/src/function_index.rs) and exposes a helper returning `OcelotResult<FunctionIndex>`
- [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) now owns the native function table and symbol table, seeded with `println`, `assert`, and `assert_eq`
- [`ocelot_resolver::resolve()`](/data/projects/ocelot/crates/resolver/src/lib.rs) now mutates `&mut Script`, resolves call expressions across statements and test bodies, and reports resolver diagnostics for unknown functions and unsupported callees
- the engine now runs parse -> resolve -> interpret consistently for scripts, test discovery, single-test execution, and full test runs
- the interpreter now dispatches native calls through resolved function metadata instead of matching callee strings at runtime
- resolver, interpreter, and engine tests were updated to cover the new phase boundary
- `cargo test` and `nao check` pass

# What concrete tasks should track this plan?

- [x] Add a `ProgramEnvironment` that owns the native function definition table and symbol table shared by resolution and interpretation.
- [x] Add a `FunctionIndex` newtype and use it in the function symbol table, resolved call metadata, and interpreter dispatch helpers.
- [x] Seed `ProgramEnvironment` with `println`, `assert`, and `assert_eq`.
- [x] Extend `CallExpression` to store an optional resolved function table index.
- [x] Add a `CallExpression` helper returning `OcelotResult<FunctionIndex>` for resolved-index access.
- [x] Implement resolver traversal that resolves call expressions across scripts and test bodies.
- [x] Add resolver diagnostics and tests for unknown function names and correct callee spans.
- [x] Run the resolver from the engine after parsing and before interpretation.
- [x] Update the interpreter to dispatch calls by resolved function index instead of callee string matching.
- [x] Update interpreter and engine tests for the new phase boundary and dispatch model.
- [x] Run `nao check`.
