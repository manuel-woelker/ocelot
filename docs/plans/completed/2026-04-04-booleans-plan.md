# Why add booleans now?

The language currently has string literals, call expressions, and test items, but it has no boolean values at all.
That leaves a basic gap in the primitive type story and makes later features such as conditions, logical operators, and richer assertions harder to add cleanly.

This slice should stay narrow:

- booleans are a primitive type
- `true` and `false` are boolean literals
- booleans may be evaluated, passed to functions, and compared by existing runtime equality logic
- no unary or binary boolean operators are introduced yet

Keeping the first slice literal-only is the right tradeoff.
It establishes the type and runtime representation without dragging in control flow or operator precedence work early.

# What is the current gap in the implementation?

The current implementation only treats string literals as first-class literal expressions.

Today:

- the lexer reserves `test` but not `true` or `false`
- the parser only builds identifier, string-literal, and call expressions
- the AST has no boolean literal node
- the interpreter runtime value enum only has `String` and `Unit`
- `println()` only accepts strings
- the spec has no primitive-types chapter content yet

That means `true` and `false` currently parse as ordinary identifiers and fail later as unresolved names.

# What AST shape should booleans use?

Booleans should be explicit literal expressions in the AST, not magic identifiers.

The recommended shape is:

- add a dedicated `BooleanLiteralExpression` node in its own file
- extend `ExpressionKind` with `BooleanLiteral(BooleanLiteralExpression)`
- preserve the existing style where literals carry their parsed value directly

That keeps the AST honest and prevents later semantic stages from having to special-case identifier names that are really literals.

Treating `true` and `false` as identifiers would be a bad shortcut.
It would make later typing, control-flow parsing, and error messages more brittle for no real benefit.

# What lexer and parser changes are required?

The lexer in [`crates/parser/src/lexer/lex.rs`](/data/projects/ocelot/crates/parser/src/lexer/lex.rs) should reserve `true` and `false` the same way it already reserves `test`.

The parser in [`crates/parser/src/parser.rs`](/data/projects/ocelot/crates/parser/src/parser.rs) should then:

- parse `true` into a boolean-literal expression with value `true`
- parse `false` into a boolean-literal expression with value `false`
- keep all existing expression rules unchanged otherwise

No new precedence or expression forms should be added in this slice.
`true` and `false` should simply become additional primary expressions.

# What runtime representation should booleans use?

Booleans should become a first-class `RuntimeValue` variant in [`crates/interpreter/src/runtime_value.rs`](/data/projects/ocelot/crates/interpreter/src/runtime_value.rs).

Recommended runtime work:

- add `RuntimeValue::Boolean(bool)`
- add constructor and accessor helpers consistent with the current `string()` and `unit()` style
- update equality so boolean values compare by value
- update assertion rendering so booleans render as stable user-facing literals such as `true` and `false`

This keeps runtime behavior unsurprising and aligns assertion output with source syntax.

# How should the interpreter treat boolean expressions?

The interpreter in [`crates/interpreter/src/interpreter.rs`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) should evaluate boolean literals directly to boolean runtime values.

No operator semantics are needed yet.
The only observable behaviors in this slice are:

- boolean literals can be evaluated without runtime errors
- `assert_eq(true, true)` can succeed once booleans are runtime values
- `assert_eq(true, false)` can fail with sensible rendered values

`println()` is part of the observable surface for this slice.

# How should `println()` interact with booleans?

`println()` should accept booleans in this slice.

Reasons:

- it gives the new primitive type an immediate observable use
- it keeps spec examples small and concrete
- it avoids the awkward result where a primitive exists but cannot be displayed except indirectly through assertion failures

The intended behavior is:

- `println()` accepts either a string or a boolean
- boolean output uses source-style rendering, `true` and `false`
- the existing arity rule stays unchanged

This is still a small change and does not imply a general-purpose `Display` protocol.

# What spec chapters describe booleans?

Booleans need a proper type chapter plus at least one example in a user-facing behavior chapter.

The implemented spec work is:

- add `10.01 Types - Booleans`
- update [`docs/spec/30.01 Standard library - println.md`](/data/projects/ocelot/docs/spec/30.01%20Standard%20library%20-%20println.md) so it explicitly accepts booleans
- add or update examples so boolean literals appear in ordinary executable code

The type chapter should specify:

- `bool` is a primitive type
- `true` and `false` are boolean literals
- booleans are distinct from strings
- no boolean operators are specified yet in this slice

The standard-library chapter should say this explicitly instead of quietly relying on implementation behavior.

# What examples should the repository add?

The repository now adds a small example file, `examples/booleans.ocelot`.

That example should demonstrate something real but still stay within scope, for example:

- printing `true` and `false`
- using `assert_eq(true, true)` in a test item

The example should not fake operators or conditionals that the language does not support yet.

# What tests verified the feature?

Verification should stay colocated and black-box where possible.

Lexer tests in [`crates/parser/src/lexer/lex.rs`](/data/projects/ocelot/crates/parser/src/lexer/lex.rs) should cover:

- `true` lexes as a reserved boolean token
- `false` lexes as a reserved boolean token
- longer identifiers such as `true_value` and `falsey` remain identifiers

Parser tests in [`crates/parser/src/parse_script.rs`](/data/projects/ocelot/crates/parser/src/parse_script.rs) should cover:

- `true;` parses as a boolean literal expression
- `false;` parses as a boolean literal expression
- boolean literals can appear as call arguments

Runtime-value tests in [`crates/interpreter/src/runtime_value.rs`](/data/projects/ocelot/crates/interpreter/src/runtime_value.rs) should cover:

- constructor and accessor behavior
- equality behavior
- assertion rendering

Interpreter or engine tests should cover:

- `println(true);` and `println(false);`
- `assert_eq(true, true);` succeeds
- `assert_eq(true, false);` reports boolean values in the assertion output

Spec validation now covers:

- successful boolean examples
- updated `println()` examples showing boolean output

# What implementation order was used?

1. Add a boolean literal AST node and extend `ExpressionKind`.
2. Reserve `true` and `false` in the lexer.
3. Parse boolean literals as primary expressions.
4. Extend `RuntimeValue` with boolean support and assertion rendering.
5. Teach the interpreter to evaluate boolean literals.
6. Update `println()` so it accepts booleans and renders them as `true` or `false`.
7. Add colocated tests across lexer, parser, runtime value, interpreter, and engine as needed.
8. Add spec chapters and example files for booleans.
9. Run `nao check`.

This order keeps the semantic model honest.
Trying to document booleans before the runtime can actually carry them would just create churn.

# What assumptions and follow-up notes should stay explicit?

- This plan assumes booleans are source literals only in the first slice and do not yet imply `if`, `&&`, `||`, or `!`.
- The exact public source spelling of the primitive type should likely be `bool`, but the current implementation may not yet have any user-visible type annotation syntax.
  That is fine as long as the spec explains booleans as a primitive type in prose rather than pretending type annotations already exist.

# What verification was completed?

Verification completed with:

- `cargo test -p ocelot-parser`
- `cargo test -p ocelot-interpreter`
- `cargo test -p ocelot-engine`
- `cargo test -p ocelot-spec-validation`
- `nao check`

# What concrete tasks should track this plan?

- [x] Add a dedicated boolean literal AST node and extend `ExpressionKind` to use it.
- [x] Reserve `true` and `false` in the lexer without breaking longer identifiers.
- [x] Parse boolean literals as primary expressions.
- [x] Extend `RuntimeValue` with boolean support, equality behavior, and assertion rendering.
- [x] Teach the interpreter to evaluate boolean literals.
- [x] Update `println()` so it accepts booleans and renders them consistently.
- [x] Add colocated lexer, parser, runtime-value, interpreter, and engine coverage for boolean literals.
- [x] Add a `10.01 Types - Booleans` spec chapter and update related spec chapters as needed.
- [x] Add or update example `.ocelot` files that demonstrate boolean literals without introducing operators.
- [x] Run `nao check`.
