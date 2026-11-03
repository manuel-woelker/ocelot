# Why must directory walks preserve their root's coordinate system?

- Learned: 2026-11-02

## What is the finding?

A directory walk must return paths in the same absolute or relative coordinate
system as its requested root. Relativizing every result against the process's
initial working directory breaks execution under an absolute root outside that
directory. Returning relative results for an absolute root inside it also creates
inconsistent entry-path identities.

The external-root failure was reproduced through the CLI. The identity problem
for mixed absolute and relative paths follows from the engine's exact path
comparisons; it is an inference from the implementation, not a separate recorded
CLI reproduction. This finding concerns path coordinates, not canonicalization,
symlink identity, or filesystem access restrictions.

## What is the context?

The initial Ocelot review exercised module execution and embedded evaluation under
an absolute temporary root while the process ran from the repository directory.
`PalReal` previously stripped its initial working-directory prefix from every
walk result. The engine also uses the caller's root and entry paths to discover
modules and identify the entry module.

## What evidence supports the finding?

Before the fix, running `ocelot /tmp/ocelot-review/main/main.ocelot` from the
repository failed with `Unable to relativize path` because the requested module
was outside the initial working directory.

The fix strips the requested walk root from each discovered path, then joins that
suffix onto the caller's original root. Colocated regression tests cover:

- `directory_walk_preserves_absolute_roots` and
  `directory_walk_preserves_relative_roots` in `pal_real.rs`.
- `executes_modules_and_embedded_calls_under_an_absolute_filesystem_root` in
  `engine.rs`, which exercises a main module, a helper module, and embedded eval
  against real files under the system temporary directory.

`nao check` passed with these tests after the fix. The original absolute-path CLI
case subsequently reached entrypoint validation instead of failing discovery.

## What references apply?

- Internal: [Directory walking and PAL regressions](../../crates/pal/src/pal_real.rs),
  [engine execution regressions](../../crates/engine/src/engine.rs),
  [module compilation and entry-path comparisons](../../crates/engine/src/compile_program.rs),
  and [completed review plan](../plans/completed/2026-11-02-initial-review-fixes-plan.md).
- Internal commit: `20c128f` (`fix(pal): preserve directory walk root coordinates`).
- Related decision: [DR-001](<../decisions/DR-001 Isolate platform interactions behind the PAL.md>)
  records why platform behavior belongs behind the PAL; this learning records the path evidence.
- External: No applicable external source; this finding is supported by repository
  behavior and tests.
