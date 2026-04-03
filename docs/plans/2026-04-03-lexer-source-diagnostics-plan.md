# Why does the lexer need to produce `SourceDiagnostics`?

The current lexer returns `OcelotResult<impl Iterator<Item = Token>>` and aborts on the first hard lexing failure.
That is enough for simple happy-path parsing, but it throws away the repository's new diagnostics infrastructure right where it would be most useful.

`unterminated string literal` is the clearest example.
Right now the parser caller only gets a plain error message.
It does not get the file path, the offending span, or a source excerpt that can be rendered consistently with other diagnostics.

The lexer should therefore become the first stage that emits structured `SourceDiagnostic` values rather than only stringly `OcelotError` failures.

# What is the current integration point?

The current flow is:

1. `parse_script()` calls `Parser::new(source_file)`.
2. `Parser::new()` calls `lex(source_file.source())?.collect()`.
3. `lex()` either returns tokens or bails immediately.

That shape has two important consequences:

- the lexer has no access to the `SourceFile` path, so it cannot currently construct a fully useful `SourceDiagnostic`
- the parser assumes lexing either fully succeeds or fully fails, so there is no place to carry diagnostics forward

Any real integration should fix both of those issues directly rather than trying to translate a plain error string into diagnostics after the fact.

# What API shape should lexing move to?

The lexer should stop taking `&str` alone and should instead lex a full `&SourceFile`.
That gives it the file path and source text it needs to build proper diagnostics.

Because the intended direction is to thread one `CompilationContext` through multiple compiler stages, the lexer should accept a mutable compilation context rather than return its own diagnostics container.

The recommended signature is:

```rust
pub fn lex(source_file: &SourceFile, context: &mut CompilationContext) -> Vec<Token>
```

That keeps diagnostics accumulation consistent with the likely future direction for parsing, validation, and later compilation passes:

- each stage returns its primary artifact directly
- each stage appends diagnostics into the shared `CompilationContext`
- the caller decides whether to continue based on `context.has_errors()`

# Why should this not stay as `Result<Vec<Token>, OcelotError>` plus extra helpers?

That approach would be a dead end.
It would keep diagnostics outside the normal control flow and force every caller to choose between:

- aborting on the first lexer problem
- translating `OcelotError` into `SourceDiagnostic` later with less source context
- maintaining two competing error channels

That is overengineered in the wrong direction.
If the goal is structured diagnostics across a multi-stage pipeline, the lexer should write structured diagnostics into the shared compilation context directly.

# How should lexer errors be represented?

For the first slice, lexer diagnostics should be ordinary `SourceDiagnostic` entries with:

- `DiagnosticLevel::Error`
- the `SourceFile` path
- a stable message such as `unterminated string literal`
- one `SourceExcerpt` for the affected line
- one `SourceAnnotation` that highlights the bad span and repeats or sharpens the message

For an unterminated string, the highlighted span should start at the opening quote and end at the end of the file or line, depending on the final scanning rule.
The first implementation should keep this simple and use the consumed byte range from the opening quote to the current index.

# How should the lexer behave after producing a diagnostic?

The first implementation should keep recovery conservative.

Recommended behavior:

- emit a diagnostic for the unterminated string
- stop lexing further tokens
- append `EndOfFile`
- return the collected tokens after mutating the shared `CompilationContext`

This keeps the lexer deterministic and easy to reason about.
It also gives the parser a stable token stream boundary without pretending that recovery is more robust than it really is.

Trying to continue scanning past an unterminated string now would likely produce garbage tokens and noisy follow-up failures.
That would be more annoying than helpful.

# How should the parser consume lexer diagnostics?

The parser should treat lexer diagnostics as fatal for AST construction, but not as plain unstructured errors.

The recommended integration is:

1. `Parser::new(source_file, context)` calls the new lexer entrypoint and stores the returned tokens.
2. If `compilation_context.has_errors()` is true immediately after lexing, parsing should stop before AST construction.
3. The parser API should then return a diagnostic-aware failure shape rather than discarding the diagnostics into a generic `OcelotError`.

There are two viable ways to model that parser boundary.

## Option: make parser entrypoints return diagnostics-aware results

Example direction:

```rust
pub struct ParseScriptResult {
    pub script: Option<Script>,
}
```

This is the cleanest long-term model because it allows:

- lexer errors without panicking the parser
- future parser diagnostics in the same shared container
- one consistent compilation pipeline shape based on an external `CompilationContext`

This option is recommended.

## Option: keep `OcelotResult<Script>` and attach diagnostics elsewhere

This would preserve more existing signatures short term, but it keeps the main API dishonest.
Callers would still not receive the structured diagnostics through the function that actually discovered them.

This option should be avoided unless the repository needs a very small temporary bridge commit.

# What implementation path keeps the blast radius reasonable?

The cleanest path is to split the work into two small stages.

## Stage 1: introduce diagnostics-capable lexer integration

- change `lex()` to accept `&SourceFile` and `&mut CompilationContext`
- return tokens directly
- add lexer-only tests for unterminated string diagnostics
- keep parser changes minimal but enough to compile

At the end of this stage, lexer diagnostics exist and are testable, and the repository has established the shared-context pattern for later phases.

## Stage 2: make parser entrypoints diagnostics-aware

- change `Parser::new()` to accept and retain access to the shared compilation context
- change `parse_script()` and related parser APIs so they no longer need to return diagnostics containers
- stop parsing when lexer diagnostics already contain errors
- migrate parser tests from string-error assertions to diagnostic assertions where appropriate

This stage finishes the integration properly instead of leaving diagnostics stranded in the lexer.

# What helper code will the lexer need?

The lexer will need a small amount of source-location support to build excerpts cleanly.
That should stay local and simple.

Useful helpers:

- a function that maps a byte index to the containing line number and line text
- a function that builds a `SourceExcerpt` and `SourceAnnotation` from a span and message

If these helpers become useful beyond the lexer, they can later move into `ocelot_base`.
They should not be extracted preemptively.

# How should tests change?

Verification should focus on observable behavior, not internal plumbing.

Lexer tests should cover:

- successful lexing still returns the expected token sequence
- unterminated strings produce one error diagnostic
- the diagnostic message is stable
- the diagnostic points at the correct file path and span
- the diagnostic includes the expected source line excerpt

Parser tests should cover:

- parser entrypoints surface lexer diagnostics instead of plain `OcelotError` text
- parsing does not proceed into misleading parser errors when lexing already failed
- valid sources still parse successfully with an empty-error compilation context

# What risks and tradeoffs should stay explicit?

- If parser APIs stay `OcelotResult`-based too long, the repository will accumulate awkward conversion glue from diagnostics back into plain errors.
- Conservative lexer recovery means only one lexer error may be reported per file in the first slice.
  That is acceptable for now and much better than losing source context entirely.
- Source excerpts depend on byte spans matching UTF-8 boundaries.
  The current lexer already scans ASCII-oriented syntax byte-by-byte, so this is acceptable, but string and identifier rules may need revisiting later if Unicode source syntax expands.
- `TokenType::Unexpected` already acts like a low-fidelity lexer error marker.
  The repository should decide whether unexpected characters also become diagnostics in this same slice or remain parser-reported follow-up errors temporarily.

# What implementation order is recommended?

1. Change the lexer entrypoint to accept `&SourceFile` and `&mut CompilationContext`.
2. Emit a `SourceDiagnostic` for unterminated strings with excerpt and annotation data.
3. Update lexer tests to assert on diagnostics instead of plain error strings.
4. Thread the shared `CompilationContext` into `Parser::new()`.
5. Change parser entrypoints so diagnostics live in the shared context instead of return values.
6. Stop parsing immediately when lexer diagnostics already contain errors.
7. Add parser tests that prove lexer diagnostics surface cleanly through the parser API.
8. Run `nao check`.

# What concrete tasks should track this plan?

- [ ] Change `lex()` to accept `&SourceFile` and `&mut CompilationContext` instead of `&str`.
- [ ] Implement unterminated-string `SourceDiagnostic` emission in the lexer.
- [ ] Add colocated lexer tests for diagnostic contents, spans, and excerpts.
- [ ] Thread the shared `CompilationContext` into `Parser::new()`.
- [ ] Update parser-facing APIs so diagnostics live in the shared compilation context instead of plain `OcelotError` strings.
- [ ] Add parser tests proving lexer failures surface as structured diagnostics.
- [ ] Run `nao check`.
