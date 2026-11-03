# DR-003: Resolve programs into one canonical semantic model

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Load and parse sources in the engine, orchestrate semantic resolution in the
resolver, and return a `ResolvedProgram` containing modules, source diagnostics,
entry path, and the canonical `SymbolTable`. Execute resolved ASTs against that
table rather than rebuilding a parallel program environment. Use typed indices
for function, type, and effect references; keep runtime values in the semantic crate.

## Why was this chosen?

A single semantic representation reduces duplicated state and keeps resolution
pass ordering out of command execution. Typed handles distinguish reference kinds.
Shared runtime values support native dispatch and interpretation without putting
runtime definitions in the syntax-only AST crate.

## What is the context?

Completed resolver simplification plans explicitly removed overlapping program
representations and narrowed compilation inputs and import scratch state. The
current resolver owns declaration registration, body resolution, and effect propagation.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Resolved program](<../../crates/semantic/src/resolved_program.rs>)
- [Canonical semantic table](<../../crates/semantic/src/symbol_table.rs>)
- [Runtime-value placement rationale](<../../crates/semantic/src/runtime_value.rs>)
- [Resolution orchestration](<../../crates/resolver/src/resolution.rs>)
- [Completed consolidation plan](<../plans/completed/2026-04-06-engine-resolver-execution-simplification-plan.md>)

## What are the consequences?

- Execution depends on resolved references and canonical metadata.
- Diagnostics remain available to callers independently of rendered CLI output.
- The current backend walks resolved ASTs; this does not commit future backends to that strategy.
- Compiler inputs, resolver scratch state, and resolved output have distinct roles.

## What alternatives were considered?

- **Separate resolver and interpreter program tables:** Require synchronization
  and duplicate ownership of essentially the same semantic state.
- **Engine-controlled low-level resolution passes:** Leak pass ordering and make
  partial or inconsistent resolution workflows easier to construct.
- **Put runtime values in the AST:** Blurs syntax and runtime responsibilities.
