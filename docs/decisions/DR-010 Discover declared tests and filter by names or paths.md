# DR-010: Discover declared tests and filter by names or paths

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Keep tests as named top-level declarations whose bodies are skipped by normal
file execution. Discover testable source files for `ocelot test`, and accept one
optional comma-separated filter expression. Select tests when any nonempty part
is a substring of their name or defining file path. A filter with no matches is an error.

## Why was this chosen?

Name and path matching support both behavior-oriented and file-oriented test
selection with one small interface. Preserving test metadata lets discovery and
execution remain separate, and avoids requiring users to enumerate file arguments.

## What is the context?

The completed test-filter plan specifies substring OR matching and rejection
of multiple CLI arguments. Current summaries preserve defining paths for passed
and failed tests; parse and compilation failures remain visible during discovery.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [CLI filtering and tests](<../../crates/cli/src/main.rs>)
- [Discovered-test identity](<../../crates/engine/src/discovered_test.rs>)
- [Declaration and execution rules](<../spec/15.01 Declarations - Test items.md>)
- [Filter-expression plan](<../plans/completed/2026-04-07-test-cli-filter-expression-plan.md>)

## What are the consequences?

- Empty filter parts are ignored; matching is neither regex nor glob syntax.
- Files with matching paths contribute all their tests.
- Discovery errors can surface for files whose tests do not match the requested name.
- Test declarations do not implicitly turn normal script execution into a test run.

## What alternatives were considered?

- **Only accept file paths:** Cannot express test-name selection naturally.
- **Filter only files:** Misses matching names inside unrelated filenames.
- **Add regex or glob syntax now:** Adds parsing and escaping rules beyond the
  specified substring behavior.
- **Silently succeed with no matching tests:** Can hide a misspelled filter.
