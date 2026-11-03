# DR-006: Propagate nominal effects through function calls

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Declare effect names with `effect name;`. A function's `can effect` introduces
that effect; it is not an exhaustive permission list. Infer effects transitively
through calls and reject a `cannot effect` prohibition when the function's body
or reachable callees contribute it. Permit one can clause followed by one cannot
clause, with builtin effects declared in core.

## Why was this chosen?

Call-graph propagation makes prohibitions useful without requiring every
caller to repeat all effects. Nominal labels fit the existing function and index
tables while keeping the initial system smaller than effect polymorphism.

## What is the context?

The completed nominal-effect plan defines lightweight function-level metadata
and transitive checking. Current propagation computes a fixed point over resolved
functions before checking prohibitions, including effects through call cycles.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Effect rules](<../spec/27.01 Effects - Nominal effect declarations.md>)
- [Fixed-point propagation](<../../crates/resolver/src/effect_propagation.rs>)
- [Effect-system plan](<../plans/completed/2026-04-04-nominal-effect-system-plan.md>)

## What are the consequences?

- A caller can inherit effects without a can annotation.
- Effects describe possible behavior, not proof that an effect executes at runtime.
- Native functions contribute declared builtin effects.
- Effect variables, polymorphism, and lists of effects in one clause remain unsupported.

## What alternatives were considered?

- **Treat can as an exhaustive allow-list:** Requires annotation of every
  inherited effect and contradicts the intended lightweight inference model.
- **Check only directly written calls:** Misses prohibited transitive effects.
- **Add effect polymorphism immediately:** Adds type-system machinery beyond the
  current nominal checking requirements.
