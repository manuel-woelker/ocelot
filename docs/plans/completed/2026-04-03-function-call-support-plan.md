# Why is function call support needed now?

The spec already describes `println()` as a normal function call, but the current implementation does not model calls at all.
Instead, the parser recognizes only a special `println` statement, the AST has a dedicated [`PrintlnStatement`](/data/projects/ocelot/crates/ast/src/println_statement.rs), and the interpreter hardcodes that statement shape in [`Interpreter::interpret_statement()`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs).

That mismatch is small today, but it will get worse the moment the language adds any second callable thing.
Adding function call support now keeps the syntax tree aligned with the spec, removes a one-off parser branch, and creates a usable path for native functions without committing to full user-defined functions yet.

# What should the first slice of function call support include?

This slice should support:

- parsing call syntax like `name(...)`
- representing calls explicitly in the AST
- supporting expression statements, which covers the current `println("hello");` shape
- resolving a small set of native functions during interpretation
- validating native call arity and argument types with source diagnostics

This slice should not yet support:

- declaring user-defined functions
- first-class function values
- methods, member access, or chained calls
- keyword arguments, default arguments, or variadics

# What is the current implementation gap?

Today:

- [`ExpressionKind`](/data/projects/ocelot/crates/ast/src/expression_kind.rs) supports only identifiers and string literals
- [`StatementKind`](/data/projects/ocelot/crates/ast/src/statement_kind.rs) supports only `Println`
- [`Parser::parse_statement()`](/data/projects/ocelot/crates/parser/src/parser.rs) rejects any statement whose leading identifier is not literally `println`
- [`Interpreter::interpret_statement()`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) has one execution path that prints a newline after evaluating one argument
- spec examples and spec validation already treat `println()` as ordinary call syntax

So the language surface already wants a call model, while the implementation is still built around a one-off special form.

# What AST shape should replace the current `println` special case?

The AST should move to a generic call representation instead of adding more builtin-specific nodes.

Recommended shape:

- add a `CallExpression` node with:
  - a callee expression
  - an ordered argument list
- add `ExpressionKind::Call(CallExpression)`
- replace `StatementKind::Println` with `StatementKind::Expression`
- remove [`PrintlnStatement`](/data/projects/ocelot/crates/ast/src/println_statement.rs) once all parser and interpreter code has migrated

Keeping call data in expressions is the important part.
Expression statements should be real statements in the AST, even if early runtime behavior is still narrow.

# How should parsing change?

The parser should stop treating `println` as syntax and instead parse call syntax generically.

Recommended parsing steps:

1. Keep primary expression parsing for identifiers and string literals.
2. Add a postfix call parse step so an identifier followed by `(` becomes a call expression.
3. Parse zero or more comma-separated arguments, while allowing the first slice to accept only the forms needed by native functions.
4. Parse a statement by parsing an expression and then requiring `;`.

This preserves the existing top-level script shape while making `println("hello");` work because it is an ordinary call expression wrapped in an expression statement, not because `println` is magical syntax.

# How should native functions be executed?

The interpreter should resolve supported native functions by callee name rather than by statement kind.

Recommended first model:

- when evaluating a call expression, require the callee to be a plain identifier
- dispatch that identifier through a small native-function table or match
- keep `println` as the first native function
- execute the native implementation only after validating arity and argument types

The plan does not need a full runtime value system yet, but call support will be cleaner if the interpreter stops returning raw `String` values from expression evaluation.
It should introduce a small runtime value enum now, even if the first version only has `String` and a unit-like value for native calls.

# What diagnostics should this first slice produce?

Function-call support should keep diagnostics in the shared compilation context for user-facing mistakes that are known during parsing or native call validation.

At minimum, this slice should cover:

- missing `)` after a call argument list
- malformed argument lists such as `println("a",)`
- unknown native function names such as `printline("x");`
- wrong native arity such as `println();`
- wrong native argument kind such as passing an unresolved identifier where text is required

The exact phase boundary can stay pragmatic:

- syntax problems belong in the parser
- native signature mismatches can be handled in parsing for known builtins when preserving source diagnostics is worth the small amount of special knowledge

The main thing to avoid is regressing into opaque string errors once calls stop being parser-special-cased.

# What implementation order keeps the change manageable?

1. Add AST nodes and modules for call expressions and the new statement shape.
2. Update parser expression parsing so identifiers can be followed by `(` argument lists to form call expressions.
3. Replace the `println`-specific statement parsing path with generic expression-statement parsing.
4. Update parser tests to assert on the new AST shape and on diagnostics for invalid call syntax.
5. Introduce a minimal runtime value type for interpreter expression evaluation.
6. Add native call dispatch with `println` as the first native function.
7. Update interpreter and engine tests to cover successful native calls, unknown functions, and native call failures in both script and test execution.
8. Update spec chapters if the wording still implies special-cased behavior or if a dedicated expressions/functions chapter is now justified.
9. Run `nao check`.

# What verification should this work include?

Verification should include:

- colocated parser tests for:
  - a single call statement
  - multiple call statements
  - call expressions with multiple arguments, even if only `println` is validated initially
  - non-call expression statements such as `"hello";` or `name;` parsing into the expected AST shape
  - malformed call syntax diagnostics
- colocated interpreter tests for:
  - successful `println("hello");`
  - unknown native function errors
  - wrong native arity or argument-kind failures
- engine-level tests covering script execution and test execution through the new call path
- spec-validation coverage updates if spec examples or stable diagnostics change
- running `nao check`

# What landed from this plan?

This change landed the first generic call-expression path in the language implementation:

- the AST now represents calls with `CallExpression` and statements with `ExpressionStatement`
- the parser now parses generic expression statements and postfix call syntax with comma-separated arguments
- `println` now executes through native function dispatch instead of a dedicated statement node
- the interpreter now uses a small `RuntimeValue` enum so call evaluation can return either strings or a unit-like result
- parser, interpreter, engine, and spec coverage were updated, including a new [`01.01 Expressions - Function calls`](/data/projects/ocelot/docs/spec/01.01%20Expressions%20-%20Function%20calls.md) chapter
- `nao check` passes

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes the first callable callee form is a bare identifier only. Supporting calls on arbitrary expressions can come later.
- There is a small design choice between reporting native signature mismatches in the parser versus the interpreter. The cleaner long-term answer is probably a semantic-analysis phase, but that would be overengineered for this slice.
- Introducing call expressions without a minimal runtime value type would paint the interpreter into a corner and make later native functions awkward.
- If `println` remains a dedicated AST node alongside generic calls, the codebase will end up with duplicate execution paths and the refactor will not have paid for itself.
- General expression statements imply an execution-time question for values with no observable effect. The first slice can treat them as evaluated-and-discarded, but that behavior should stay explicit.
- The spec likely wants a new chapter under `01` or `05` for call expressions once the implementation exists, even if this first implementation only uses native functions.

# What concrete tasks should track this plan?

- [x] Add an active plan for first-slice function call support oriented around native functions.
- [x] Add AST support for call expressions and expression statements, replacing the `PrintlnStatement` special case.
- [x] Update the parser to parse generic call syntax and generic expression statements.
- [x] Update parser diagnostics and parser tests for valid and invalid call syntax.
- [x] Introduce a minimal runtime value model needed to evaluate calls cleanly.
- [x] Implement native function dispatch for `println` and any supporting diagnostics for unknown or invalid native calls.
- [x] Update interpreter and engine tests to cover the new call execution path in scripts and tests.
- [x] Update spec chapters and spec validation coverage if the new implementation changes wording or stable diagnostics.
- [x] Run `nao check`.
