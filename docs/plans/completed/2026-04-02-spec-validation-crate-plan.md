# Why do we need a spec validation crate?

`docs/spec` already uses a Markdown structure that was explicitly chosen so examples can become executable conformance cases.
Right now that structure is only a convention.
Without an automated validator, spec examples will drift, successful examples can silently stop working, and failure examples can become fiction.

This plan defines a dedicated Rust crate that extracts examples from spec chapters, runs them through `Engine::run_script` so the full engine pipeline is exercised, and reports mismatches in a way that is readable in both tests and a dedicated validation runner.

# What should the first slice of the crate do?

The first slice should create a new workspace crate at `crates/spec_validation`.
That crate should expose a library API plus a small validation runner binary.

The library should:

- discovers `docs/spec/*.md` in chapter order
- extracts executable examples from the documented Markdown shape
- runs each example as a single-file script through the current parser and interpreter pipeline
- captures whether the example succeeded or failed
- compares the observed outcome to the expected `### Output` text block
- returns a structured report covering passed, failed, and malformed examples

The runner should provide a stable command entrypoint that can be called from `.nao/nao.kdl`.
The first slice should optimize for stable, reviewable validation results rather than for a large general-purpose command surface.

# What Markdown contract should the crate enforce?

The validator should treat the current spec example shape as a strict contract rather than a loose heuristic.
Each executable example should include:

- a visible `## Example: ...` heading
- exactly one fenced `ocelot` block directly associated with that example
- exactly one `### Output` section with a fenced `text` block

The crate should report malformed examples separately from runtime mismatches.
That distinction matters because a broken Markdown structure is a documentation authoring error, not a language behavior regression.

The validator should initially ignore non-example prose and any future headings that do not match the executable example contract.

# How should examples be extracted from Markdown?

The extraction layer should parse Markdown into a small domain model instead of scraping with brittle line-based regexes.
That model should make the following concepts explicit:

- chapter metadata such as path and display name
- example name
- source code
- expected output
- source location information useful for diagnostics

`pulldown-cmark` is a reasonable fit for the first implementation because it keeps the parser dependency small while still giving enough structure to validate heading and fenced-code relationships.

The extractor should fail loudly when an example is ambiguous.
“Best effort” parsing would make the spec look valid while quietly skipping cases, which is exactly the kind of nonsense this crate is supposed to prevent.

# How should examples be executed?

The validator should execute examples through `ocelot_engine::engine::Engine::run_script`.
That is the right seam because it exercises the repository’s real script-loading pipeline instead of reimplementing parsing and interpretation steps inside the validator.

The execution flow should be:

1. materialize each extracted example as a synthetic `.ocelot` file path
2. provide that file to an `Engine` instance through the PAL boundary
3. call `Engine::run_script` with the example path
4. capture printed output and failure information through the PAL-backed execution environment

Tests should use `PalMock` so they can inject files and capture output deterministically while still going through `Engine::run_script`.
The validation runner can use either a small capturing PAL wrapper around `PalReal` or a similarly direct approach that still calls `Engine::run_script` as the execution entrypoint.

The crate should introduce a small execution result model that normalizes both success and failure into stable text intended for spec comparison.

# What output should the validator compare?

The validator should compare the spec’s `### Output` block against a normalized observed output string.
For the first slice, that normalized string should be:

- captured standard output for successful execution
- a stable diagnostic summary for expected failures

The validator should not compare against the current CLI renderer.
That renderer includes headline text, source locations, and tracing details that are useful for humans in a terminal but too unstable for spec fixtures.

Instead, the crate should define its own comparison-oriented rendering rules for failures.
The first version can use the root error message plus selected causal context as long as the formatting is deterministic and intentionally documented inside the crate.

# How should results be reported?

The crate should return a structured validation report that can drive both tests and future user-facing output.
That report should make it easy to answer:

- which chapters were scanned
- which examples passed
- which examples failed because observed output differed
- which examples were malformed and could not be executed

Each failure should include enough detail to debug quickly:

- chapter path
- example heading
- expected output
- actual output
- a short reason such as `output mismatch` or `invalid example shape`

A compact text renderer should be included in the crate and used by the validation runner so `.nao` can surface useful summaries without rebuilding report formatting elsewhere.

# What should the implementation order be?

The work should land in small slices so failures stay local and easy to review.

1. Add `crates/spec_validation` to the workspace with a minimal library surface, a validation runner binary, and colocated tests.
2. Mark `crates/cli` as the workspace default member so `cargo run` continues to target the `ocelot` CLI after the new runner exists.
3. Implement Markdown discovery and extraction for the current `## Example` plus `### Output` contract.
4. Add extraction tests using inline Markdown fixtures that cover valid and malformed chapter shapes.
5. Implement engine-backed execution through `Engine::run_script` using `PalMock` in tests and a real runner entrypoint for repository validation.
6. Add execution and comparison tests for both successful output examples and failing examples.
7. Add a report renderer that summarizes totals and prints per-example mismatches.
8. Add a `validate` task to `.nao/nao.kdl` that runs the validation runner.
9. Decide whether `check` should depend on `validate` immediately or whether that should land after the initial validator stabilizes.

This order keeps the execution path honest and avoids bolting automation on as an afterthought.

# What should be considered out of scope for the first slice?

The first slice should not attempt to solve every future conformance need.
It should explicitly leave these out of scope:

- multi-file examples
- module graphs or imports
- normalization rules for nondeterministic output
- snapshot updating workflows
- parallel execution
- a guarantee that every future spec heading style is executable

Those are real future concerns, but dragging them into the first slice would overengineer a very small harness.

# How should this work be verified?

Verification should focus on the crate as a reusable library and on the current spec chapters as real fixtures.

The work should include:

- colocated tests for Markdown extraction and malformed-example diagnostics
- colocated tests for execution result normalization
- at least one integration-style test that scans the real `docs/spec` directory and validates the current chapters
- verification that result reporting includes chapter paths and example headings for failures
- verification that the validation runner exits non-zero on mismatches
- verification that `.nao/nao.kdl` exposes a `validate` task that runs the runner
- running `nao check`

The real-spec test matters because otherwise the crate can pass while the repository’s actual documentation quietly diverges from the assumed format.

# What assumptions and open questions should stay explicit?

- The current spec contract uses `## Example` and `### Output`; if documentation authors want additional executable section types later, the extractor contract will need to expand deliberately rather than implicitly.
- Failure expectations are currently written under `### Output`; that is acceptable for the first slice, but a later revision may want headings such as `### Error` once the diagnostic model becomes richer.
- The current pipeline only covers a tiny language subset, but the validator should still go through `Engine::run_script` so spec execution tracks the real engine entrypoint instead of a parallel harness pipeline.
- The validator crate should own stable comparison formatting for failures so spec fixtures do not become coupled to transient CLI presentation details.
- If spec chapters ever need per-example metadata such as “skip”, “todo”, or “requires multi-file setup”, that metadata should be added as an explicit Markdown convention rather than hidden in prose.
- Adding a validation runner introduces another runnable package, so the workspace should explicitly keep `crates/cli` as the default package for root-level `cargo run`.

# What concrete tasks should track this plan?

- [x] Add `crates/spec_validation` to the workspace.
- [x] Add a validation runner binary for the spec validator crate.
- [x] Mark `crates/cli` as the workspace default member so `cargo run` keeps launching `ocelot`.
- [x] Define a small domain model for spec chapters, extracted examples, execution outcomes, and validation reports.
- [x] Implement Markdown chapter discovery in numeric filename order.
- [x] Implement strict extraction for `## Example`, fenced `ocelot`, and `### Output` `text` blocks.
- [x] Report malformed examples with source locations and actionable messages.
- [x] Execute extracted examples through `Engine::run_script`.
- [x] Use `PalMock` in tests to inject example files and capture output while still exercising the engine pipeline.
- [x] Normalize successful output and failure diagnostics into stable comparison text.
- [x] Implement report rendering for summary output and per-example mismatch details.
- [x] Add colocated data-driven tests for extraction, execution, and report rendering.
- [x] Add a test that validates the real `docs/spec` chapters.
- [x] Add a `validate` task to `.nao/nao.kdl` that runs the validation runner.
- [x] Decide whether `check` should depend on `validate` in the same slice or in follow-up work.
- [x] Run `nao check`.
