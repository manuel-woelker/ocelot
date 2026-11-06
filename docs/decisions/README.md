# Where are decisions recorded?

This directory preserves significant technical and product choices. Follow
[DR-000](<DR-000 Record decisions in the repository.md>) for numbering, required
content, acceptance, and supersession. Use `DR-NNN Decision title.md` with the next
unused identifier; IDs remain stable even when a decision is rejected or superseded.

# Which decisions are recorded?

| Record | Decision | Status |
| --- | --- | --- |
| [DR-000](<DR-000 Record decisions in the repository.md>) | Record decisions in the repository | Accepted |
| [DR-001](./DR-001%20Isolate%20platform%20interactions%20behind%20the%20PAL.md) | Isolate platform interactions behind the PAL | Accepted |
| [DR-002](./DR-002%20Use%20SharedString%20for%20shared%20immutable%20text.md) | Use SharedString for shared immutable text | Accepted |
| [DR-003](./DR-003%20Resolve%20programs%20into%20one%20canonical%20semantic%20model.md) | Resolve programs into one canonical semantic model | Accepted |
| [DR-004](./DR-004%20Use%20filesystem%20modules%20and%20explicit%20script%20extensions.md) | Use filesystem modules and explicit script extensions | Accepted |
| [DR-005](./DR-005%20Declare%20native%20functions%20in%20the%20core%20module.md) | Declare native functions in the core module | Accepted |
| [DR-006](./DR-006%20Propagate%20nominal%20effects%20through%20function%20calls.md) | Propagate nominal effects through function calls | Accepted |
| [DR-007](./DR-007%20Format%20from%20one%20AST%20with%20retained%20trivia.md) | Format from one AST with retained trivia | Accepted |
| [DR-008](./DR-008%20Keep%20language%20rules%20in%20an%20executable%20Markdown%20specification.md) | Keep language rules in an executable Markdown specification | Accepted |
| [DR-009](./DR-009%20Reuse%20source%20backed%20embedded%20sessions%20for%20the%20REPL.md) | Reuse source backed embedded sessions for the REPL | Accepted |
| [DR-010](./DR-010%20Discover%20declared%20tests%20and%20filter%20by%20names%20or%20paths.md) | Discover declared tests and filter by names or paths | Accepted |
| [DR-011](./DR-011%20Require%20argument%20labels%20with%20explicit%20positional%20opt%20in.md) | Require argument labels with explicit positional opt in | Accepted |
| [DR-012](./DR-012%20Record%20reusable%20development%20learnings%20with%20evidence.md) | Record reusable development learnings with evidence | Accepted |
| [DR-013](./DR-013%20Commit%20verified%20logical%20units%20using%20message%20files.md) | Commit verified logical units using message files | Accepted |
| [DR-014](<DR-014 Use explicit typed function returns.md>) | Use explicit typed function returns | Superseded by DR-015 |
| [DR-015](<DR-015 Use block values before expression conditionals.md>) | Use block values before expression conditionals | Accepted |
| [DR-016](<DR-016 Implement conditionals as expressions with compatible branch results.md>) | Implement conditionals as expressions with compatible branch results | Accepted |

DR-001 through DR-013 retrospectively document implemented choices and existing
project policy. Their dates record acceptance of these accounts on 2026-11-03,
not invented original decision dates. Linked current code and specifications take
precedence over superseded intermediate designs mentioned in historical plans.
Alternative comparisons are retrospective assessment unless the linked history
explicitly establishes the original discussion.

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
