# DR-011: Require argument labels with explicit positional opt in

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Require `parameter: expression` for ordinary function parameters. A parameter
declared `positional name: type` permits omission of its label while still accepting
the explicit label. Preserve declaration order and left-to-right evaluation.
Positional parameters precede parameters that require labels; unlabeled arguments
precede labeled arguments. `println` and `assert` permit omission;
`assert_eq` requires `expected:` and `actual:` labels.

## Why was this chosen?

Required labels make argument roles explicit, especially for booleans and
same-typed values. An explicit per-parameter modifier puts the exception in the API
declaration instead of asking the compiler to judge which functions are obvious.

## What is the context?

The user requested keyword calls, exceptions for obvious cases, and a clearer
alternative to an underscore marker. The completed plan and current spec establish
the explicit positional modifier for user-defined and native functions.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Calling convention](<../spec/02.01 Expressions - Function calls.md>)
- [Declaration syntax and positional examples](<../spec/15.02 Declarations - Function definitions.md>)
- [Label validation](<../../crates/resolver/src/resolver.rs>)
- [Completed keyword-argument plan](<../plans/completed/2026-11-03-keyword-arguments-plan.md>)

## What are the consequences?

- Parameter names are public calling API; renaming them requires caller updates.
- Missing, unknown, duplicate, or reordered labels fail during semantic validation.
- The formatter preserves explicit labels even when omission is permitted.
- External/internal label aliases, shorthand, defaults, and argument reordering are unsupported.

## What alternatives were considered?

- **Positional calls by default:** Preserve the ambiguity the user wanted removed.
- **An underscore omission marker:** Less explicit than spelling positional in declarations.
- **Compiler-chosen obvious-function exceptions:** Hide policy outside the declaration.
- **Positional-only marked parameters:** Prevent explicit labels even when a caller
  prefers them; the accepted modifier permits omission rather than requiring it.
- **Immediate argument reordering:** Adds mapping and evaluation-order complexity
  beyond the accepted source-order convention.
