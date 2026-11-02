# What will this plan simplify?

The spec repeats rules and examples across declarations, runtime behavior, and
library chapters. Historical implementation prose obscures current behavior,
and the README has an outdated index and authoring guidance.

# What will be implemented?

- [x] Inventory executable examples and identify redundant cases before editing.
- [x] Merge test syntax and runtime behavior into the existing test declaration chapter.
- [x] Merge eval results into embedded evaluation and link to its rendering rules.
- [x] Move block-comment failures into comments and string failures into a string syntax chapter.
- [x] Give function syntax/calls, module lookup, and file execution distinct ownership with cross-links.
- [x] Remove historical boilerplate and redundant headings; trim duplicate examples while retaining distinct coverage.
- [x] Rewrite the README as a current reading guide and concise authoring reference.
- [x] Validate examples, check local links and example coverage, and run `nao check`.
- [x] Record verification and move the completed plan to `completed`.

# What assumptions and tradeoffs apply?

Keep existing chapter numbers and filenames where their subject remains useful.
Remove superseded chapters and update live links; historical completed plans remain
historical records. Add a numbered strings chapter rather than keep a separate
lexer-error category. Preserve exact expectation syntax and diagnostic snapshots:
semantic error assertions require a separate validator design and are deferred.
This is an editorial restructuring, with no language or validator behavior changes.
Question headings remain the documentation convention, except executable example
and expectation headings, whose exact spelling is required by the validator.

# How will coverage and correctness be verified?

Record every existing example before editing. Every removed example must either
move unchanged or have its rule demonstrated by a retained example; record the
intentional reductions in this plan. Run the executable spec harness after the
main merges, inspect links to retired chapters outside historical plans, then
run `nao check` for the full workspace. No new Rust tests are needed for prose
changes; the executable examples provide automated validation.

# Which examples were consolidated?

| Removed examples | Retained coverage |
| --- | --- |
| Separate true/false literals | One example prints both boolean literals. |
| Boolean chapter negation | Prefix-negation chapter retains simple and nested negation. |
| Basic call used as a statement | Typed call example demonstrates the same statement form. |
| Standalone nominal effect declaration | Caller-inherits-effect example retains the declaration and call relationship. |
| Declaration chapter's two test examples | Merged chapter retains test-only and mixed-script execution examples. |
| Standalone leading line comment | Comment-around-statement example covers leading and trailing positions. |
| Script printing one line | Library chapter retains the basic println example. |
| Each println call writes a line | Script order and boolean println examples retain multiple output lines. |
| Library template-string example | Template-string chapter retains the same interpolation behavior. |

55 existing examples retain their source and expectation fences verbatim. A new
combined boolean-literal example replaces the two separate literal examples,
leaving 56 examples across 17 numbered chapters. Startup examples moved from
file modules to file execution; eval-result and lexer-error examples moved to
their canonical chapters. Chapter text decreased from 1,836 to 1,438 lines.

# What verification has completed?

The standalone spec harness passed all 56 examples after consolidation.
A local-link audit found no broken links within the spec. References to retired
chapters outside the spec are confined to historical completed plans, which remain
unchanged. Source/expectation comparison confirmed all 55 retained examples are
unchanged. The validator's existing expectation syntax and full diagnostics are
preserved; semantic-error assertions remain a separate tooling proposal.

`nao check` passed after the final edits, covering build, formatting, clippy,
Rust tests, and spec validation. `git diff --check` passed. All planned editorial
work is complete; no language implementation or validator changes were needed.
