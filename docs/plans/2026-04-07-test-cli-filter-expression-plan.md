# Why should `ocelot test` accept a filter expression instead of file paths?

The current CLI treats every argument after `ocelot test` as a source path.
That is too narrow for the requested workflow because it forces users to think in terms of files even when they want to target tests by name.

This slice should change `ocelot test` so it accepts at most one optional filter expression.
The filter expression is a comma-separated string.
Each filter part should match a discovered test when it matches either:

- the test name
- the file path that defined the test

This keeps the CLI simple while still supporting both common intents:

- "run the tests named like this"
- "run the tests from files like this"

# What behavior should the CLI commit to?

Recommended CLI behavior:

- `ocelot test` with no extra argument still discovers all `.ocelot` and `.ocelot-script` files and runs all discovered tests
- `ocelot test <filter-expression>` treats the single extra argument as one filter expression, not as one or more file paths
- the filter expression is split on commas
- empty filter parts created by leading, trailing, or repeated commas are ignored
- a test is selected when any non-empty filter part matches either the test name or the file path that contains the test
- if filtering removes every discovered test, the CLI exits with an error explaining that no tests matched the filter expression

Recommended matching rule:

- treat each filter part as a simple substring match against test name and source file path

This is the simplest behavior that fits the request and keeps the implementation honest.
If exact matching, globbing, or regex support becomes desirable later, that should be a separate change instead of sneaking complexity into this slice.

# What gaps in the current implementation need to close?

Today [`crates/cli/src/main.rs`](/data/projects/ocelot/crates/cli/src/main.rs) parses `ocelot test` into `CliCommand::Test { script_paths: Vec<String> }` and either:

- walks the repository for all testable source files when no paths are provided, or
- treats every argument as an explicit file path to execute

That model no longer fits once the argument becomes a filter expression.

There is also a data-shape gap in the engine summary:

- [`crates/engine/src/test_run_summary.rs`](/data/projects/ocelot/crates/engine/src/test_run_summary.rs) records passed test names only
- [`crates/engine/src/failed_test_result.rs`](/data/projects/ocelot/crates/engine/src/failed_test_result.rs) records failure name and message, but the CLI-level summary logic does not currently preserve source file information for successful tests

Because filename matching is now part of the public CLI contract, the implementation needs enough metadata to evaluate a filter against both test name and defining file path before execution.

# Where should filter matching live?

The recommended design is to keep path discovery in the CLI layer and move test-level matching into the engine-facing test summary model.

Recommended approach:

- keep CLI file discovery as it is today so `ocelot test` still walks all relevant source files
- have the engine expose discovered tests with both test name and source file path
- apply the optional filter at the test-item level rather than the file level
- execute only the matching tests from each file
- preserve the current behavior that parse or resolver failures for a file still surface as failures tied to that file

Filtering at the file level would be the wrong abstraction.
It would make filename filtering easy, but it would either miss name-based matches in otherwise unrelated files or require a second discovery mechanism anyway.

# How should the engine API change to support this cleanly?

The current engine API distinguishes between running one named test and running all tests in one file.
This slice likely needs a middle shape: running a selected subset of tests from one file after discovery.

Recommended engine changes:

- extend discovered test metadata so each discovered test carries its source file path in addition to its name
- reshape [`TestRunSummary`](/data/projects/ocelot/crates/engine/src/test_run_summary.rs) so successful results preserve enough metadata for CLI filtering and reporting
- add an engine path that can:
  - discover tests in the entry file
  - filter them by predicate or explicit names
  - execute only the selected tests
- keep existing single-test and all-tests helpers if they remain useful, but implement them in terms of the shared discovery-and-run path when practical

This keeps the behavior centralized instead of splitting discovery logic between the CLI and the engine worker.

# What implementation order keeps the change small?

1. Update CLI parsing in [`crates/cli/src/main.rs`](/data/projects/ocelot/crates/cli/src/main.rs) so `test` stores one optional filter expression instead of a list of paths.
2. Add a small parser/helper for comma-separated filter expressions, including trimming and ignoring empty parts.
3. Extend engine-side test metadata so discovered or reported tests include both test name and source file path.
4. Add an engine execution path for "run matching tests from this file" without reintroducing file-path arguments at the CLI boundary.
5. Update CLI test execution to:
   - discover all eligible source files
   - collect and run only tests matching the optional filter
   - fail with a clear error when no tests match
6. Update usage text and any user-facing diagnostics so the command shape is explicit.
7. Add or update colocated tests in the CLI and engine crates.
8. Run `nao check`.

# What verification should this work include?

Verification should include colocated tests for:

- parsing `ocelot test` with no extra argument
- parsing `ocelot test smoke` as one optional filter expression
- rejecting extra positional arguments after the optional filter expression if the CLI is intended to accept only one argument
- splitting comma-separated filter expressions and ignoring empty parts
- matching by test name
- matching by source file path
- matching when any filter part matches
- running all tests when no filter is provided
- running only matching tests when a filter is provided
- returning a failing exit code and stable error text when no tests match the filter expression
- preserving parse or resolver failure reporting for files that still participate in the run
- rendering summary output correctly when a filtered run includes both passing and failing tests
- running `nao check`

# What assumptions and open questions should stay explicit?

- Assumption: each filter part should use substring matching, not exact matching. The request says tests should be compared to the parts of the filter, and substring matching is the smallest user-friendly interpretation.
- Assumption: filter matching should be case-sensitive unless there is a broader CLI convention that says otherwise. Introducing case folding now would make the rules less obvious.
- Assumption: an empty filter expression such as `ocelot test ""` should behave the same as "no tests matched" rather than silently running everything.
- Open question: should the "no tests matched" error mention the original filter expression verbatim, for example `no tests matched filter expression 'foo,bar'`? That would be more useful than a generic error and is the recommended behavior.
- Open question: should files that contain no matching tests be skipped silently even if they parse successfully? That is the recommended behavior because the filter is test-oriented, not file-oriented.

# What concrete tasks should track this plan?

- [ ] Replace `CliCommand::Test { script_paths }` with a shape that stores one optional filter expression.
- [ ] Update `ocelot test` usage text so it documents `[filter-expression]` instead of `[source-file...]`.
- [ ] Add a helper that parses comma-separated filter parts, trims whitespace, and ignores empty entries.
- [ ] Extend engine test metadata so a discovered or reported test includes both its name and source file path.
- [ ] Add an engine path for running only the tests from one file that match a provided filter.
- [ ] Update CLI test execution to discover testable files once and apply the optional filter across discovered tests by name or filename.
- [ ] Return a CLI failure when no tests match the filter expression.
- [ ] Add CLI tests for parsing, matching, filtered execution, and the no-match error path.
- [ ] Add engine tests for filtered test discovery and execution behavior.
- [ ] Run `nao check`.
