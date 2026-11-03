# Where are decisions recorded?

This directory preserves significant technical and product choices. Follow
[DR-000](<DR-000 Record decisions in the repository.md>) for numbering, required
content, acceptance, and supersession. Use `DR-NNN Decision title.md` with the next
unused identifier; IDs remain stable even when a decision is rejected or superseded.

# When should I create or consult a record?

Consult relevant records before changing architecture, public interfaces, project
conventions, or product behavior. Add a record when a significant choice needs
durable rationale; routine edits do not require one. Review it alongside the work.
Writing a proposal does not establish agreement or implementation.

Use [plans](../PLANS.md) for implementation tasks, [the spec](../spec/README.md)
for language rules, and [learnings](../learnings/README.md) for evidence-backed
discoveries. Cross-link them rather than duplicating their content.

# How should past decisions be documented?

Ground retrospective records in existing implementation, completed plans, or
commits. State which details are supported and which rationale or alternatives
are retrospective assessment. Record the current acceptance date when the
original decision date is unknown, and explicitly identify its meaning.
Do not present an old plan's abandoned design as the current accepted choice.

# What template should I use?

```markdown
# DR-NNN: Decision title

- Status: Proposed
- Date: YYYY-MM-DD

## What is the decision?

The concrete choice and its scope.

## Why was this chosen?

Requirements, constraints, rationale, and tradeoffs.

## What is the context?

The problem, assumptions, and links to supporting plans, code, or discussion.
Identify retrospective reconstruction when applicable.

## What are the consequences?

Benefits, costs, risks, limitations, and follow-up work.

## What alternatives were considered?

Plausible alternatives and a specific reason for not choosing each.
```
