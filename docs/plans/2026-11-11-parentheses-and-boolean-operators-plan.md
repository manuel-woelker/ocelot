# What language rule will this implement?

There is no precedence between operators. At each expression level, different
operators must be separated by explicit parentheses. This includes prefix `not`:
`not a == b`, `a == b and c`, and `a and b or c` are errors. Write `(not a) == b`,
`(a == b) and c`, or `a and (b or c)` to state the intended structure.

Allow repeated `not`, and left-to-right chains of the same `and` or `or`.
Keep equality non-chainable, including repeated `==`, as already specified;
group nested comparisons explicitly. Calls are syntactic operands rather than
operators: `not ready()` and `ready() and enabled` remain valid. Arguments,
conditions, interpolation expressions, and block bodies have their own expression
boundaries; this rule does not inspect operators hidden inside those boundaries.

# What will parentheses mean?

`(expression)` groups one expression and preserves its type, value, effects,
and early-return behavior. It does not create a lexical scope or discard a value.
Retain parentheses in the AST and formatter, including redundant/nested groups.
Grouped identifier callees remain direct calls; grouping does not introduce
first-class functions. Empty `()` is not a value literal in this change; use `{}`
for unit. Existing `()` type annotations and call argument syntax are unchanged.
Preserve existing restrictions on comments inside expressions rather than dropping
comments during formatting.

# How will boolean operators behave?

Add reserved keywords `and` and `or`, requiring boolean operands and producing
boolean results. Evaluate left once. `and` skips right when left is false; `or`
skips right when left is true. Returns and errors propagate only along evaluated
paths. Statical checking still resolves both operands and conservatively tracks
both operands' effects, even when right is skipped for a literal left value.

A non-fallthrough left makes the entire expression non-fallthrough. A returning
right alone does not, since short-circuiting may bypass it. Do not add constant
truth analysis in this change. Grouping preserves the internal non-fallthrough type.

# In what order will the work be implemented?

- [ ] Add separate grouped-expression and logical-expression AST types, logical
  operator enum, and lexer keywords.
- [ ] Parse explicit groups and homogeneous boolean chains without a precedence
  table; reject mixed operators and ungrouped equality chains with useful errors.
  Preserve spans and distinguish grouping from call/annotation parentheses.
- [ ] Resolve group/call transparency, boolean operand types, conservative effects,
  and non-fallthrough behavior. Implement interpreter short-circuiting and guarded
  C lowering with existing cleanup/return semantics.
- [ ] Preserve explicit groups and operator spelling in the formatter, checking
  parse/format idempotence and existing comment rules.
- [ ] Add source-driven tests for syntax, mixing in both operand positions,
  grouped calls, malformed groups, truth tables/chains, short-circuit effects and
  errors, returns on either side, static errors/effects in skipped operands,
  binding scope, embedded sessions, and native/interpreter parity.
- [ ] Replace current precedence-dependent examples/tests and update the executable
  spec and overview. Supersede DR-018 with DR-022 while retaining equality's type,
  evaluation-order, and non-chaining rules.
- [ ] Run `nao check` (including every spec example in both modes), strict C and
  sanitizer checks, documentation link checks, and `git diff --check`. Record
  results and move this plan to completed before the implementation commit.

# What assumptions and risks require attention?

This deliberately rejects previously accepted `not`/equality mixtures. Update
their fixtures to explicit groups without weakening the new syntax rule.
Do not accidentally infer unconditional returns from a short-circuited right
operand, hide errors/effects in skipped code, or emit the C right operand before
its guard. Grouped callees must retain existing function lookup and effect behavior.
Do not add arithmetic, tuples, unit literals, first-class functions, or an implicit
operator ordering while adding parentheses.

# How will completion be established?

The same source-driven spec expectations must pass in interpreter and native
modes, including effect/return ordering and exact rejection diagnostics. Rust
tests cover AST spans and formatter round-trips, and targeted native regression
tests exercise skipped allocations and return cleanup under sanitizers. All
implementation work remains pending when this plan is committed.
