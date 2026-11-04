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

- [ ] Represent block expressions and distinguish tail expressions from discarded statements.
- [ ] Parse blocks in expression positions and tail expressions in bodies, preserving
  source spans, comments, and nested template interpolation delimiters.
- [ ] Reuse block result typing for functions, tests, and standalone expressions;
  validate implicit results and preserve explicit return checking.
- [ ] Propagate normal function returns through nested expression evaluation while
  preserving evaluation order and skipping subsequent effects after a return.
- [ ] Format nested blocks and preserve semicolon choices and supported comments.
- [ ] Add source-driven tests for values, unit, annotations, discarded values,
  nested returns, unreachable checks, type errors, templates, evaluation order,
  imported calls, embedded eval, malformed input, and formatter round trips.
- [ ] Update executable specs and overview; supersede DR-014 with a bidirectionally
  linked decision that establishes block results as the basis for future if expressions.
- [ ] Run `nao check`, inspect diagnostics, record verification, and move the plan
  to `completed` before committing the feature.

# What risks need attention?

This intentionally makes a missing final semicolon meaningful: existing unit
functions with final expressions must use semicolons to discard them. Return
control must travel through expressions without masquerading as a runtime error.
Template lexing must distinguish a block's closing brace from an interpolation's
closing brace. Future conditional branches will need result unification and
path analysis, but should consume these same block values and return signals.
