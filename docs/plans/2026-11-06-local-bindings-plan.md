# What syntax and semantics will be added?

Add immutable `let name = expression;` statements, optionally annotated as
`let name: string = expression;`, `bool`, or `()`. Initializers are required,
evaluate exactly once, and determine inferred types. Explicit annotations are
checked against the initializer. Bindings are visible only after initialization.
`let` becomes reserved and `=` is recognized solely for binding initialization.

# How will lexical scope behave?

- Blocks create nested scopes. Inner bindings may shadow outer bindings, and
  an inner initializer may still reference the outer binding of the same name.
- Duplicate names in one scope are rejected. Function parameters share the
  function body's scope, so redeclaring a parameter there is an error.
- Scope exit restores outer names after normal completion, early return, or error.
- Functions and tests start isolated scopes and do not capture script bindings.
- Script top-level bindings live for that execution. Each test has a fresh scope.
- Embedded execute retains successful top-level values along with source; eval can
  read them. Failed execute commits neither source nor new bindings. Earlier
  initializers must not replay. Block-local values created by eval do not persist.
- No reassignment, mutability, destructuring, or uninitialized declarations are added.

# What will be implemented and verified?

- [ ] Add binding AST, lexer tokens, parser syntax, annotations, and formatting.
- [ ] Replace parameter-only resolution with lexical scopes, inference, duplicate
  detection, declaration ordering, and annotated-type validation.
- [ ] Execute initializers once, manage runtime scopes, and propagate returns from
  initializers without inserting an uninitialized value.
- [ ] Preserve script bindings, isolate function/test calls, and transactionally
  retain embedded top-level values without replaying prior effects.
- [ ] Add source-driven regressions for scope restoration, shadowing, duplicates,
  use-before-binding, annotations, branch-local names, early returns, errors,
  function boundaries, test isolation, effects, embedding, and parser/formatter behavior.
- [ ] Add canonical spec examples, overview updates, and a decision record.
- [ ] Run `nao check`, inspect diagnostics and a REPL smoke test, verify links,
  record results, and move this plan to `completed` before committing the feature.

# What assumptions and risks apply?

The existing value kinds remain string, bool, and unit. The internal non-fallthrough
type must propagate through initializers but is not an annotation users can declare.
Bindings are local values, not callable functions; binding a function's name hides
unqualified calls in that scope while qualified calls remain available.
Annotation parsing should reuse the existing result-type spelling rules.
The interpreter needs lexical state but no assignment or shared mutable language
values. Immutable runtime values may be copied when creating an embedded candidate
session; effects performed before a runtime failure remain observable as before.

# How will verification establish correctness?

Prefer colocated, data-driven source tests over constructing binding ASTs manually.
Test inferred and annotated primitive/unit values, conditional initializers,
initialization order, nested block/branch lookup, parameter collisions, failures
before effects, return and error cleanup, isolated calls/tests, persistent REPL
values, failed-session rollback, and absence of initializer replay. Validate source
spans, malformed declarations, semicolon rules, formatter idempotence, and executable
spec diagnostics. Run the complete `nao check` graph before feature commit.
