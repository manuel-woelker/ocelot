# DR-017: Use immutable local bindings and lexical scopes

- Status: Accepted
- Date: 2026-11-06

## What is the decision?

Add `let name = expression;` with inferred types and optional `string`, `bool`, or
`()` annotations. Require initialization, evaluate it once, and introduce the name
afterward. Reject duplicates within one scope and allow shadowing in nested blocks.
Function parameters share the body's scope. Functions and tests cannot capture
script values; each test starts independently. Returns and errors unwind nested scopes.

Persist successful embedded top-level values alongside retained source. Evaluate
new submissions against a candidate value snapshot and commit source and values
only on success. Do not replay initializers or undo external effects on failure.

## Why was this chosen?

Immutable names make conditional and block results reusable without introducing
assignment or mutable runtime state. Declaration order and lexical shadowing keep
lookup predictable. Isolating functions avoids implicit script initialization and
capture dependencies. A session needs runtime values as well as source to preserve
bindings without repeating observable initialization effects.

## What is the context?

[Block values](<DR-015 Use block values before expression conditionals.md>) and
[conditionals](<DR-016 Implement conditionals as expressions with compatible branch results.md>)
already produce typed results. Previously only parameters bound local values.
This extends [source-backed sessions](<DR-009 Reuse source backed embedded sessions for the REPL.md>)
with immutable value snapshots.

- [Plan](../plans/completed/2026-11-06-local-bindings-plan.md)
- [Language rules](<../spec/05.01 Statements - Local bindings.md>)
- [Resolver](../../crates/resolver/src/resolver.rs)
- [Interpreter](../../crates/interpreter/src/interpreter.rs)
- [Embedded session](../../crates/engine/src/embedded_engine.rs)

## What are the consequences?

Bindings compose with all existing expression forms and retain static effect checks.
An initializer that returns exits before binding its name. Script values remain
private to that execution, while successful REPL values remain accessible to later
submissions. Candidate snapshots copy immutable values; sessions still recompile
retained source. Mutation, destructuring, closures, and module-level value exports
remain future work.

## What alternatives were considered?

- **Assignment and mutable bindings now:** Adds state and update rules before basic
  lexical bindings have been established.
- **Reject all shadowing:** Prevents useful nested reuse of names; same-scope
  duplicate rejection already catches accidental redeclarations.
- **Capture script values in functions:** Introduces initialization-order and
  lifetime dependencies requiring a separate capture design.
- **Replay retained source:** Repeats effects and may produce different values.
- **Commit bindings incrementally during failed submissions:** Leaves session
  values inconsistent with retained source.
