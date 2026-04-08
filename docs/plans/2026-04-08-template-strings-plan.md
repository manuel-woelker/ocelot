# Why add template strings now?

The language already has string literals, function calls, local bindings, and runtime value rendering, but building user-facing text still requires awkward manual composition outside the language.
Template strings close that gap with a small surface-area feature that is immediately useful for `println()` and future diagnostics, while still fitting the current expression-oriented implementation.

The requested behavior is clear:

- quoted strings may contain `${...}` interpolation markers
- the content inside `${` and `}` is an ordinary expression
- those expressions are evaluated at runtime
- the final string is produced by concatenating literal text and rendered expression results in source order

This is a good feature to add now because it exercises the lexer, parser, formatter, resolver, and interpreter together without forcing larger control-flow or collection-language work.

# What should the first slice of template-string support include?

This slice should support:

- plain template text like `"Hello"`
- mixed templates like `"Hello ${name}!"`
- multiple interpolations in one string
- arbitrary currently-supported expressions inside `${...}`, including:
  - identifiers
  - qualified identifiers
  - function calls
  - boolean literals
  - string literals
  - nested prefix `not`
- runtime evaluation of each interpolation in left-to-right order
- formatting the interpolated runtime values with the same display rendering already used by [`RuntimeValue::render_for_display()`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs)

This slice should not try to add:

- escape-sequence support
- raw strings
- string concatenation operators
- custom formatting directives
- multi-line string semantics

That keeps the feature small and avoids mixing interpolation with unrelated string-design questions.

# What is the current implementation gap?

Today the implementation treats every quoted string as one indivisible lexer token and one AST payload value:

- [`TokenType::String`](/data/projects/ocelot/crates/parser/src/lexer/token_type.rs) represents the entire quoted region
- [`Parser::parse_primary_expression()`](/data/projects/ocelot/crates/parser/src/parser.rs) strips the opening and closing quote and builds [`ExpressionKind::StringLiteral`](/data/projects/ocelot/crates/ast/src/expression_kind.rs)
- [`StringLiteralExpression`](/data/projects/ocelot/crates/ast/src/string_literal_expression.rs) stores only the final cooked text, with no place for embedded expressions
- the resolver assigns all string literals the builtin string type directly
- the interpreter evaluates a string literal by returning [`RuntimeValue::string()`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs)
- the formatter simply writes the stored text back between quotes

That model is too flat for interpolation.
Once `${...}` is legal, the compiler must preserve the boundary between literal text and embedded expressions all the way through parsing and runtime evaluation.

# What AST shape should represent template strings?

Template strings should become an explicit expression node instead of being smuggled through the existing plain-string payload.

Recommended shape:

- add a dedicated `TemplateStringExpression` node in its own file
- represent a template string as an ordered list of parts
- add a dedicated `TemplateStringPart` enum in its own file with:
  - `Text`
  - `Interpolation`
- keep plain non-interpolated strings representable without losing source intent

There are two viable AST strategies:

1. Replace `StringLiteralExpression` with the new template-string node for all quoted strings.
2. Keep `StringLiteralExpression` for interpolation-free strings and add `ExpressionKind::TemplateString(...)` for mixed strings.

The second option is the better fit for this repository right now.
It keeps simple strings cheap and readable in tests, minimizes churn in unrelated code, and makes it obvious where interpolation-specific logic begins.

Under that model:

- `"hello"` stays `ExpressionKind::StringLiteral`
- `"Hello ${name}!"` becomes `ExpressionKind::TemplateString`
- resolver and interpreter treat both variants as producing the string type

# How should lexing and parsing work?

The current lexer cannot keep template strings as a single `String` token.
Doing that would force the parser to re-lex string bodies manually, which is exactly the kind of cleverness this codebase should avoid.

Recommended lexer approach:

- introduce dedicated template-string tokens for structural boundaries and text segments
- let the lexer switch between normal mode and string mode when it sees `"`
- emit interpolation boundary tokens for `${` and `}`
- emit literal text chunks between interpolation boundaries as their own token kind
- keep existing trivia behavior outside of strings

One practical token model would be:

- `StringStart`
- `StringText`
- `InterpolationStart`
- `StringEnd`

The parser can then:

1. Detect whether a quoted string is a plain string or a template string.
2. Parse string text chunks directly into text parts.
3. When it sees `${`, call ordinary expression parsing for the interpolation body until the matching `}`.
4. Build `StringLiteralExpression` for interpolation-free strings and `TemplateStringExpression` for mixed strings.

Two design constraints matter here:

- interpolation bodies should reuse normal expression parsing rather than inventing a separate mini-language
- the lexer/parser boundary should preserve enough structure that future expression forms inside `${...}` work automatically

Test-item names need one explicit decision.
For the first slice, test names should remain plain string literals only, even if ordinary expression strings support interpolation.
Runtime-dependent test names would make test discovery, filtering, and reporting needlessly unstable.

# How should diagnostics behave?

Template strings add a few new failure modes that should be diagnosed at the syntax layer.

At minimum, this slice should cover:

- unterminated template strings
- `${` without a closing `}`
- missing expression content such as `"hello ${}"`
- malformed expressions inside interpolation, reported through the existing parser diagnostics

The existing unterminated-string spec chapter should stay valid for missing closing quotes.
If interpolation-specific diagnostics become user-visible, they should be specified explicitly rather than left as accidental parser wording.

# How should resolution and runtime evaluation work?

The resolver should treat template strings as string-typed expressions.

Recommended resolver behavior:

- resolve every interpolation expression recursively
- assign the builtin string type to the outer template-string expression
- let any inner type errors surface normally from the embedded expressions

The interpreter should evaluate template strings left to right:

1. start with an empty output buffer
2. append each text part as-is
3. evaluate each interpolation expression
4. render the resulting runtime value with `render_for_display()`
5. append that rendered text to the output buffer
6. return `RuntimeValue::string(...)`

Using display rendering is the right default.
It matches what users already see through `println()`, avoids assertion-style quotes leaking into normal strings, and leaves richer formatting protocols for later language work.

# What else needs to change besides the parser and interpreter?

Template strings affect more than the syntax pipeline.

The work should also update:

- [`crates/formatter/src/format_compilation_unit.rs`](/data/projects/ocelot/crates/formatter/src/format_compilation_unit.rs) so `ocelot fmt` preserves and normalizes template-string syntax
- resolver tests and helpers that currently assume only plain string literals exist
- interpreter tests that assert on string runtime results
- spec validation coverage so executable examples cover the new syntax end to end

Spec updates should likely include:

- a new expressions chapter for template strings, for example `02.03 Expressions - Template strings.md`
- updates to [`30.01 Standard library - println.md`](/data/projects/ocelot/docs/spec/30.01%20Standard%20library%20-%20println.md) so examples can demonstrate interpolation naturally
- updates to [`91.01 Lexer errors - Unterminated strings.md`](/data/projects/ocelot/docs/spec/91.01%20Lexer%20errors%20-%20Unterminated%20strings.md) or a nearby syntax-errors chapter if interpolation introduces stable new diagnostics

# What implementation order keeps this change manageable?

1. Add active planning documentation for template strings.
2. Add AST nodes for template-string expressions and parts.
3. Extend lexer token kinds and lexing logic to preserve template-string structure.
4. Update parser string handling so interpolation-free strings still parse simply while interpolated strings build the new AST node.
5. Keep test-item parsing restricted to plain string names.
6. Teach the resolver to resolve interpolation expressions and assign template strings the string type.
7. Teach the interpreter to evaluate template strings into runtime strings using display rendering.
8. Update the formatter to print template strings and interpolated expressions correctly.
9. Add or update spec chapters and examples.
10. Run targeted crate tests and `nao check`.

# What verification should this work include?

Verification should stay colocated and cover the full pipeline.

Lexer tests in [`crates/parser/src/lexer/lex.rs`](/data/projects/ocelot/crates/parser/src/lexer/lex.rs) should cover:

- interpolation-free quoted strings still lex correctly
- `"Hello ${name}!"` produces the expected template-string token sequence
- multiple interpolation segments in one string
- unterminated interpolation and unterminated string diagnostics

Parser tests in [`crates/parser/src/parse_compilation_unit.rs`](/data/projects/ocelot/crates/parser/src/parse_compilation_unit.rs) should cover:

- plain strings still parse as `StringLiteral`
- interpolated strings parse as `TemplateStringExpression`
- arbitrary embedded expressions such as `"${not false}"` and `"${helper(name)}"`
- invalid interpolation syntax produces stable diagnostics
- test names reject interpolated strings

Resolver tests in [`crates/resolver/src/tests.rs`](/data/projects/ocelot/crates/resolver/src/tests.rs) should cover:

- template strings resolve to the string type
- interpolation expressions still get normal function and identifier resolution
- bad interpolation expressions surface the expected resolver errors

Interpreter or engine tests should cover:

- `println("Hello ${name}!");` with a bound local value
- multiple interpolations in one string
- interpolation of boolean values using display-style rendering
- interpolation of user-defined call results if the current runtime model allows it

Formatter tests in [`crates/formatter/src/format_compilation_unit.rs`](/data/projects/ocelot/crates/formatter/src/format_compilation_unit.rs) should cover:

- preserving interpolation syntax
- normalizing whitespace inside `${...}` according to formatter rules

Repository-level verification should include:

- `cargo test -p ocelot-parser`
- `cargo test -p ocelot-resolver`
- `cargo test -p ocelot-interpreter`
- `cargo test -p ocelot-engine`
- `cargo test -p ocelot-formatter`
- `cargo test -p ocelot-spec-validation`
- `nao check`

# What assumptions and open questions should stay explicit?

- This plan assumes interpolated values render with [`RuntimeValue::render_for_display()`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs), not `render_for_assertion()`.
- This plan assumes interpolation is allowed only in expression-position strings, not test-item names.
- The current language has no escape sequences. If escape syntax is added later, template-string lexing will need a deliberate update rather than ad hoc exceptions.
- If future expressions introduce braces, the template-string lexer/parser boundary may need explicit nesting rules so `}` closes the right construct.
- Keeping plain string literals as their own AST variant is slightly more code than collapsing everything into one node, but it buys simpler tests and less churn across the repo.

# What concrete tasks should track this plan?

- [ ] Add AST support for template-string expressions and ordered template-string parts.
- [ ] Extend lexer token kinds and lexing logic to preserve `${...}` structure inside quoted strings.
- [ ] Update parser string parsing to build either `StringLiteralExpression` or `TemplateStringExpression` as appropriate.
- [ ] Keep test-item names restricted to non-interpolated string literals and add coverage for that rule.
- [ ] Update resolver logic and tests so template strings resolve as string-typed expressions.
- [ ] Update interpreter evaluation so template strings evaluate embedded expressions at runtime and concatenate display-rendered results.
- [ ] Update formatter support and tests for template-string syntax.
- [ ] Add or update spec chapters, examples, and spec-validation coverage for template strings and any new stable diagnostics.
- [ ] Run targeted crate tests and `nao check`.
