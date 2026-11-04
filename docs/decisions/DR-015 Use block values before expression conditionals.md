# DR-015: Use block values before expression conditionals

- Status: Accepted
- Date: 2026-11-04
- Supersedes: [DR-014](<DR-014 Use explicit typed function returns.md>), specifically its requirement for explicit returns instead of tail values.

## What is the decision?

Treat `{ ... }` as an expression. A final expression without `;` supplies its
value; a semicolon discards the value, and an empty block produces unit.
Apply the same completion rule to function bodies. Check that the resulting
value matches the declared result type. Preserve explicit result annotations,
default unit annotations, native signature validation, and explicit early returns.

An explicit return exits the enclosing function even when it occurs inside a
block nested in another expression. Propagate normal return control separately
from runtime errors and consume it at the function-call boundary. Use an internal
non-fallthrough type for expressions that always return, permitting them wherever
a value is expected. Continue checking unreachable code and all inferred effects.

Establish this block foundation before adding conditionals. The intended subsequent
`if` construct is an expression whose branches use these block results; this
change does not introduce conditional syntax. Test bodies remain unit contexts.
Statement-position expressions, including blocks, require a discarding semicolon.

## Why was this chosen?

One block-value rule supports nested expressions, function completion, and later
conditional branch values. Semicolons make value retention explicit without a
second return keyword. Preserving declared result types keeps public signatures
stable and supports forward references without result inference.

## What is the context?

The user requested block result semantics first so that if can always be an
expression. DR-014 deliberately deferred tail returns; this decision revises that
choice while retaining its typed-signature and explicit-return foundation.

- [Block result specification](<../spec/02.04 Expressions - Blocks.md>)
- [Implementation plan](../plans/completed/2026-11-04-block-results-plan.md)
- [Block typing](../../crates/resolver/src/resolver.rs)
- [Expression return propagation](../../crates/interpreter/src/interpreter.rs)

## What are the consequences?

- Removing a final semicolon can change both a block's type and a function's result.
- An omitted function annotation still means unit; a string tail requires a string annotation.
- Returns in call arguments skip later arguments and the call itself. Returns inside
  called functions produce values for their caller normally.
- Template lexing must track braces inside each active interpolation.
- The internal non-fallthrough type is not a user-declared return type or runtime value.
- Future conditionals need branch-result compatibility and path analysis based on these rules.

## What alternatives were considered?

- **Keep explicit-return-only bodies:** Leaves blocks without values and prevents
  reuse of one completion rule for future expression conditionals.
- **Add statement-only if first:** Introduces a separate control-flow form before
  the expression-based design the user requested.
- **Infer function result annotations:** Changes the signature model beyond block results.
- **Treat returns as unit-valued expressions:** Would require a fake value where
  execution has already exited and incorrectly reject valid nested returns.
- **Use runtime errors for nested return control:** Conflates ordinary control flow with failures.
