# DR-002: Use SharedString for shared immutable text

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Use `ocelot_base::shared_string::SharedString` for stored strings when shared,
predominantly immutable ownership fits. Keep mutable builders and other justified
uses of `String`; this is not a blanket ban. The current wrapper uses `ecow::EcoString`.

## Why was this chosen?

Compiler names, source text, paths, and diagnostics are copied between data
structures. A common shared representation supports cheap cloning and consistent
conversions without committing every crate directly to the backing string library.
This rationale does not establish a measured performance gain for every workload.

## What is the context?

The base crate implements the wrapper and its serialization and conversion
support. The AST, semantic model, and contributor guidance already use it for
shared text. The original adoption date is not established by this record.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Shared-string policy](<../../AGENTS.md>)
- [Wrapper implementation and tests](<../../crates/base/src/shared_string.rs>)
- [Identifier representation](<../../crates/ast/src/identifier.rs>)

## What are the consequences?

- Shared text has a consistent repository type and cheap clone semantics.
- Mutation behavior and wrapper conversion costs still matter; builders may use `String`.
- The backing library is an implementation dependency, not a language-level string model.

## What alternatives were considered?

- **Owned `String` in every stored field:** Cloning repeatedly duplicates text
  even when shared ownership would fit.
- **Expose `EcoString` throughout the workspace:** Couples domain APIs directly to
  one backing library instead of the repository wrapper.
- **Replace all string uses indiscriminately:** Adds conversions where mutable or
  uniquely owned text is a better fit.
