# DR-000: Record decisions in the repository

- Status: Accepted
- Date: 2026-11-03

## What is the decision?

Record significant technical and product decisions as Markdown files in
`docs/decisions/`. Each record explains the decision, its rationale and context,
its consequences, and the alternatives considered with reasons for not choosing them.

Use `DR-NNN Decision title.md`, with sequential identifiers starting at `DR-000`.
Capitalize the first letter of the title and use spaces between words. The document
title includes the same identifier and title. Assign the next unused number; never
renumber or reuse an identifier, including one belonging to a rejected proposal.

Every record must contain:

- **Status and date:** Proposed, Accepted, Rejected, or Superseded, and the date
  it reached that status.
- **Decision:** The concrete choice and its scope.
- **Rationale:** Why the choice meets the requirements, including its tradeoffs.
- **Context:** The problem, requirements, constraints, and assumptions.
- **Consequences:** Benefits, costs, limitations, risks, and follow-up work.
- **Considered alternatives:** Plausible alternatives and specific reasons for
  not choosing them, including keeping the existing approach where relevant.

Use the question-driven headings in the [guide and template](README.md).
Create a record when a choice materially affects architecture, public APIs,
project conventions, or product behavior, especially when reversal would be costly
or its reasoning easy to lose. Routine changes and individual bug fixes do not
automatically require records.

Review records alongside their implementation. A proposal becomes Accepted when
the choice is agreed upon; writing a proposal alone does not establish acceptance.
Link relevant records from related documentation and implementation discussions.

Keep accepted records as historical accounts. Correct typos or clarify wording
without changing their meaning. When a decision changes, create a new record,
mark the earlier one Superseded, update its status date, and link both records
to each other. Resolve identifier collisions before merging concurrent branches.

For retrospective records, identify the supporting code, plans, or commits and
state that the rationale and alternatives are reconstructed where original
deliberations are unavailable. Do not invent historical acceptance dates. The
status date may reflect acceptance of the retrospective record, with that meaning
made explicit. Current code and the spec determine implemented behavior; a record
does not turn an unimplemented proposal into a feature.

## Why was this chosen?

Versioned records make reasoning discoverable alongside the code. Explicit
alternatives distinguish deliberate tradeoffs from overlooked options, and
preserving superseded records explains how the project evolved.

## What is the context?

Ocelot already keeps specifications, implementation plans, tests, and development
learnings. Architectural and product decisions are scattered across those sources
and conversations. The user requested adoption of Pixui's DR-000 convention and
records for past decisions. This record adapts Pixui's accepted DR-000, dated
2026-10-03, to Ocelot's question-driven documentation and retrospective backfill.

## What are the consequences?

- Significant choices and their tradeoffs become reviewable and searchable.
- Authors must maintain status, dates, references, and links to successors.
- Historical records may describe behavior that has since changed; check status
  and supporting implementation before treating them as current guidance.
- Markdown and normal review suffice; no new tool or dependency is required.
- Plans describe implementation steps, the spec describes language rules, and
  learnings preserve observed evidence. Records link these sources without
  replacing or duplicating them.

## What alternatives were considered?

- **Conversations and commit messages alone:** Reasoning remains scattered and
  often omits consequences and alternatives.
- **An external wiki:** Adds a separate access dependency and separates decisions
  from code review and version history.
- **Only comments, plans, and current documentation:** Useful supporting sources,
  but lack a consistent history of accepted and superseded choices.
- **A record for every change:** Routine entries would obscure significant choices.
- **Dedicated decision-management tooling:** Adds maintenance without a present
  need beyond Markdown and repository review.
