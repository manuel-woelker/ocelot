# What semantics will blocks establish?

`{ statements; expression }` is an expression whose value and type come from its
last, semicolon-free expression. A semicolon discards an expression's value, and
an empty block produces unit. Function bodies follow the same rule. Declared
result types remain explicit; an omitted annotation still means unit.

Explicit `return` exits the enclosing function from any nested block, including
inside call arguments, negation, and template interpolation. A block that always
returns has an internal non-fallthrough type that is compatible with expected
types; it does not produce a runtime value at that point. Function calls consume
their own returns rather than returning from their callers.

# What is the scope?

Establish block expressions before conditionals. Do not add if syntax, inference
of function result annotations, local bindings, or loops in this change. Keep
statement-position semicolons required, including for discarded block expressions.
Tests remain unit contexts and cannot return from a function. Continue checking
unreachable code and conservatively tracking effects. Preserve comments in the
currently supported item/statement positions, including around tail expressions.

# What work and verification are required?

- [x] Represent block expressions and distinguish tail expressions from discarded statements.
- [x] Parse blocks in expression positions and tail expressions in bodies, preserving
  source spans, comments, and nested template interpolation delimiters.
- [x] Reuse block result typing for functions, tests, and standalone expressions;
  validate implicit results and preserve explicit return checking.
- [x] Propagate normal function returns through nested expression evaluation while
  preserving evaluation order and skipping subsequent effects after a return.
- [x] Format nested blocks and preserve semicolon choices and supported comments.
- [x] Add source-driven tests for values, unit, annotations, discarded values,
  nested returns, unreachable checks, type errors, templates, evaluation order,
  imported calls, embedded eval, malformed input, and formatter round trips.
- [x] Update executable specs and overview; supersede DR-014 with a bidirectionally
  linked decision that establishes block results as the basis for future if expressions.
- [x] Run `nao check`, inspect diagnostics, record verification, and move the plan
  to `completed` before committing the feature.

# What risks need attention?

This intentionally makes a missing final semicolon meaningful: existing unit
functions with final expressions must use semicolons to discard them. Return
control must travel through expressions without masquerading as a runtime error.
Template lexing must distinguish a block's closing brace from an interpolation's
closing brace. Future conditional branches will need result unification and
path analysis, but should consume these same block values and return signals.

# What was implemented and verified?

Block expressions and function/test bodies reuse one statement-sequence result
rule. The AST preserves whether an expression has a semicolon; tail trivia stays
attached to its statement. An internal non-fallthrough type models nested explicit
returns without treating them as unit values. Expression evaluation propagates
normal return control through calls, arguments, negation, and templates, consuming
it only at function boundaries.

The interpolation lexer now retains brace depth per active interpolation. Its
reusable finding is recorded in development learnings. DR-015 supersedes DR-014's
explicit-return-only restriction while retaining declared result types.

Colocated regressions cover values and unit, semicolons, imported calls, annotation
errors, nested returns through each expression form, evaluation order, skipped
effects, unreachable name/type/effect checks, native and user call arguments,
template nesting, malformed blocks, comments, and formatter idempotence.

`nao check` passed, including Rust tests and all 70 executable examples across
19 spec chapters. `git diff --check` and local documentation link checks passed.
An isolated REPL smoke test evaluated a block with an effect and called a retained
function with a tail result. Conditional syntax remains the subsequent feature;
all planned block-foundation work is complete.
