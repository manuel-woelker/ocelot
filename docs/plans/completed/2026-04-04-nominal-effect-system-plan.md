# Why add a lightweight nominal effect system now?

The repository already lists a lightweight effect type system as a desired language feature, and the current compiler pipeline is finally at the point where it can support one cleanly.
Function declarations, call resolution, type annotations, and resolver diagnostics already exist, which means effect tracking can ride on real semantic data instead of being bolted onto the interpreter later.

This slice should introduce explicit nominal effect declarations such as `effect exec;`, function annotations such as `can exec` and `cannot exec`, including the case where both appear on the same function, and resolver-time propagation of effects upward through the call graph.
The first implementation should stay deliberately narrow: nominal names, function-level allow/forbid annotations, builtin effects for `println` and assertions, and diagnostics when inferred effects violate a prohibition.

# What language behavior should this slice define?

The intended first behavior is:

- top-level effect declarations use `effect <name>;`
- functions may declare allowed effects with `fun foo() can exec {}`
- functions may declare forbidden effects with `fun bar() cannot exec {}`
- functions may declare both clauses with `fun baz() can exec cannot panic {}`
- when both clauses are present, `can` always precedes `cannot`
- unannotated functions do not need to list every effect explicitly
- effects propagate upward through calls, so a caller inherits the effects of the functions it calls
- a function annotated with `cannot <effect>` produces a resolver error if its body directly or transitively performs that effect
- builtin native functions contribute builtin effects:
  - `println` contributes `write_stdout`
  - `assert` contributes `panic`
  - `assert_eq` contributes `panic`

This plan should treat `can` as documentation plus explicit metadata, not as an exhaustive allow-list.
In other words, `can exec` means the function explicitly advertises that `exec` may happen, but the resolver should still infer effects from the body and should not require all inferred effects to be listed in `can`.
That keeps the feature lightweight and matches the requested "effects travel upward the call-graph" model.

# What is the current implementation gap?

Today:

- the lexer and parser do not recognize `effect`, `can`, or `cannot`
- there is no AST representation for declared effects or function effect annotations
- [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) tracks functions and types, but not effect definitions
- [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) stores argument types and callable kind, but not declared, forbidden, or inferred effects
- the resolver already walks function bodies and call expressions, but it does not compute any call-graph-derived metadata beyond function indices and simple type checks
- builtin behavior for `println`, `assert`, and `assert_eq` is known operationally, but not modeled semantically as effects

Without effect metadata in the AST and environment, there is nowhere reliable to record declarations, infer transitive effects, or report prohibition failures against stable source spans.

# What data model should represent effects?

The cleanest first design is to mirror the existing type and function tables.

Recommended additions:

- add an [`Effect`](/data/projects/ocelot/crates/ast/src/effect.rs) type with:
  - effect name
  - whether the effect is builtin or user-declared if that distinction proves useful in diagnostics
- add an [`EffectIndex`](/data/projects/ocelot/crates/ast/src/effect_index.rs) handle type instead of passing raw integers
- extend [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) with:
  - `effects: Vec<Effect>`
  - `effect_symbols: HashMap<SharedString, EffectIndex>`
  - helpers such as `resolve_effect()`, `effect_definition()`, and builtin effect accessors
- seed builtin effects during environment construction:
  - `write_stdout`
  - `panic`

Function metadata should also grow effect fields:

- extend [`FunctionItem`](/data/projects/ocelot/crates/ast/src/function_item.rs) with parsed effect annotations so source intent survives into resolution
- extend [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) with:
  - explicitly declared `can` effects
  - explicitly declared `cannot` effects
  - inferred or resolved effects for the function body
  - source spans for effect clauses if diagnostics need to point at the annotation rather than the body

Use sets, not lists with duplicates.
The obvious shape is `BTreeSet<EffectIndex>` or `HashSet<EffectIndex>`, but if deterministic ordering matters in tests and diagnostics, `BTreeSet` is the less annoying choice.

# What syntax and AST changes are needed?

The parser currently only knows `fun`, `test`, identifiers, literals, and simple expressions, so effect syntax needs a complete front-to-back pass:

1. Add lexer keywords for `effect`, `can`, and `cannot`.
2. Extend the AST item model with a top-level effect declaration item, likely something like [`EffectItem`](/data/projects/ocelot/crates/ast/src/effect_item.rs).
3. Extend [`ItemKind`](/data/projects/ocelot/crates/ast/src/item_kind.rs) so scripts can contain effect declarations alongside functions, tests, and statements.
4. Extend [`FunctionItem`](/data/projects/ocelot/crates/ast/src/function_item.rs) so it can carry optional `can` and `cannot` clauses parsed between `)` and `{`, including both on the same function.
5. Add a small AST type for effect clauses rather than stuffing raw identifiers directly onto `FunctionItem`.

Recommended grammar shape for the first slice:

- `effect <identifier>;`
- `fun <identifier>() { ... }`
- `fun <identifier>() can <identifier> { ... }`
- `fun <identifier>() cannot <identifier> { ... }`
- `fun <identifier>() can <identifier> cannot <identifier> { ... }`

Ordering should be strict:

- if both clauses are present, `can` must come before `cannot`
- `cannot ... can ...` should be rejected in the parser as invalid function annotation order

Keep the first version to exactly one effect name per clause.
Supporting comma-separated effect lists is possible, but it adds parser and diagnostic surface area immediately and is not required by the request.
If multiple effects are needed later, a follow-up can extend `can` and `cannot` to accept lists without invalidating the core data model.

# How should resolution and propagation work?

Effect checking belongs in the resolver, not in the parser and not in the interpreter.
The resolver already has the right responsibilities:

- it registers functions before bodies are resolved
- it walks statements and expressions recursively
- it owns source diagnostics
- it has access to the shared [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs)

Recommended resolution strategy:

1. Register top-level effect declarations before resolving function bodies, similar to the existing function-registration prepass.
2. Resolve effect names used in `can` and `cannot` clauses to `EffectIndex` values and store them in [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs).
3. Associate builtin native functions with builtin effect sets in the environment:
   - `println` => `write_stdout`
   - `assert` => `panic`
   - `assert_eq` => `panic`
4. While resolving a function body, collect direct effect dependencies from calls:
   - calls to native functions contribute their builtin effect sets
   - calls to user-defined functions contribute an edge from caller to callee
5. After all functions have registered their direct dependencies, compute the transitive closure of effects over the user-defined call graph.
6. Compare each function's inferred effect set against its `cannot` set and emit diagnostics for conflicts.
7. Store the final inferred effect set on the function definition so later phases and tests can inspect it without recomputing.

There are two sensible implementation shapes for step 5:

- iterative fixed-point propagation over function definitions
- a graph walk over strongly connected components

The fixed-point version is the simplest correct first implementation and is probably enough here.
Recursive functions and cycles matter, but they do not require a fancy algorithm if the effect sets are monotonic and finite.

# How should diagnostics behave?

There are three core diagnostic categories for this slice:

- unknown effect name
- duplicate effect declaration
- forbidden effect violation

Recommended behavior:

- `effect exec; effect exec;` should be a resolver error with the second declaration pointing back to the first
- `fun foo() can missing {}` should be a resolver error on `missing`
- `fun foo() cannot exec { run(); }` should be a resolver error once `run()` is known or inferred to have `exec`
- the error should mention both the forbidden effect name and the function where the violation occurs
- the primary span should usually point at the offending call site in the body
- a secondary excerpt may point at the `cannot exec` clause when that improves readability

Do not report speculative diagnostics just because a function has `can exec`.
The only hard failure requested is when an inferred effect clashes with a forbidden effect.

Also move builtin argument-count validation for `println` out of the parser if that becomes necessary to keep effect diagnostics and function-call semantics in one place.
The current parser-side check is fine for now, but effect checking will make the parser an increasingly bad home for semantic call rules.

# What spec and documentation changes should land with this work?

This feature changes surface syntax and semantics, so it should not land as code only.

Recommended documentation work:

- add a new spec chapter under `27`, likely `docs/spec/27.01 Effects - Nominal effect declarations.md`
- update [`docs/spec/15.02 Declarations - Function definitions.md`](/data/projects/ocelot/docs/spec/15.02%20Declarations%20-%20Function%20definitions.md) to mention optional `can` and `cannot` clauses
- update [`docs/spec/30.01 Standard library - println.md`](/data/projects/ocelot/docs/spec/30.01%20Standard%20library%20-%20println.md) and [`docs/spec/30.02 Standard library - assert.md`](/data/projects/ocelot/docs/spec/30.02%20Standard%20library%20-%20assert.md) so builtin effects are explicit
- update [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md) to list the new effect chapter

The effect spec should include at least:

- declaring one nominal effect
- a function that explicitly says `can exec`
- effect propagation through one or more calls
- a `cannot` violation with a stable diagnostic example
- builtin effect examples for `println` and `assert`

# What implementation order keeps the work manageable?

1. Add AST support for effect declarations, effect indices, and function effect metadata.
2. Seed builtin effects and effect symbol lookup in [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs).
3. Extend the lexer and parser to recognize `effect`, `can`, and `cannot`, plus top-level effect items.
4. Register effect declarations before function-body resolution and diagnose duplicate declarations.
5. Resolve effect names used in function annotations and diagnose unknown effects.
6. Attach builtin effect sets to native functions.
7. Extend the resolver to record direct call dependencies and direct builtin effects per function.
8. Add a fixed-point propagation step that computes transitive inferred effects for all user-defined functions.
9. Diagnose conflicts between inferred effects and `cannot` clauses.
10. Add or update spec chapters and examples.
11. Run `nao check`.

# What verification should this work include?

Verification should include:

- lexer tests for `effect`, `can`, and `cannot`
- parser tests for:
  - `effect exec;`
  - `fun foo() can exec {}`
  - `fun foo() cannot exec {}`
  - `fun foo() can exec cannot panic {}`
  - rejecting `fun foo() cannot panic can exec {}`
  - preserving effect annotation spans and names in the AST
- program-environment tests for:
  - builtin effect seeding
  - effect symbol lookup
  - stable effect indices
- resolver tests for:
  - direct builtin effect inference from `println`, `assert`, and `assert_eq`
  - upward propagation across one and multiple function calls
  - forward references still working with effect propagation
  - recursive functions reaching a stable inferred effect set
  - `cannot` rejecting direct violations
  - `cannot` rejecting transitive violations
  - unknown effect names in declarations and annotations
  - duplicate effect declarations
- spec validation updates if the spec harness already covers the new examples
- running `nao check`

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes effect names live in a global nominal namespace per compilation environment, not per module. That is the simplest first model, but it should be made explicit in the implementation and spec.
- This plan assumes builtin effects do not require explicit `effect write_stdout;` or `effect panic;` declarations by users. They should be seeded automatically.
- This plan treats `can` as optional explicit metadata, not as a complete list of all effects the function may have. If the language later wants an exhaustive effect signature, that is a different and stricter feature.
- This plan assumes a function may carry both `can` and `cannot`, and that the grammar enforces `can` before `cannot`. That keeps parsing deterministic and matches the requested surface syntax.
- It is still an open question whether top-level executable statements should participate in effect checking as an implicit script function. The simplest first slice is to limit `can` and `cannot` to named functions and let top-level script execution remain unconstrained.
- Another open question is whether test items should implicitly allow builtin effects such as `write_stdout` and `panic`, or whether tests should later gain the same annotation model as functions. That is probably follow-up work, not part of this slice.
- If user-defined functions are allowed to declare `can exec` without a matching `effect exec;`, the model stops being nominal and gets sloppy fast. The resolver should require explicit effect declarations for user-defined names, while still exempting seeded builtin effects.
- The current parser enforces `println` arity directly. If more semantic validation moves into the resolver during this work, watch out for duplicated or inconsistent diagnostics across phases.

# What landed from this plan?

This slice landed a lightweight nominal effect system across the parser, AST, resolver, and spec:

- the lexer and parser now recognize `effect`, `can`, and `cannot`
- scripts can declare top-level effects and functions can carry `can` and `cannot` clauses, including the combined `can ... cannot ...` form
- [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) now seeds builtin `write_stdout` and `panic` effects and stores user-declared nominal effects
- [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) now stores declared effects, direct effects, transitive inferred effects, and effect dependency metadata for diagnostics
- the resolver now registers effect declarations before function registration, resolves effect names in function annotations, propagates effects upward through the user-defined call graph, and reports duplicate, unknown, and forbidden-effect diagnostics
- builtin function effects are modeled semantically:
  - `println` contributes `write_stdout`
  - `assert` contributes `panic`
  - `assert_eq` contributes `panic`
- the spec now documents nominal effect declarations, propagation, and builtin effects
- verification now includes expanded AST, parser, and resolver tests plus `cargo test --workspace` and `nao check`

# What concrete tasks should track this plan?

- [x] Add AST types and module wiring for effect declarations, effect indices, and function effect metadata.
- [x] Seed builtin effects and effect-symbol lookup in [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs).
- [x] Extend the lexer and parser for `effect`, `can`, and `cannot` syntax.
- [x] Register effect declarations and diagnose duplicate effect names.
- [x] Resolve effect names in function annotations and diagnose unknown effects.
- [x] Associate builtin effect sets with native functions.
- [x] Record direct call dependencies and direct builtin effects during resolution.
- [x] Implement fixed-point propagation of inferred effects across user-defined functions.
- [x] Report diagnostics when inferred effects conflict with `cannot` clauses.
- [x] Add or update spec chapters and examples for effect declarations, propagation, and diagnostics.
- [x] Add colocated tests across AST, parser, and resolver coverage.
- [x] Run `nao check`.
