# Why add a `not` prefix operator now?

Booleans exist as a primitive type, but the language still cannot express the most basic boolean transformation.
That makes booleans useful mainly as literal values passed through builtins rather than as values that can participate in even simple logic.

This slice should stay narrow:

- add a single prefix operator, spelled `not`
- limit it to boolean operands
- do not add binary boolean operators yet
- do not add control-flow syntax in the same change

That keeps the feature set honest.
`not` is the smallest useful operator for booleans, and it gives later conditionals and richer assertions a cleaner foundation.

# What is the current implementation gap?

Today:

- the lexer reserves `true`, `false`, and `test`, but not `not`
- the parser only supports primary expressions followed by call expressions
- the AST has no unary-expression node
- the interpreter can evaluate boolean literals, but it has no boolean operator semantics
- the boolean spec chapter explicitly says boolean operators are not specified yet
- the TextMate bundle does not know that `not` is a keyword

That means `not true` currently parses as an identifier named `not` followed by a second expression-shaped token stream that fails.

# What AST shape should prefix negation use?

`not` should use an explicit AST node rather than piggybacking on identifiers or a stringly-typed operator field.

The recommended shape is:

- add a dedicated `NotExpression` node in its own file
- extend `ExpressionKind` with `Not(NotExpression)`
- store the operand expression directly on that node

This follows the repository rule that each AST type lives in its own file and keeps the expression tree easy to read.
Adding a generic unary-operator enum right now would be premature abstraction because this language only has one unary operator.

# What precedence and parsing rules should `not` use?

`not` should be a prefix operator that binds looser than function calls and tighter than statement termination.

The intended parsing behavior is:

- `not true` parses as negation of the boolean literal `true`
- `not not false` parses as nested negation
- `not foo()` parses as negation of the call result, not as a call on `not foo`
- `assert(not false);` parses normally as a call argument

The parser in [`crates/parser/src/parser.rs`](/data/projects/ocelot/crates/parser/src/parser.rs) should therefore grow an explicit prefix-expression layer instead of trying to bolt `not` onto primary-expression parsing.

Recommended structure:

1. `parse_expression()` delegates to `parse_prefix_expression()`
2. `parse_prefix_expression()` consumes leading `not` and recursively parses another prefix expression
3. the non-`not` path delegates to the existing call-expression layer
4. the call-expression layer continues to build calls on top of primary expressions

This keeps precedence predictable and leaves room for later unary operators without another parser rewrite.

# What lexer changes are required?

The lexer in [`crates/parser/src/lexer/lex.rs`](/data/projects/ocelot/crates/parser/src/lexer/lex.rs) should reserve `not` as a keyword token.

Tests should prove:

- `not` lexes as its own token
- longer identifiers such as `notify`, `not_value`, and `knot` remain identifiers

Reserving `not` now is the right call.
Treating it as a contextual keyword would only complicate the parser for a language this small.

# How should the interpreter evaluate `not`?

The interpreter in [`crates/interpreter/src/interpreter.rs`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) should evaluate the operand, require a boolean runtime value, and return the inverted boolean.

The intended semantics are:

- `not true` evaluates to `false`
- `not false` evaluates to `true`
- `not` applied to a non-boolean value is a type error

The runtime error should be direct and stable, for example `type error: \`not\` expects a boolean operand`.
There is no reason to invent coercions here.
Allowing strings or truthiness would make the language sloppier before it has even established its boolean model.

# What spec chapters should change?

This feature needs both a new expression chapter and an update to the boolean type chapter.

The recommended spec work is:

- add `02.02 Expressions - Prefix negation`
- update [`docs/spec/10.01 Types - Booleans.md`](/data/projects/ocelot/docs/spec/10.01%20Types%20-%20Booleans.md) so it no longer claims boolean operators are unspecified
- update [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md) to list the new expression chapter

`02.02 Expressions - Prefix negation` should specify:

- the source spelling is `not`
- the operand must be boolean
- the result type is boolean
- nested negation is allowed
- function calls bind tighter than prefix negation

`10.01 Types - Booleans` should stop pretending booleans are literal-only now that one operator exists.

# What examples should the repository add or update?

The repository should add at least one example file that demonstrates `not` in ordinary executable code.

Recommended example work:

- add `examples/not.ocelot` with `println(not false);`
- include a small `test` item such as `assert(not false);`
- update any existing boolean example if a simpler `not` use makes it clearer

The examples should stay within scope.
Do not sneak in `and`, `or`, `if`, or comparison operators just because `not` now exists.

# What support files should be updated beyond the core implementation?

The TextMate bundle in [`support/ocelot.tmbundle/Syntaxes/Ocelot.tmLanguage`](/data/projects/ocelot/support/ocelot.tmbundle/Syntaxes/Ocelot.tmLanguage) should highlight `not` as a keyword.

This is small, but it matters.
Shipping a new keyword while editor support colors it like an identifier is avoidable paper-cut territory.

# What tests should verify the feature?

Verification should stay colocated and black-box where possible.

Lexer tests in [`crates/parser/src/lexer/lex.rs`](/data/projects/ocelot/crates/parser/src/lexer/lex.rs) should cover:

- `not` lexes as a reserved token
- longer names that contain `not` remain identifiers

Parser tests in [`crates/parser/src/parse_script.rs`](/data/projects/ocelot/crates/parser/src/parse_script.rs) should cover:

- `not true;`
- `not not false;`
- `not foo();`
- `assert(not false);`

Interpreter and engine tests should cover:

- `println(not false);` prints `true`
- `println(not true);` prints `false`
- `assert(not false);` succeeds
- `assert(not true);` fails with the existing assert failure path
- `not "hello"` reports a boolean-only type error

Spec validation should cover:

- the new `02.02` chapter examples
- the updated boolean chapter examples

# What implementation order makes the change clean?

1. Add a dedicated `NotExpression` AST node and extend `ExpressionKind`.
2. Reserve `not` in the lexer without breaking longer identifiers.
3. Refactor parser expression entry points to add a prefix-negation layer with clear precedence relative to calls.
4. Teach the interpreter to evaluate `not` and reject non-boolean operands.
5. Add colocated lexer, parser, interpreter, and engine coverage.
6. Add the new spec chapter, update the boolean chapter, and update the spec index.
7. Update the TextMate bundle so `not` highlights as a keyword.
8. Add or update example `.ocelot` files.
9. Run `nao check`.

This order keeps the tree shape, runtime behavior, docs, and support files moving in sync.

# What assumptions and risks should stay explicit?

- This plan assumes `not` is a reserved keyword rather than a contextual keyword.
- This plan intentionally avoids introducing a generic unary-operator framework until the language actually has a second unary operator.
- This plan assumes call expressions should bind tighter than `not`, which is the least surprising behavior for users and the easiest rule to explain in the spec.
- This plan does not yet imply `and`, `or`, `if`, or boolean comparisons.

# What verification was completed?

Verification completed with:

- `cargo test -p ocelot-parser`
- `cargo test -p ocelot-interpreter`
- `cargo test -p ocelot-engine`
- `cargo test -p ocelot-spec-validation`
- `nao check`

# What concrete tasks should track this plan?

- [x] Add a dedicated `NotExpression` AST node and extend `ExpressionKind`.
- [x] Reserve `not` in the lexer without breaking longer identifiers.
- [x] Refactor parser expression parsing to support prefix negation with calls binding tighter than `not`.
- [x] Teach the interpreter to evaluate `not` for booleans and report a stable type error for non-boolean operands.
- [x] Add colocated lexer, parser, interpreter, and engine coverage for `not`.
- [x] Add `02.02 Expressions - Prefix negation` and update the boolean/spec index chapters accordingly.
- [x] Update the TextMate bundle so `not` highlights as a keyword.
- [x] Add or update example `.ocelot` files that demonstrate `not` without introducing more operators.
- [x] Run `nao check`.
