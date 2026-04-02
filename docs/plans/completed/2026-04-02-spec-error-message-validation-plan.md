# Why do we need a dedicated plan for spec validation of error messages?

The spec validator already executes failing examples and compares their observed failure text against the chapter’s expected `### Output` block.
That is enough for rough conformance today, but it is too underspecified for diagnostics work.

Right now the repository has no clear contract for when a spec example is asserting:

- normal program output
- a stable error message
- a richer rendered diagnostic shape with source excerpts

Without a deliberate plan, diagnostic validation will drift into a mess of ad hoc strings.
That would make the spec fragile, make error-message changes hard to review, and blur the boundary between user-facing diagnostics and comparison-oriented validation output.

# What problem should this plan solve first?

The first follow-up slice should make spec-level validation of error messages explicit.
The validator should stop treating all expected text as a generic `### Output` block when the example is actually documenting a failure case.

The plan should answer two questions cleanly:

1. how should spec chapters declare that an example expects an error
2. what exact text should the validator compare for that error

# What should the spec authoring contract become?

The recommended contract is to introduce an explicit `### Error` section for examples that are expected to fail.

That means an executable example should use exactly one of:

- `### Output` with a fenced `text` block for successful execution
- `### Error` with a fenced `text` block for expected failure text

This is a better contract than overloading `### Output` for both cases.
It makes intent obvious to readers, lets the validator distinguish “program printed text” from “program failed with a diagnostic,” and avoids a future pile of special cases.

# Why prefer `### Error` over keeping everything under `### Output`?

Keeping failures under `### Output` was acceptable for the first validator slice because the language barely had any diagnostics.
It becomes a bad fit once the spec starts caring about actual error-message behavior.

`### Error` is the better choice because:

- it is clearer for readers skimming the spec
- it gives the extractor an explicit execution expectation
- it lets reporting use terms like “error mismatch” instead of pretending failures are output
- it leaves room for later headings such as `### Warning` or `### Diagnostic` without redefining old meaning

The current `### Output` failure examples should be migrated rather than preserved as a permanent synonym.

# What text should the validator compare for an expected error?

The validator should compare a stable error-message rendering owned by the validator crate, not the full CLI error presentation.

For the next slice, the comparison text should be:

- the root error message on the first line
- followed by `caused by: ...` lines for nested causes when present

That is very close to the current `render_validation_error` behavior and should stay intentionally narrow.
The validator should not yet compare source locations, terminal headlines, or ANSI styling.

This keeps fixtures reviewable while still validating the actual wording that users care about.

# How should this relate to richer source diagnostics?

The repository now has a structured source-diagnostic model and a renderer in `ocelot-base`.
That does not mean the spec validator should immediately compare full rendered diagnostics.

The recommended sequencing is:

- first, make error-message expectations explicit with `### Error`
- second, keep comparison text limited to stable error message content
- only later decide whether the spec should validate full rendered diagnostics, source excerpts, or annotation spans

Jumping straight to full rendered diagnostic validation would be overkill right now.
It would make fixtures noisy and tightly couple the spec to presentation details before the diagnostic model is settled.

# What data model changes should the validator make?

The validator’s extracted example model should stop storing only `expected_output`.
Instead, it should record an explicit expected outcome kind.

A reasonable first step is:

```text
SpecExample
  expected_outcome: ExpectedOutcome

ExpectedOutcome
  Output(SharedString)
  Error(SharedString)
```

That keeps the model honest all the way through extraction, execution, comparison, and failure reporting.
If the extractor continues flattening everything to one generic string, the `### Error` syntax will not buy much.

# How should Markdown extraction change?

The extractor should treat `### Output` and `### Error` as mutually exclusive expectation sections for one example.

The new rules should be:

- an example must contain exactly one fenced `ocelot` block
- an example must contain exactly one expectation section
- that expectation section must be either `### Output` or `### Error`
- the section must contain exactly one fenced `text` block
- examples with both headings should be reported as malformed
- examples with neither heading should be reported as malformed

This is strict on purpose.
Loose extraction would make the spec look valid while silently accepting ambiguous author intent.

# How should execution and comparison change?

The execution layer can stay simple.
It should still run the example through the existing engine entrypoint and normalize the observed result.

The new behavior should be:

- if the example expects `Output`, compare against captured stdout on success
- if the example expects `Error`, compare against the normalized failure rendering on error
- if execution succeeds for an example that expected `Error`, report that as a validation failure
- if execution fails for an example that expected `Output`, report that as a validation failure

That distinction is important because it turns “wrong execution mode” into a first-class mismatch instead of hiding it inside a generic text diff.

# How should validation failures be reported?

Failure reporting should distinguish at least these categories:

- malformed example
- output mismatch
- error mismatch
- expected error but execution succeeded
- expected output but execution failed

The current single `OutputMismatch` category is too vague once expectations become typed.
Reviewing validator output should make it obvious whether the problem is wording, structure, or the wrong success/failure outcome.

# What spec changes should be included in the same slice?

The first implementation slice should migrate the existing failure examples in `docs/spec` from `### Output` to `### Error`.

That should include:

- current wrong-arity `println()` examples
- any other failure examples added while this work is in flight
- an update to `docs/spec/README.md` so the authoring contract documents both headings explicitly

If the code changes but the current spec chapters are left on the old contract, the validator will end up carrying compatibility baggage immediately.

# What should remain out of scope for this slice?

This slice should not attempt to validate:

- full CLI error presentation
- ANSI styling
- source locations
- multi-snippet rendered diagnostics
- warnings emitted during successful execution
- diagnostic IDs, help text, or suggestions

Those are valid future directions, but they should land after the repository has real diagnostic producers that justify the extra spec surface.

# What implementation order makes sense?

The work should land in a few small steps:

1. Extend the extracted example model with an explicit expected-outcome enum.
2. Update Markdown extraction to accept `### Output` or `### Error` and reject ambiguous combinations.
3. Expand validation failure kinds so mismatches distinguish output, error, and wrong execution outcome.
4. Keep using the validator-owned stable error renderer for error comparisons.
5. Update report rendering to name error-specific mismatches clearly.
6. Migrate existing failing spec examples to `### Error`.
7. Update `docs/spec/README.md` to document the new contract.
8. Add or update tests for extraction, execution, mismatch reporting, and the real `docs/spec` directory.

This order keeps the data model and extractor ahead of the fixture migration, which is the right direction.

# How should this work be verified?

Verification should include:

- colocated extraction tests for valid `### Error` examples
- colocated extraction tests for malformed examples that use both `### Output` and `### Error`
- execution tests proving expected-error examples compare against normalized failure text
- validation tests proving success-for-error and failure-for-output cases are reported distinctly
- report-rendering tests covering the new failure categories
- a real-spec validation test that exercises migrated `docs/spec` chapters
- running `nao check`

The real-spec test matters here because this change is mostly about tightening the contract between documentation and tooling.

# What assumptions and open questions should stay explicit?

- This plan assumes the validator should still compare a narrow, stable message rendering rather than full rendered `SourceDiagnostic` output.
- If the language later wants to specify richer diagnostics, that should likely use a deliberate heading such as `### Diagnostic` rather than overloading `### Error`.
- The validator may choose to temporarily accept legacy `### Output` failure examples during migration, but that compatibility should be treated as transitional and removed quickly.
- Error-message validation is only useful if the compared text is intentionally stable; if parser or type-checker wording is still in heavy flux, spec fixtures should stay focused and minimal.

# What concrete tasks should track this plan?

- [x] Add an explicit expected-outcome enum to extracted spec examples.
- [x] Update Markdown extraction to support `### Output` and `### Error` expectation sections.
- [x] Reject examples that define both expectation headings or neither heading.
- [x] Distinguish output mismatches, error mismatches, and wrong execution outcome failures.
- [x] Keep error comparisons based on validator-owned stable message rendering.
- [x] Update validation report rendering for the new failure categories.
- [x] Migrate existing failing spec examples in `docs/spec` from `### Output` to `### Error`.
- [x] Update `docs/spec/README.md` to document the explicit error-expectation contract.
- [x] Add or update colocated tests for extraction, execution, report rendering, and real-spec validation.
- [x] Run `nao check`.
