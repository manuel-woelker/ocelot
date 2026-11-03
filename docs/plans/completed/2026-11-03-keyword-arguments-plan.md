# What behavior will change?

Function arguments require `name: expression` by default. A parameter declared
`positional name: type` permits either a labeled argument or omission of its label.
The policy is part of the declaration for user-defined and native functions.

# What rules will the implementation enforce?

- Arguments follow declaration order and evaluate left to right; reordering is not supported.
- Positional parameters precede parameters that require labels.
- Unlabeled arguments precede labeled arguments at a call site.
- Labels must match their corresponding parameter; unknown and duplicate labels are errors.
- Required labels cannot be omitted. Existing arity and type checks remain in force.
- `println(positional value: any)` and `assert(positional condition: bool)` permit
  omission; `assert_eq(expected: any, actual: any)` requires both labels.
- `positional` is a reserved keyword. Empty calls remain unchanged.

# What will be implemented and in what order?

- [x] Add AST argument labels and parameter policy, then lexer/parser support and formatting.
- [x] Preserve declaration parameter metadata for native and user-defined functions;
  enforce label, order, arity, and type rules before effects can execute.
- [x] Evaluate argument values in source order and bind them to the corresponding parameters.
- [x] Update builtins, executable examples, fixtures, and tests to the new syntax.
- [x] Add colocated parser, formatter, resolver, and engine regressions, including
  imported/qualified/native calls, templates, side effects, and invalid inputs.
- [x] Update the canonical function-call and function-definition specification chapters.
- [x] Run `nao check`, inspect diagnostics and formatting, record verification, and
  move this plan to `completed` before committing the completed feature.

# What assumptions and risks apply?

This intentionally breaks existing unlabeled calls to ordinary parameters. Parameter
names become public API. There are no aliases, shorthand labels, default arguments,
overloads, or argument reordering in this change. The formatter preserves explicit
labels even where omission is permitted. Existing diagnostic snapshots need updated
source spans when calls gain labels. Native runtime signatures still describe types;
the Ocelot declarations define labels and omission policy.

# How will verification work?

Use data-driven black-box tests for required/optional labels, mixed signatures,
unknown/duplicate/out-of-order labels, missing labels, malformed syntax, arity,
type errors, and evaluation order. Verify parser metadata and formatter round trips.
Migrate all current executable examples without removing their distinct rules;
run the full spec harness and Rust suite through `nao check`. The AST/parser/formatter
work may land as a separately verified preparatory commit before semantics switch.

# What was implemented and verified?

The feature landed as one coherent implementation unit after the plan commit.
`CallArgument` retains explicit labels and values; resolved function metadata now
retains parameter names, types, spans, and positional policy instead of only types.
The interpreter evaluates values in source order without argument reordering.
The formatter preserves explicit labels and positional declarations.

Colocated regressions cover parsing and source spans, malformed inputs, formatting
round trips, required and optional labels, duplicate/unknown/reordered labels,
declaration order, arity and types, native calls, imported/qualified calls, local
value forwarding, evaluation order, and persistent embedded sessions. Rejected
calls are checked before effects execute. Several manually assembled test ASTs
were replaced with parsed source fixtures so labels and spans cannot drift apart.

`nao check` passed, including build, formatting, clippy, Rust tests, and all 59 spec
examples. `git diff --check` passed. Direct script execution of the migrated
boolean and negation examples passed. A broader `ocelot test booleans,not` smoke
run passed those example tests but also reported the repository's existing
malformed comment and unterminated-string examples during discovery; those
negative examples were left unchanged and are outside this feature.

No planned work remains. General argument reordering and label aliases remain
outside the supported calling convention.
