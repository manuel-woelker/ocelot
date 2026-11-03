# DR-014: Use explicit typed function returns

- Status: Accepted
- Date: 2026-11-03

## What is the decision?

Declare function results with `-> string`, `-> bool`, or `-> ()` after the
parameter list and before effect clauses. An omitted annotation means unit.
Use explicit `return expression;` or bare `return;` for unit. Validate signatures
before bodies so calls carry the declared type, including forward and recursive
calls. Native source declarations must match registered result signatures.

Require returns to match the result type and reject non-unit fallthrough.
The current statement language is straight-line, so an explicit return is sufficient
to prevent fallthrough. Keep checking every statement, including unreachable ones,
and conservatively propagate their effects. Returns exit the current function;
script and test bodies cannot return. Direct module execution discards main's
result, while embedded eval returns the runtime value to its host.

## Why was this chosen?

Typed results let functions compose into call arguments, negation, and templates
without weakening static checking. Explicit declarations support forward references
without body-driven result inference. Explicit return statements make early exit
and value production visible while retaining the existing expression-statement model.

## What is the context?

The user requested typed return values after closing the unresolved-value semantic
gap. The existing runtime already represents strings, booleans, and unit; function
calls previously always had unit type. No new value kind or execution backend is
needed. The semantic-gap fix remains independently committed as `c16e96e`.

- [Implementation plan](../plans/completed/2026-11-03-typed-return-values-plan.md)
- [Canonical return rules and examples](<../spec/20.01 Functions - Return values.md>)
- [Signature registration](../../crates/resolver/src/declaration_indexer.rs)
- [Return and call checking](../../crates/resolver/src/resolver.rs)
- [Runtime return propagation](../../crates/interpreter/src/interpreter.rs)

## What are the consequences?

- Result types become part of the public calling interface and are checked at use sites.
- Existing functions with no annotation retain unit behavior; final expression
  statements do not become implicit returns.
- Return control flow stays separate from runtime errors and exits only one invocation.
- Unsupported or unknown result types, including `any`, are rejected.
- Unreachable statements remain checked and may contribute inferred effects.
- Conditional or looping statements will require path-sensitive return analysis.
- Local variables, result inference, implicit tail returns, and exit-code conventions
  remain separate future design questions.

## What alternatives were considered?

- **Keep every function unit-valued:** Prevents useful expression composition.
- **Infer result types from bodies:** Adds dependency and recursion inference work
  beyond the explicit-signature model requested here.
- **Implicitly return the last expression:** Changes expression-statement semantics
  and introduces block-result rules before explicit returns are established.
- **Use runtime errors to carry returns:** Conflates normal control flow with failures.
- **Check only reachable statements:** Would weaken name, type, and effect validation
  depending on statement position; the initial model keeps static checks conservative.
