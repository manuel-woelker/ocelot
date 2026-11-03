# DR-013: Commit verified logical units using message files

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Run `nao check` before committing useful units of work and leave completed
work committed. Separate distinct logical changes. For minor follow-ups to the
latest commit from the current task, rerun checks and amend it. Supply multiline
messages through a temporary file with git commit -F; do not use heredocs.
Retain Conventional Commits, User Prompts and Model sections, separate staging
and committing commands, and the prohibition on pushing.

## Why was this chosen?

Focused verified commits provide reviewable milestones. Amending minor
follow-ups avoids fragmenting one change, and message files avoid heredoc-based
message delivery while retaining the required prompt and model provenance.

## What is the context?

The user explicitly requested these commit-policy changes during the initial
review and spec work. Current contributor guidance records the policy; the checks
task covers formatting, build, lint, Rust tests, and executable spec validation.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Commit instructions](<../../AGENTS.md>)
- [Repository verification tasks](<../../.nao/nao.kdl>)

## What are the consequences?

- Completion includes verification and local commit creation.
- Minor follow-ups rewrite only the latest related commit; this is not permission
  to rewrite unrelated history or publish changes.
- Commit bodies retain the user prompts and model identifier for the unit.
- Different logical changes remain separately reviewable even when completed together.

## What alternatives were considered?

- **Leave completed changes uncommitted:** Conflicts with the requested workflow.
- **One commit for unrelated work:** Makes review and selective reversal harder.
- **A new commit for every minor correction:** Splits a useful unit across follow-ups.
- **Heredoc commit messages:** Explicitly disallowed by the requested policy.
