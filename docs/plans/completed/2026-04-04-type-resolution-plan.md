# Why add explicit type metadata now?

The AST and resolver already carry resolved function information, but expressions still have no place to store even the most basic type facts.
That makes the next semantic steps harder than they need to be because every later pass would need to rediscover obvious cases like string literals, boolean literals, and boolean negation.

Adding a small type table now gives the project a stable place to put:

- canonical primitive types shared across the compilation pipeline
- expression-level type annotations filled in by a resolver phase
- typed handles instead of raw integers when future declarations and signatures arrive

# What should this slice include?

This slice should include:

- a `Ty` struct with a name and a `TypeKind`
- a `TypeKind` enum with only the current primitive cases needed now
- a `TypeIndex` handle type where index `0` means unresolved
- a type table plus `type_symbols` lookup on [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs)
- an expression-level `ty: TypeIndex` field that defaults to unresolved when parsing
- a type-resolution pass that fills in `expression.ty` where this slice has enough information

This slice should not yet include:

- user-declared types
- type syntax in the parser
- variable or parameter type resolution
- function signatures or return-type inference
- a full value-name resolver

# What is the current implementation gap?

Today:

- [`Expression`](/data/projects/ocelot/crates/ast/src/expression.rs) stores only `kind` and `span`
- [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) owns only function data
- the resolver crate resolves function calls, but it does not annotate expression types
- there is no canonical table for primitive type metadata shared across parsing, resolution, and later execution phases

That means even the obvious cases such as `"hello"` being a string and `not true` being a boolean are not represented anywhere in the tree.

# What data model should represent types?

The first implementation should mirror the existing function-table pattern closely, but not blindly.

Recommended shape:

- add [`Ty`](/data/projects/ocelot/crates/ast/src/ty.rs) with:
  - `name`
  - `kind`
- add [`TypeKind`](/data/projects/ocelot/crates/ast/src/type_kind.rs) with only:
  - unresolved
  - string
  - boolean
- add [`TypeIndex`](/data/projects/ocelot/crates/ast/src/type_index.rs) as a small typed wrapper around `u32`
- reserve `TypeIndex(0)` as the unresolved sentinel instead of using `Option<TypeIndex>`
- initialize table slot `0` with a real `Ty` entry whose kind is `TypeKind::Unresolved`

That last point is intentionally different from [`FunctionIndex`](/data/projects/ocelot/crates/ast/src/function_index.rs).
The user-facing requirement is that every expression always has a `ty` field, and the default parser value should be unresolved.
Using an always-present `TypeIndex` makes that cheaper and keeps equality assertions in tests straightforward.

The environment should own:

- `types: Vec<Ty>` with slot `0` reserved for the unresolved sentinel type
- `type_symbols: HashMap<SharedString, TypeIndex>`
- helpers such as `resolve_type()` and `type_definition()`

The primitive seed set should be minimal:

- `string`
- `boolean`

# How should expressions carry type information?

[`Expression`](/data/projects/ocelot/crates/ast/src/expression.rs) should gain a `ty: TypeIndex` field.

Recommended behavior:

- `Expression::new(...)` initializes `ty` to `TypeIndex::unresolved()`
- parser tests continue to construct expressions through `Expression::new(...)` so the default stays centralized
- resolver code mutates `expression.ty` in place once the type is known

That keeps parsing simple and makes unresolved-vs-resolved state explicit in the AST rather than implicit in phase ordering.

# How should type resolution work in this slice?

The simplest correct design is to extend the existing resolver traversal so it performs type annotation after the expression tree below a node has already been visited.

Recommended behavior:

1. Seed primitive types into [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) before resolution begins.
2. Reuse the current recursive traversal in [`crates/resolver/src/lib.rs`](/data/projects/ocelot/crates/resolver/src/lib.rs).
3. After resolving child expressions, assign `expression.ty` for the cases this slice can fully understand.

Initial typing rules:

- string literal expressions resolve to the `string` type
- boolean literal expressions resolve to the `boolean` type
- `not <expr>` requires the operand to resolve to `boolean` and resolves the result to `boolean`
- identifier expressions remain unresolved for now
- call expressions remain unresolved for now unless this slice also introduces callable type metadata

This is the important constraint in the current design: the project does not yet have variable symbols or function return types, so pretending every expression can be typed now would be overengineered and wrong.
The first pass should type what it truly knows and leave the rest as `TypeIndex(0)`, which points at the canonical unresolved type-table entry.

# How should diagnostics behave?

The type-resolution phase should report real semantic errors when it has enough information to do so.

Recommended diagnostics for this slice:

- `not "hello"` should fail in the resolver/type-resolution stage with a message that the operand must be boolean
- the diagnostic span should point at the operand expression or the `not` expression span, whichever reads better in the existing renderer

Non-goals for this slice:

- reporting unknown identifier types
- checking function-call argument types
- inferring return types for calls

Those cases should remain unresolved rather than producing speculative or misleading errors.

# How should the implementation be staged?

1. Add AST modules for `Ty`, `TypeKind`, and `TypeIndex`, following the repository rule that each type lives in its own file.
2. Extend [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) with a seeded type table and type-symbol lookup.
3. Extend [`Expression`](/data/projects/ocelot/crates/ast/src/expression.rs) with `ty: TypeIndex`, defaulting to unresolved.
4. Update AST, parser, and resolver tests that compare full expression values so they account for the new default field.
5. Extend the resolver traversal to annotate literals and `not` expressions with concrete type indices.
6. Add resolver diagnostics for invalid `not` operands.
7. Decide whether call expressions stay unresolved in this slice or whether function definitions need a small return-type model now.
8. Run `nao check`.

# What verification should this work include?

Verification should include:

- colocated AST tests for:
  - `TypeIndex(0)` behaving as unresolved
  - the type table storing `TypeKind::Unresolved` at slot `0`
  - seeded primitive types appearing in the environment
  - type-symbol lookups returning stable indices
- parser tests proving newly constructed expressions start with unresolved `ty`
- resolver tests proving:
  - string literals get the string type
  - boolean literals get the boolean type
  - nested `not` expressions resolve to boolean
  - `not` applied to a string literal reports a resolver diagnostic
  - identifiers and call expressions still carry unresolved type indices when no rule applies
- running `nao check`

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes the canonical primitive names should be `string` and `boolean`. If the language wants `bool` instead, decide that before tests and diagnostics hard-code the names.
- This plan assumes `TypeIndex` should allow `0` directly, unlike [`FunctionIndex`](/data/projects/ocelot/crates/ast/src/function_index.rs). Reusing the exact `NonZeroU32` shape would fight the requested unresolved sentinel.
- This plan now treats unresolved as both an index sentinel and a canonical table entry at slot `0`. That redundancy is intentional in this design so every `TypeIndex`, including the unresolved one, can be dereferenced through the same environment API.
- The biggest open question is call-expression typing. The current function model does not expose parameter or return types, so this plan should not fake resolved call types unless function metadata grows in the same change.
- If the resolver continues to own both function resolution and type resolution, its responsibilities are growing. That is acceptable for this slice, but if name resolution, type checking, and future flow analysis all pile into one file, it will get ugly fast.
- Adding `ty` to [`Expression`](/data/projects/ocelot/crates/ast/src/expression.rs) will fan out through many parser and interpreter tests because structural equality on expressions will now include the new field. That churn is expected and should be handled deliberately instead of papered over.

# What landed from this plan?

This change introduced the first explicit type metadata path in the compiler pipeline:

- [`Ty`](/data/projects/ocelot/crates/ast/src/ty.rs), [`TypeKind`](/data/projects/ocelot/crates/ast/src/type_kind.rs), and [`TypeIndex`](/data/projects/ocelot/crates/ast/src/type_index.rs) now model canonical types in the AST layer
- [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) now owns a seeded type table, with slot `0` initialized to `TypeKind::Unresolved` and primitive `string` and `boolean` entries indexed through `type_symbols`
- [`Expression`](/data/projects/ocelot/crates/ast/src/expression.rs) now carries a `ty` field that defaults to unresolved at parse time
- [`ocelot_resolver::resolve()`](/data/projects/ocelot/crates/resolver/src/lib.rs) now annotates string literals, boolean literals, and `not` expressions with concrete type indices
- invalid `not` operands such as `not "hello"` now fail during resolution instead of surfacing later as runtime type errors
- identifier and call expressions intentionally remain unresolved in this slice because the current function metadata does not yet expose return types
- `cargo test` and `nao check` pass

# What concrete tasks should track this plan?

- [x] Add AST modules for `Ty`, `TypeKind`, and `TypeIndex`.
- [x] Seed an unresolved type entry at slot `0`, plus primitive `string` and `boolean` entries in the program environment, and expose a `type_symbols` lookup.
- [x] Extend `Expression` with a `ty: TypeIndex` field that defaults to unresolved.
- [x] Update parser and AST tests for the new expression shape and unresolved default.
- [x] Extend the resolver to annotate string literals, boolean literals, and `not` expressions with concrete type indices.
- [x] Add resolver diagnostics and tests for invalid boolean negation operands.
- [x] Leave identifiers and call expressions unresolved unless callable type metadata is added in the same slice.
- [x] Run `nao check`.
