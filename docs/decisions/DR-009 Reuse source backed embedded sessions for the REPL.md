# DR-009: Reuse source backed embedded sessions for the REPL

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Expose persistent `EmbeddedEngine` sessions rooted at a base path. Successful
execute calls retain source; subsequent calls compile candidate source through
the common module pipeline and execute only newly supplied statements. Eval accepts
exactly one expression and returns a runtime value without retaining its source.
Build the CLI REPL on this API.

## Why was this chosen?

Sharing the pipeline avoids a second language implementation for interactive
use. Retained source makes earlier definitions available without replaying old
statements. Candidate compilation prevents invalid source from replacing the last
valid session.

## What is the context?

The embedding and REPL plans established persistent sessions and the host API.
The review fixes tightened eval to one expression before execution. The current
REPL tries eval and falls back to execute for parser-rejected statement input.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Session implementation and regressions](<../../crates/engine/src/embedded_engine.rs>)
- [REPL integration](<../../crates/cli/src/main.rs>)
- [Session and result rules](<../spec/28.03 Runtime behavior - Embedded eval.md>)
- [Embedding plan](<../plans/completed/2026-04-10-embedded-engine-eval-plan.md>)
- [REPL plan](<../plans/completed/2026-04-10-repl-cli-command-plan.md>)

## What are the consequences?

- Definitions survive successful execute calls; previous effects are not replayed.
- Failed calls do not commit source, but cannot roll back effects already performed.
- Each call recompiles source and reloads modules; this is not an incremental compiler.
- The REPL currently reads one line at a time; multiline editing is outside this interface.

## What alternatives were considered?

- **A fresh engine for each expression:** Loses persistent definitions.
- **Replay accumulated statements:** Repeats prior observable effects.
- **A separate REPL interpreter:** Duplicates parser and semantic behavior.
- **Incremental mutable semantic state immediately:** Adds rollback and dependency
  invalidation machinery before current embedding requirements need it.
