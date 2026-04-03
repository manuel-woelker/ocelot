# Why is another parser plan needed if the parser already accepts `&mut CompilationContext`?

The parser already takes a shared [`CompilationContext`](/data/projects/ocelot/crates/base/src/compilation_context.rs) in [`Parser::new()`](/data/projects/ocelot/crates/parser/src/parser.rs) and [`parse_script()`](/data/projects/ocelot/crates/parser/src/parse_script.rs).
That part of the earlier lexer integration is already done.

The real gap is that the parser only uses the shared context to observe lexer diagnostics and stop early.
Parser-originated failures still go through `ocelot_base::bail!()` and return plain `OcelotError`s instead of contributing `SourceDiagnostic`s to the shared compilation context.

As a result:

- lexer errors are structured diagnostics
- parser errors are still plain Rust-side error strings
- the engine still routes rendered diagnostics through `OcelotError`, and the current fallback path is not yet cleanly separated from ordinary internal failures

That is inconsistent and is the main problem this plan should address.

# What behavior should the parser move toward?

The parser should use the shared `CompilationContext` as its primary channel for user-facing parse failures.

The intended end state is:

- lexer appends diagnostics into `CompilationContext`
- parser appends diagnostics into the same `CompilationContext`
- parsing uses `ParseOutcome` internally and reaches a public non-artifact outcome when fatal diagnostics were added
- engine-level consumers can distinguish expected compilation failures from exceptional internal failures

This keeps the pipeline model consistent across stages and removes the current split-brain behavior where some errors are structured and others are not.

# What is the current status of parser integration?

Today:

- [`Parser`](/data/projects/ocelot/crates/parser/src/parser.rs) stores `&mut CompilationContext`
- [`Parser::parse_script()`](/data/projects/ocelot/crates/parser/src/parser.rs) returns `Ok(None)` when lexer errors are already present
- parser syntax and semantic checks such as missing test names, unexpected expressions, and invalid `println` argument counts still use `bail!()`
- [`Engine::parse_script()`](/data/projects/ocelot/crates/engine/src/engine.rs) renders diagnostics only for the lexer-error case, then wraps the rendered text in `OcelotError::message(...)`
- [`OcelotError`](/data/projects/ocelot/crates/base/src/error.rs) now has `ErrorKind::CompilationError(CompilationStage)` as a bridge for marking expected compilation-stage failures, but the parser and engine do not use it yet

That means the parser has the right plumbing but not the right error production path yet.

# What parser failures should become `SourceDiagnostic`s first?

The first slice should focus on failures that are already local and easy to pinpoint with spans.

Recommended first set:

- expected test name string
- expected `{` after test name
- expected `}` after test body
- expected statement
- expected `println` statement
- expected `(` after `println`
- expected `)` after argument
- expected `;` after statement
- expected expression
- unexpected token
- `println` zero-argument type error

These are enough to establish the pattern and replace the most visible plain parser errors.

# How should parser diagnostics be represented?

Parser diagnostics should use the same base types as lexer diagnostics:

- `SourceDiagnostic`
- `SourceExcerpt`
- `SourceAnnotation`
- `DiagnosticLevel::Error`

Each parser diagnostic should include:

- the source file path
- a stable summary message
- one excerpt for the relevant source line
- one primary annotation over the best available token or fallback span

The implementation should prefer precise token spans when available.
For end-of-file cases such as missing closing braces, the parser may need a small fallback span convention so diagnostics still point somewhere sensible.

# What API changes should happen in the parser?

The parser already accepts `&mut CompilationContext`, so the main API change is behavioral rather than structural.

Recommended direction:

- replace parser `bail!()` paths with helpers that append a diagnostic to `self.compilation_context`
- use a small internal `ParseOutcome` type for fatal-diagnostic control flow instead of raising `OcelotError`
- keep `parse_script(source_file, &mut context)` as the public entrypoint unless a cleaner result type becomes necessary

The parser should continue using `OcelotResult` only for unexpected infrastructure failures, not ordinary user-facing syntax problems.

A good shape is:

```rust
enum ParseOutcome<T> {
    Parsed(T),
    FatalDiagnostic,
}
```

or an equivalent internal result alias built on the same idea.
This keeps `?` ergonomics available inside the parser without collapsing parser diagnostics back into the generic error channel.

# How should the engine change once parser diagnostics exist?

The current engine fallback is the root cause of Rust source locations leaking into diagnostics.

Once parser-generated diagnostics use `CompilationContext`, the engine should:

- stop treating parser-originated failures like ordinary message errors
- use `CompilationError(CompilationStage::Parser)` as the short-term bridge if the diagnostics still need to cross an `OcelotError` boundary
- move toward a path where rendered source diagnostics are final user-facing output rather than nested inside generic error formatting
- keep `OcelotError` for true internal or platform failures

The repository now has the first bridge piece in place through `CompilationError(CompilationStage)`.
That makes the next step smaller: parser and engine code can begin tagging expected compilation failures without having to solve the full final rendering architecture in the same change.

# What implementation approach keeps this manageable?

The safest path is to add parser-diagnostic helpers before converting call sites.

Useful helper shape:

- a method that builds a `SourceDiagnostic` from a message and a span
- a method that appends that diagnostic to `self.compilation_context`
- a method that computes line/annotation data from a span in the current `SourceFile`

Once those helpers exist, each `bail!()` site can be converted one by one.
That keeps the change mechanical and testable instead of turning into a broad parser rewrite.

# What open questions should be clarified before implementation?

- How much of `ParseOutcome` should stay internal to the parser implementation, and should the public parser API remain `OcelotResult<Option<T>>` for now or expose a similar outcome type later?
- Should the engine use `CompilationError(CompilationStage::Parser)` as a temporary boundary type for parser failures, or should it jump straight to a dedicated diagnostics result path?
- Should `TokenType::Unexpected` remain a parser-level diagnostic for now, or should it move entirely into the lexer so the parser never sees it as a primary error source?

The plan below assumes the repository can introduce `ParseOutcome` internally first, keep the existing public parser signature for now, and improve the external API later only if it still feels justified.

# What assumptions and risks should stay explicit?

- This plan assumes parser errors are fatal enough that returning `Ok(None)` after appending diagnostics is acceptable for the current language maturity.
- This plan assumes one or a small number of parser diagnostics per file is acceptable; full recovery is out of scope for this slice.
- If the engine continues routing rendered diagnostics through generic `OcelotError::message(...)`, user-facing output will still include Rust implementation details even after parser diagnostics are added.
- `CompilationError(CompilationStage)` is a bridge, not the final diagnostics transport. Overusing it would blur the separation between diagnostics in `CompilationContext` and exceptional failures in `OcelotError`.
- Converting parser errors without a small helper layer first would be easy to botch and would likely duplicate excerpt-building logic.

# What implementation order is recommended?

1. Add parser-local helper functions for building and appending `SourceDiagnostic`s from spans.
2. Introduce an internal `ParseOutcome` type for fatal-diagnostic control flow.
3. Convert the highest-value parser `bail!()` sites to diagnostics appended into `CompilationContext`.
4. Change parser control flow so fatal parse diagnostics use `ParseOutcome` internally and surface as a non-artifact outcome at the public boundary instead of plain `OcelotError`.
5. Update parser tests to assert on diagnostics in the shared context rather than string error messages.
6. Update engine parse-failure handling to distinguish parser compilation failures from ordinary message errors, using `CompilationError(CompilationStage::Parser)` as the bridge if needed.
7. Add or update integration tests covering parser-originated diagnostics.
8. Run `nao check`.

# What verification should this work include?

Verification should include:

- colocated parser tests for each converted parser failure
- assertions on diagnostic message, file path, excerpt text, and annotation span
- at least one engine-level or integration-style test showing parser failures are tagged as parser compilation failures and do not regress diagnostics rendering behavior
- running `nao check`

# What concrete tasks should track this plan?

- [x] Add parser helper code to build `SourceDiagnostic`s from source spans.
- [x] Introduce an internal `ParseOutcome` type for parser diagnostic control flow.
- [x] Convert parser syntax/type-error `bail!()` sites to shared-context diagnostics.
- [x] Change parser control flow so parser diagnostics use `ParseOutcome` internally and reach a non-artifact public outcome instead of plain `OcelotError`.
- [x] Update parser tests to assert on structured diagnostics.
- [x] Update engine parse-failure handling to distinguish parser compilation failures, using `CompilationError(CompilationStage::Parser)` as the short-term bridge if needed.
- [x] Add integration coverage for parser-generated diagnostics.
- [x] Run `nao check`.
