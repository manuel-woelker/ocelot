# What will this plan simplify?

The spec repeats rules and examples across declarations, runtime behavior, and
library chapters. Historical implementation prose obscures current behavior,
and the README has an outdated index and authoring guidance.

# What will be implemented?

- [ ] Inventory executable examples and identify redundant cases before editing.
- [ ] Merge test syntax and runtime behavior into the existing test declaration chapter.
- [ ] Merge eval results into embedded evaluation and link to its rendering rules.
- [ ] Move block-comment failures into comments and string failures into a string syntax chapter.
- [ ] Give function syntax/calls, module lookup, and file execution distinct ownership with cross-links.
- [ ] Remove historical boilerplate and redundant headings; trim duplicate examples while retaining distinct coverage.
- [ ] Rewrite the README as a current reading guide and concise authoring reference.
- [ ] Validate examples, check local links and example coverage, and run `nao check`.
- [ ] Record verification and move the completed plan to `completed`.

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
