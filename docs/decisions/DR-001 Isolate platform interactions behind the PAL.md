# DR-001: Isolate platform interactions behind the PAL

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Use `ocelot_pal::pal::Pal` for filesystem access, clocks, watching, process
execution, and related platform behavior. Share implementations through `PalHandle`;
use `PalReal` in production and prefer `PalMock` for platform-dependent tests.
Keep the asynchronous process runtime private to `PalReal` behind the synchronous,
event-delivering process API.

## Why was this chosen?

A shared boundary lets compilation and execution use deterministic test inputs
without requiring real processes or files for every test. Keeping runtime machinery
inside the real adapter avoids spreading async dependencies through domain APIs.

## What is the context?

The repository already exposes a PAL trait, real and mock implementations,
and process-event types. Contributor guidance requires the boundary; real-filesystem
regressions complement mock-based tests.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Contributor guidance](<../../AGENTS.md>)
- [PAL contract](<../../crates/pal/src/pal.rs>)
- [Real adapter and runtime rationale](<../../crates/pal/src/pal_real.rs>)
- [Mock adapter](<../../crates/pal/src/pal_mock.rs>)
- [Path-coordinate finding](<../learnings/2026-11-02 Directory walks must preserve root coordinates.md>)

## What are the consequences?

- Platform-dependent features must extend or use the PAL rather than bypass it.
- The abstraction and mock must stay consistent; real-adapter tests remain necessary.
- A synchronous public boundary can block its caller even when implementation I/O is async.
- Path-coordinate findings are preserved separately in development learnings.

## What alternatives were considered?

- **Direct OS calls in each crate:** Make platform behavior harder to substitute
  consistently and scatter filesystem and process policy.
- **An async runtime in every caller:** Expands the public dependency and execution
  model beyond what current compiler and CLI callers need.
- **Mock-only verification:** Cannot establish the behavior of the real platform adapter.
