# What belongs in development learnings?

Record reusable findings from development: complex debugging discoveries,
surprising behavior, disproved assumptions, and performance or platform pitfalls.
Create an entry when its evidence could save someone from repeating the investigation.
Routine edits and untested guesses do not need a record.

Consult related entries before investigating unfamiliar behavior. Keep findings
alongside the work that establishes them; update an existing entry when new evidence
changes its conclusion or limits. Preserve useful context rather than silently
replacing an earlier conclusion.

# How are records named?

Use `YYYY-MM-DD Finding title.md`, with an ISO 8601 date for when the finding was
established. For example, `2026-11-02 Directory walks must preserve root coordinates.md`.
Descriptive titles distinguish findings from the same day; sequential IDs are unnecessary.

# What must each record contain?

- **Finding:** The observation, practical implication, and limits. Distinguish
  evidence from hypotheses or inference.
- **Context:** The task, original assumption, environment, relevant versions, and constraints.
- **Evidence:** Reproduction steps, tests, measurements, or logs, including conditions
  and expected versus observed behavior. Claim only verification actually performed.
- **References:** Relevant internal code, tests, documentation, plans, or commits,
  and external primary documentation, papers, or issues. Include both kinds when
  available; explicitly state when either has no applicable source. Explain enough
  that the finding remains useful if a link breaks.

Use question headings for these four sections, as in the template. Keep secrets and
sensitive raw logs out of records. Link plans and architectural decisions rather
than duplicating their rationale: learnings preserve what the evidence established.
Language rules belong in [the spec](../spec/README.md), and implementation steps
belong in [plans](../PLANS.md).

# What template should I use?

Copy this into a dated file and replace the placeholders:

```markdown
# What was discovered?

- Learned: YYYY-MM-DD

## What is the finding?

Observed behavior, practical implication, confidence, and limits.
Separate inference from directly observed evidence.

## What is the context?

Task, original assumption, environment, relevant versions, and constraints.

## What evidence supports the finding?

Reproduction steps, tests, commands, or measurements and their observed results.

## What references apply?

- Internal: [Relevant implementation or test](../../path/to/file).
- External: Link applicable primary sources, or state that none applies.
```

# Where did this convention come from?

This mechanism is adapted from Noice's development-learnings guide and
`DR-012 Record development learnings`: dated, evidence-backed findings maintained
with the work that establishes them. Ocelot uses its existing question-driven
documentation style and needs no additional tooling. Related architectural and
product choices are recorded in [decision records](../decisions/README.md).
Ocelot's adoption is recorded in [DR-012](<../decisions/DR-012 Record reusable development learnings with evidence.md>).
