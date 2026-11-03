# DR-007: Format from one AST with retained trivia

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Retain comment and newline trivia on lexer tokens and attach it to compilation
units, items, and statements. Format from the same AST used by compilation.
Support comments around items and statements, including same-line trailing comments,
while rejecting interior expression and parameter-list comment positions.

## Why was this chosen?

One enriched AST avoids maintaining a second syntax representation while
preserving comments in supported locations. Restricting placement keeps ownership
of trivia clear until broader token-level formatting is justified.

## What is the context?

The formatter plan explicitly chose trivia on the shared AST instead of a
separate concrete syntax tree. The CLI formats parsed sources and replaces changed
files through a temporary sibling file and rename.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Trivia representation](<../../crates/ast/src/trivia.rs>)
- [Trivia placement and validation](<../../crates/parser/src/parser.rs>)
- [Formatter and round trips](<../../crates/formatter/src/format_compilation_unit.rs>)
- [Formatter representation plan](<../plans/completed/2026-04-05-formatter-crate-plan.md>)

## What are the consequences?

- Formatting normalizes spacing and layout while retaining supported comments.
- Unsupported comment positions are parse errors rather than silently discarded text.
- The AST carries formatting metadata in addition to semantic syntax.
- Broader comment placement would need a deliberate representation change.

## What alternatives were considered?

- **Discard comments while lexing:** Prevents source rewriting without losing text.
- **Introduce a separate concrete syntax tree immediately:** Duplicates syntax
  infrastructure beyond the current formatter's placement rules.
- **Accept comments everywhere without ownership:** Risks losing or moving comments
  when emitting source from the AST.
