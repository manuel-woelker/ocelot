# DR-012: Record reusable development learnings with evidence

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Maintain dated `YYYY-MM-DD Finding title.md` records under `docs/learnings`.
Include the finding, context, evidence, and references. Record reusable discoveries
alongside the work that establishes them, distinguish observation from inference,
and explicitly identify when applicable internal or external sources are absent.

## Why was this chosen?

Evidence-backed findings prevent repeated investigation while keeping observations
distinct from choices and implementation plans. A lightweight Markdown convention
preserves useful limits and reproduction conditions without creating a diary.

## What is the context?

The user requested integration of Noice's learnings mechanism. Ocelot adopted
its guide and template, contributor links, and a first directory-walk record that
separates the reproduced external-root failure from an inferred identity problem.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Learning guide and template](<../learnings/README.md>)
- [Initial evidence-backed record](<../learnings/2026-11-02 Directory walks must preserve root coordinates.md>)
- [Contributor learning workflow](<../../AGENTS.md>)

## What are the consequences?

- Routine changes need no learning; meaningful findings should be discoverable.
- Findings must be corrected or qualified as new evidence changes them.
- Link related decisions rather than copying their rationale into evidence records.
- Avoid secrets and sensitive logs; no new runtime tool or dependency is required.

## What alternatives were considered?

- **Use decision records for every observation:** Conflates evidence with choices.
- **Leave findings only in chat or logs:** Makes the conclusion and reproduction
  conditions harder to recover.
- **Require an entry for every edit:** Buries reusable discoveries in routine updates.
