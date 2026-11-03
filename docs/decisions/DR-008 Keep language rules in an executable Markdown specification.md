# DR-008: Keep language rules in an executable Markdown specification

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Maintain canonical language rules in numbered chapters under `docs/spec`.
Use labeled source fences and explicit Output, Eval result, or Error expectations
that the spec harness extracts and executes through the engine. Keep one canonical
home per rule, link related chapters, and retain the current diagnostic expectation format.

## Why was this chosen?

Executable examples connect readable rules to implementation behavior.
Canonical ownership reduces duplicated prose and contradictory examples. Exact
error expectations preserve diagnostic behavior at the cost of presentation-sensitive updates.

## What is the context?

The spec-validation work introduced the harness; the later consolidation
merged overlapping chapters and placed lexer failures beside the relevant syntax.
Semantic-error assertions were discussed as a future tooling change, not adopted.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Spec authoring and chapter ownership](<../spec/README.md>)
- [Example extraction](<../../crates/spec_validation/src/load_spec_chapters.rs>)
- [Execution and formatting checks](<../../crates/spec_validation/src/execute_spec_example.rs>)
- [Spec consolidation and deferred tooling](<../plans/completed/2026-11-02-streamline-spec-plan.md>)

## What are the consequences?

- Examples must obey the harness's labels, entry filenames, and expectation headings.
- Parseable source examples must match the formatter's output.
- Error rendering changes can require snapshot edits; the current format remains authoritative.
- Rust tests retain exhaustive edge coverage where repeated spec examples add little explanation.

## What alternatives were considered?

- **Prose without executable examples:** Lets implementation drift go unnoticed.
- **Repeat every rule in related chapters:** Makes maintenance and consistency harder.
- **Replace full error snapshots with semantic assertions now:** Requires a new
  validator contract; it was deferred during editorial consolidation.
- **Put all tests in the spec:** Buries readable rules beneath implementation edge cases.
