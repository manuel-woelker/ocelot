# Why is a first slice of function definitions needed now?

The language can already parse and resolve call expressions, but it still only knows about native functions seeded into [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs).
That makes function calls useful for `println`, `assert`, and `assert_eq`, but it leaves no syntax for declaring user-defined functions and no way for the resolver to discover them from source.

Adding a first slice of function definitions now should focus on declaration and resolution first, with only the minimum execution support needed to keep spec examples real.
That keeps the work small, gives the language a real `fun` declaration surface, and sets up the next slice to interpret richer user-defined calls without redesigning the symbol table again.

# What should this slice include?

This slice should include:

- parsing top-level function definitions introduced by the `fun` keyword
- supporting function names and empty parameter lists only, such as `fun greet() { ... }`
- storing user-defined functions as top-level items alongside statements and test items
- recording function bodies as statement lists
- extending resolution so function definitions are collected into the function table before call expressions are resolved
- spec chapters and executable examples for function definitions
- TextMate bundle updates so `fun` highlights as a declaration keyword

This slice should not yet include:

- function parameters
- return values or `return` statements
- closures, nested functions, or local function declarations
- parameters or argument binding for user-defined functions

# What is the current implementation gap?

Today:

- [`ItemKind`](/data/projects/ocelot/crates/ast/src/item_kind.rs) supports only statements and test items
- [`Parser::parse_item()`](/data/projects/ocelot/crates/parser/src/parser.rs) recognizes only `test` as a top-level declaration keyword
- the lexer does not currently emit a `fun` token
- [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) is built entirely from native functions supplied by the engine
- [`ocelot_resolver::resolve()`](/data/projects/ocelot/crates/resolver/src/lib.rs) performs a single traversal that resolves calls against the prebuilt environment
- the TextMate bundle in [`Ocelot.tmLanguage`](/data/projects/ocelot/support/ocelot.tmbundle/Syntaxes/Ocelot.tmLanguage) highlights `test` and `not`, but not `fun`

That is fine for native-only calls, but it is the wrong shape for source-defined functions because declarations must be registered before call sites are resolved.

# What AST and environment model should function declarations use?

The implementation should add a real top-level function declaration node instead of trying to squeeze user-defined functions into the existing native metadata type.

Recommended model:

- add a dedicated AST node for source function declarations, likely something like `FunctionItem` or `FunctionDeclaration`
- store:
  - the function name
  - the body statements
  - the full source span
- add `ItemKind::Function(...)` so functions remain top-level declarations like tests
- update [`Script::statements()`](/data/projects/ocelot/crates/ast/src/script.rs) and similar helpers so top-level executable statements still exclude functions and tests

For the shared function table:

- keep one [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) type as the table entry model
- introduce an enum inside that table entry so one function definition can represent either:
  - a native function
  - a user-defined function
- let the enum carry only the metadata each kind needs:
  - native functions can keep their existing builtin identifier
  - user-defined functions can store the parsed body and any declaration metadata needed for later interpretation
- keep `FunctionIndex` as the stable handle used by resolution and later interpretation
- keep one `ProgramEnvironment` function symbol table that includes both native functions and parsed user-defined functions

This is the simplest model that supports the requested two-phase resolution without introducing duplicate symbol tables or a second function lookup path.

# How should parsing change?

Parsing should recognize `fun` as a top-level declaration keyword and accept the smallest useful function syntax:

```ocelot
fun greet() {
  println("hello");
}
```

Recommended parsing steps:

1. Add `TokenType::Fun` and lex `fun` as a keyword.
2. Extend `parse_item()` so `fun` dispatches to a dedicated function-definition parser.
3. Parse:
   - `fun`
   - an identifier name
   - `(`
   - `)`
   - `{`
   - zero or more statements
   - `}`
4. Build a top-level function item with a statement-list body.
5. Add parser diagnostics for missing names, missing parentheses, and unterminated function bodies.

Restricting the first slice to empty parameter lists is the right tradeoff here.
It keeps the syntax honest without committing to a half-designed parameter model.

# How should resolution change?

Resolution should become an explicit two-phase pass over the parsed script.

Recommended behavior:

1. Phase one walks top-level items and registers every function declaration into the function table and symbol table.
2. Phase one should reject duplicate function names across:
   - user-defined functions
   - existing native functions
3. Phase two walks statements, test bodies, and function bodies to resolve call expressions against the completed function table.
4. Phase two should allow forward references, so a call can resolve a function declared later in the file.

That matches the user requirement directly and avoids order-dependent resolution bugs.

The current single-pass `resolve_item()` traversal in [`ocelot_resolver::resolve()`](/data/projects/ocelot/crates/resolver/src/lib.rs) is too early for this because it encounters call sites before later declarations have been registered.

# What diagnostics should this slice produce?

At minimum, this work should define stable parser and resolver diagnostics for:

- `fun` without a function name
- `fun name` without `()`
- `fun name()` without a body
- duplicate function declarations
- a user-defined function name that collides with a native function name
- unknown function calls after the function table has been populated from both native and user-defined definitions

Resolver diagnostics should still point at the callee identifier span, not the full call.
Duplicate-name diagnostics should point at the later declaration and, if practical, reference the original declaration location.

# How should spec documentation be added?

This work should add a dedicated spec chapter for function definitions with executable examples.

Recommended scope:

- add a new declarations chapter, likely `docs/spec/15.02 Declarations - Function definitions.md`
- describe the first-slice syntax as `fun name() { ... }`
- include examples showing:
  - a minimal function definition
  - a function declared before a call
  - a function declared after a call, proving forward resolution
  - an error example for duplicate function names or malformed declarations
- update [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md) so the new chapter is listed with the existing numbered chapters

Using the declarations chapter family is the pragmatic choice for now.
If the project later grows a larger chapter `20` dedicated to function semantics, this syntax chapter can stay where it is or be cross-linked without blocking this slice.

# How should the TextMate bundle change?

The TextMate bundle in [`Ocelot.tmLanguage`](/data/projects/ocelot/support/ocelot.tmbundle/Syntaxes/Ocelot.tmLanguage) should highlight `fun` as a declaration keyword alongside `test`.

This work does not need deeper grammar changes unless function definitions reveal missing scope names for identifiers or punctuation.
Keeping the bundle update small is enough for this slice.

# What implementation order keeps the work manageable?

1. Add this active plan for first-slice function definitions.
2. Introduce lexer support for `fun` and parser support for top-level function definition items with empty parameter lists.
3. Add AST nodes and module wiring for top-level function declarations and function bodies.
4. Refactor the function-table model so `FunctionDefinition` becomes an enum-backed table entry that can represent both native and user-defined functions cleanly.
5. Update resolver entry points to perform an explicit first phase that registers function declarations before resolving any call expressions.
6. Extend phase two resolution to walk function bodies in addition to top-level statements and test bodies.
7. Add parser and resolver tests for valid declarations, forward references, duplicate names, and malformed syntax.
8. Add spec chapters and examples, then update `docs/spec/README.md`.
9. Update the TextMate bundle so `fun` is highlighted correctly.
10. Run `nao check`.

# What verification should this work include?

Verification should include:

- colocated parser tests for:
  - one empty-parameter function definition
  - multiple top-level function definitions
  - mixed top-level items containing functions, statements, and tests
  - parser diagnostics for malformed `fun` declarations
- colocated resolver tests for:
  - resolving a call to a previously declared user-defined function
  - resolving a forward reference to a later function definition
  - resolving calls inside function bodies
  - duplicate function-name failures
  - collisions between native and user-defined function names
- spec validation coverage for the new function-definition examples
- running `nao check`

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes function definitions are allowed only at top level for now. Allowing them inside tests or other function bodies would complicate scope rules immediately.
- This plan assumes function bodies reuse existing statement syntax and do not need a special terminator or implicit return behavior yet.
- This plan assumes [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) will be generalized into an enum-backed table entry rather than split into separate native and user-defined table types. That is the cleaner model unless the implementation reveals a strong reason to separate declaration AST nodes from runtime function metadata more aggressively.
- This work ended up adding minimal execution for zero-argument user-defined functions so spec examples could remain executable end to end. That is a small, justified scope expansion, not a full function-runtime design.
- If the interpreter still builds `ProgramEnvironment` entirely in the engine before parsing, that setup will need to shift so parsed function declarations can be merged in deterministically during resolution.

# What landed from this plan?

This change landed the first slice of function definitions:

- the lexer and parser now recognize top-level `fun name() { ... }` items
- the AST now has a dedicated [`FunctionItem`](/data/projects/ocelot/crates/ast/src/function_item.rs) alongside statements and tests
- [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) now uses [`FunctionKind`](/data/projects/ocelot/crates/ast/src/function_kind.rs) so native and user-defined functions share one function table model
- the resolver now performs a declaration-registration phase before resolving call expressions, which enables forward references and duplicate-name diagnostics
- function bodies now participate in resolution
- the interpreter now executes zero-argument user-defined functions by resolved function index
- the engine, spec docs, spec chapter index, and TextMate bundle were updated for the new declaration form
- `cargo test -q -p ocelot-ast -p ocelot-parser -p ocelot-resolver -p ocelot-interpreter -p ocelot-engine -p ocelot-spec-validation` and `nao check` pass

# What concrete tasks should track this plan?

- [x] Add lexer support for the `fun` keyword.
- [x] Add AST support for top-level function definitions with empty parameter lists and statement-list bodies.
- [x] Update the parser to parse `fun name() { ... }` items and report stable diagnostics for malformed declarations.
- [x] Refactor `FunctionDefinition` into an enum-backed table entry so native and user-defined functions can share one `ProgramEnvironment`.
- [x] Split resolver work into a declaration-registration phase and a call-resolution phase.
- [x] Extend resolution to walk function bodies as well as top-level statements and test bodies.
- [x] Add parser and resolver tests for declarations, forward references, duplicate names, and native-name collisions.
- [x] Add spec chapters and examples for function definitions and update [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md).
- [x] Update the TextMate bundle so `fun` is highlighted as a declaration keyword.
- [x] Run `nao check`.
