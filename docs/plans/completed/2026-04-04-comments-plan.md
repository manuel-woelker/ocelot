# Why does `ocelot` need explicit comment support now?

The language can currently skip whitespace, but it has no comment syntax.
That makes examples noisy, makes test files harder to explain inline, and leaves a basic ergonomic gap in the source language.

The first slice should support:

- line comments starting with `//` and continuing to the end of the line
- block comments delimited by `/*` and `*/`
- nested block comments, so `/* outer /* inner */ outer */` is valid

Comments should behave like trivia rather than syntax nodes.
They should be discarded by the lexer and should not appear in the AST.

# What behavior should the implementation commit to?

The implementation should treat comments exactly like whitespace between tokens.
That means comments may appear:

- between top-level items
- between tokens inside expressions and statements
- before or after braces, parentheses, commas, and semicolons
- adjacent to newlines without changing runtime behavior

The lexer should not treat comment markers inside string literals as comments.

For the first version, line comments should end at `\n`, `\r`, or end of file.
Block comments should be able to span multiple lines and should update nesting depth whenever another `/*` appears before the matching `*/`.

# What lexer changes are required?

This feature belongs almost entirely in [`crates/parser/src/lexer/lex.rs`](/data/projects/ocelot/crates/parser/src/lexer/lex.rs).
The current byte-oriented scanner already handles whitespace directly, so the cleanest change is to extend that trivia-skipping path instead of inventing a separate token kind for comments.

Recommended lexer work:

1. Factor the leading trivia handling into a helper that can skip whitespace, `// ...`, and `/* ... */`.
2. Detect `//` before falling through to `TokenType::Unexpected`.
3. Detect `/*` and scan until the matching `*/`, tracking nesting depth.
4. Preserve the current behavior that strings are scanned as strings first, so `\"//\"` and `\"/*\"` stay ordinary string contents.
5. Leave the token model unchanged unless implementation pressure proves otherwise.

Keeping comments out of [`crates/parser/src/lexer/token_type.rs`](/data/projects/ocelot/crates/parser/src/lexer/token_type.rs) is the right default.
Comments are not semantically meaningful tokens in the current parser, and promoting them to tokens would add parser noise for no benefit.

# What diagnostics should block comments produce when they are malformed?

Nested block comments imply one important failure mode: an unterminated block comment.
The lexer should report this as a structured lexer diagnostic, consistent with the existing unterminated-string path.

Recommended behavior:

- emit one `SourceDiagnostic` with a stable message such as `unterminated block comment`
- annotate from the opening `/*` through the end of file
- stop lexing after the diagnostic and append `EndOfFile`

This keeps error recovery aligned with the repository's current lexer strategy.
Trying to continue after an unterminated block comment would likely create misleading parser errors.

# What parser changes should be expected?

Parser changes should be minimal.
If comments are fully discarded by the lexer, [`crates/parser/src/parser.rs`](/data/projects/ocelot/crates/parser/src/parser.rs) and [`crates/parser/src/parse_script.rs`](/data/projects/ocelot/crates/parser/src/parse_script.rs) should only need verification coverage proving that comment-heavy inputs still parse into the same AST shape.

That is the right shape.
If comment support requires meaningful parser changes, the lexer design is probably wrong.

# How should the spec account for comments?

Comments need both a positive syntax chapter and an error chapter.
This work updates [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md) so lexical structure becomes chapter `01` and the existing chapter list matches the renumbered files.

The implemented documentation work is:

1. Update the top-level chapter numbering in [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md) so lexical structure becomes chapter `01` and the existing chapter list is renumbered to match.
2. Add a new spec chapter for comment syntax and behavior.
3. Add a new lexer-error chapter for unterminated block comments.

The numbering change is explicit rather than implicit.
Implemented top-level renumbering:

- `01`: Lexical structure
- `02`: Expressions
- `05`: Statements
- `10`: Types
- `15`: Declarations
- `20`: Functions
- `25`: Modules
- `27`: Effects
- `28`: Runtime behavior
- `30`: Standard library
- `91`: Lexer errors

This gives comments a correct home at the front of the spec and leaves room for future chapters without immediately renumbering `30` again.

That README update includes both:

- the numbered outline under "What top-level chapter numbers are planned?"
- the concrete chapter links under "What chapters exist today?"

Existing chapters are renamed to match the new scheme rather than leaving the README with stale file names and stale numbers.

The comment syntax chapter is `01.01 Lexical structure - Comments`.
The unterminated block comment diagnostic is `91.02 Lexer errors - Unterminated block comments`.

The syntax chapter should specify:

- `//` starts a line comment
- `/* ... */` starts a block comment
- block comments nest
- comments are ignored as whitespace and do not affect execution
- comment markers inside string literals are not comments

The error chapter should lock down the rendered diagnostic for unterminated block comments the same way [`docs/spec/91.01 Lexer errors - Unterminated strings.md`](/data/projects/ocelot/docs/spec/91.01%20Lexer%20errors%20-%20Unterminated%20strings.md) already does for strings.

# How should examples be updated?

The repository should not stop at spec prose.
It should also include concrete user-facing examples that demonstrate comments in ordinary source files.

Implemented example work:

- add a focused source example such as `examples/comments.ocelot`
- update at least one existing example file to include a small comment in realistic context if that improves readability
- include spec examples for line comments, nested block comments, and comments mixed with executable code
- include one failing spec example for an unterminated block comment

The dedicated example file should show both comment forms in a small script, not a contrived wall of trivia.

# What tests verified the feature?

Verification should stay black-box and colocated with the affected code.

Lexer tests in [`crates/parser/src/lexer/lex.rs`](/data/projects/ocelot/crates/parser/src/lexer/lex.rs) should cover:

- line comments before and after tokens
- block comments between tokens
- nested block comments
- block comments spanning multiple lines
- comment markers inside strings remaining ordinary string text
- unterminated block comments producing one lexer diagnostic with a stable message and excerpt

Parser tests in [`crates/parser/src/parse_script.rs`](/data/projects/ocelot/crates/parser/src/parse_script.rs) should cover:

- scripts parse the same with and without comments
- test items still parse when comments appear around names, braces, and statements
- lexer diagnostics from unterminated block comments surface through the shared compilation context

Spec validation now covers:

- successful comment examples
- the unterminated block comment diagnostic example

# What implementation order kept this small and honest?

1. Extend the lexer's trivia scanning to skip line comments and nested block comments.
2. Add unterminated-block-comment diagnostics in the lexer.
3. Add colocated lexer coverage for successful and failing comment inputs.
4. Add parser coverage proving comments do not change AST structure or test-item parsing.
5. Renumber the planned outline and existing chapter references in [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md).
6. Add spec chapters and examples for comments and unterminated block comments, renaming existing chapter files as needed to keep numbering consistent.
7. Add or update repository example files to show real comment usage.
8. Run `nao check`.

This order keeps the language behavior real before documenting it, while still making spec and examples part of the planned work rather than follow-up cleanup.

# What assumptions and follow-up notes should stay explicit?

- The current lexer is byte-oriented and ASCII-oriented. This is fine for `//`, `/*`, and `*/`, but span calculations for multiline diagnostics should still be checked carefully.
- The first version should treat comments strictly as discarded trivia. Source-preserving tooling can revisit comment retention later if the language grows formatting or documentation tooling.
- The repository now renumbers existing spec chapters to make room for `01` lexical structure and `30` standard library.
- The plan assumes nested block comments are part of the core language contract, not an implementation detail that can be relaxed later.

# What verification was completed?

Verification completed with:

- `cargo test -p ocelot-parser`
- `cargo test -p ocelot-spec-validation`
- `nao check`

# What concrete tasks should track this plan?

- [x] Extend the lexer trivia path to skip `//` line comments.
- [x] Extend the lexer trivia path to skip `/* ... */` block comments with nesting depth.
- [x] Report unterminated block comments as structured lexer diagnostics.
- [x] Add colocated lexer tests for line comments, block comments, nested block comments, and unterminated block comments.
- [x] Add parser tests proving comments do not affect script or test-item parsing.
- [x] Renumber the outline and existing chapter links in [`docs/spec/README.md`](/data/projects/ocelot/docs/spec/README.md).
- [x] Add a spec chapter for comment syntax and behavior.
- [x] Add a spec chapter for unterminated block comment diagnostics.
- [x] Rename existing spec chapter files as needed so their filenames match the new numbering.
- [x] Add or update example `.ocelot` files that demonstrate both line and block comments.
- [x] Run `nao check`.
