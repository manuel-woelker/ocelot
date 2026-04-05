# Why does the formatter still need lexer and AST changes first?

`ocelot` still has no formatter crate, but the main blocker is not formatting logic by itself.
The lexer currently discards comments, and the AST only retains semantic structure plus spans.
That is enough for compilation, but not enough for source emission that preserves comments.

The formatter goal in this slice is narrower and more pragmatic than a full concrete syntax tree:

- keep one AST
- retain comment and newline trivia in the lexer
- attach parsed trivia to AST nodes where comments are allowed
- format from that enriched AST without losing comments

This avoids introducing a separate syntax tree while still giving the formatter something real to work with.

# What behavior should this plan target?

The target is a first formatter pipeline that can:

- parse source files while retaining leading trivia on tokens
- attach trivia to `CompilationUnit`, top-level items, and statements
- preserve comments when formatting
- normalize spacing and line breaks for the currently supported language
- keep existing compiler stages working on the same AST crate

The first slice should prioritize fidelity and a clean ownership model over flexible comment placement everywhere.

# Where should comments be allowed in this first version?

Comments should only be allowed in item and statement positions for now.
That means comments may appear:

- before top-level items
- before statements in block bodies
- after an item or statement as a trailing same-line comment
- at file start or file end through `CompilationUnit` trivia

Comments should not be allowed inside expressions or other interior token positions in this slice.
Examples that should remain invalid for now include comments:

- between function arguments
- between function parameters
- between unary operators and operands
- between a callee and `(`

This restriction keeps the AST and parser small enough to implement without token-level formatter machinery everywhere.

# How should trivia be represented?

The AST should gain an explicit trivia type that can be attached to nodes.
The type should be structured, not a raw vector.

Recommended shape:

```rust
pub struct Trivia {
    pub leading: Vec<TriviaPiece>,
    pub trailing: Vec<TriviaPiece>,
}
```

with pieces that are formatter-relevant rather than raw whitespace dumps, for example:

- line comments
- block comments
- newline runs or blank-line separators

The AST should attach `Trivia` to:

- [`CompilationUnit`](/data/projects/ocelot/crates/ast/src/compilation_unit.rs)
- [`Item`](/data/projects/ocelot/crates/ast/src/item.rs)
- [`Statement`](/data/projects/ocelot/crates/ast/src/statement.rs)

The first version should not add trivia fields to expressions.
If later language work needs comments inside expressions, the same type can be extended to more nodes.

# How should the lexer expose trivia?

The lexer should stop discarding comments and relevant newline structure.
Comments and newlines should be lexed as trivia, not promoted to ordinary grammar tokens.

The token model should become trivia-aware, for example:

```rust
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
    pub leading_trivia: Trivia,
}
```

with these ownership rules:

- trivia before a token belongs to that token's `leading_trivia`
- the lexer only populates token-leading trivia
- any file-ending trivia is attached to the `EndOfFile` token's `leading_trivia`

This means a separate EOF-trivia container is unnecessary.
The parser can consume trivia from token boundaries and reassign it onto AST nodes.

# How should the parser map token trivia onto AST nodes?

The parser should continue parsing the existing language grammar from ordinary tokens.
The new work is to move trivia from token boundaries onto AST nodes with stable ownership rules.

Recommended ownership rules:

- trivia before an item belongs to that item's `trivia.leading`
- trivia before a statement belongs to that statement's `trivia.leading`
- a same-line comment after an item or statement belongs to that node's `trivia.trailing`
- file-leading trivia before the first item belongs to `CompilationUnit.trivia.leading`
- file-trailing trivia before `EndOfFile` after the last node belongs to the last node's trailing trivia when appropriate, otherwise to `CompilationUnit.trivia.trailing`

When the parser encounters comment trivia while parsing inside an expression or other unsupported interior position, it should emit a clear parser diagnostic rather than silently dropping or relocating that comment.

# What should the combined AST look like after this change?

The repository can keep one AST crate and enrich existing nodes instead of adding a separate syntax tree.
That means nodes remain semantic enough for the compiler, but gain formatter-facing trivia ownership where needed.

The expected change is modest:

- [`CompilationUnit`](/data/projects/ocelot/crates/ast/src/compilation_unit.rs) gains file-level trivia
- [`Item`](/data/projects/ocelot/crates/ast/src/item.rs) gains node trivia
- [`Statement`](/data/projects/ocelot/crates/ast/src/statement.rs) gains node trivia
- expressions remain trivia-free in the first slice

This keeps the tree readable while still making formatting possible.

# What should the formatter crate do in the first version?

The formatter crate should stay deliberately small.
It does not need to solve arbitrary token-preserving pretty-printing yet.

Recommended formatter scope:

- add `crates/formatter` to the workspace
- expose a function that formats one parsed compilation unit
- preserve all comments attached through node trivia
- normalize layout for the currently supported item and statement forms
- produce stable output so formatting twice is idempotent

The formatter may normalize whitespace aggressively as long as it preserves comment text, ordering, and attachment.

# What implementation order keeps this small and honest?

1. Add `Trivia` and `TriviaPiece` types to the AST crate.
2. Add trivia fields to `CompilationUnit`, `Item`, and `Statement`.
3. Update lexer token structures so each token retains `leading_trivia`, including EOF trivia on the `EndOfFile` token.
4. Add colocated lexer tests proving comments and newline structure are retained.
5. Update the parser to map token trivia onto AST node trivia at compilation-unit, item, and statement boundaries.
6. Add parser diagnostics for comments that appear in unsupported interior positions.
7. Add parser tests proving trivia ownership is stable and invalid comment placements fail clearly.
8. Create `crates/formatter` and implement stable emission for the current language surface.
9. Add formatter tests for comment preservation, normalized layout, and idempotence.
10. Run `nao check`.

# How should the work be verified?

Verification should prove both fidelity and boundary enforcement.

Required coverage:

- colocated lexer tests showing line comments, block comments, nested block comments, newline runs, and EOF trivia are retained in token trivia
- parser tests showing file-leading, item-leading, statement-leading, and same-line trailing comments are attached to the expected AST nodes
- parser tests showing comments in unsupported positions produce stable diagnostics
- formatter tests proving attached comments are emitted and not reordered
- formatter idempotence tests where formatting formatter output yields the same text
- fixture-style tests for comment-heavy files with top-level items, function bodies, and trailing comments
- `nao check`

The verification bar is "no comment loss in supported positions" rather than byte-for-byte source reproduction.

# What risks, assumptions, and open questions should stay explicit?

- This plan intentionally supersedes the assumption in [`2026-04-04-comments-plan`](/data/projects/ocelot/docs/plans/completed/2026-04-04-comments-plan.md) that comments should always be discarded after lexing.
- Restricting comments to item and statement positions is a real language design choice, not just an implementation shortcut. If that rule is relaxed later, more AST nodes will need trivia ownership.
- `Trivia` should stay formatter-oriented. Storing arbitrary raw whitespace text would make the model heavier without helping the current formatter goal much.
- The ownership rules for trailing comments need to be implemented consistently or the formatter will feel flaky.
- Keeping one AST is simpler today, but semantic annotations and formatting data now coexist in the same nodes. That is acceptable in this slice as long as the trivia surface stays narrow.

# What concrete tasks should track this plan?

- [ ] Add `Trivia` and `TriviaPiece` types to [`crates/ast`](/data/projects/ocelot/crates/ast).
- [ ] Add trivia fields to [`CompilationUnit`](/data/projects/ocelot/crates/ast/src/compilation_unit.rs), [`Item`](/data/projects/ocelot/crates/ast/src/item.rs), and [`Statement`](/data/projects/ocelot/crates/ast/src/statement.rs).
- [ ] Update lexer token structures so tokens retain `leading_trivia`, with trailing file trivia attached to the `EndOfFile` token.
- [ ] Add colocated lexer tests for line comments, block comments, nested block comments, newline runs, and EOF trivia retention.
- [ ] Update the parser to transfer token trivia onto compilation-unit, item, and statement AST trivia.
- [ ] Add parser diagnostics for comments in unsupported interior positions.
- [ ] Add parser tests for trivia ownership and invalid comment placement.
- [ ] Add `crates/formatter` to the workspace.
- [ ] Implement formatter emission from the enriched AST with comment preservation and stable output.
- [ ] Add formatter tests and fixtures for comment-heavy supported inputs.
- [ ] Run `nao check`.
