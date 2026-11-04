# Why does interpolation need its own brace-depth stack?

- Learned: 2026-11-04

## What is the finding?

Template interpolation and ordinary block braces are separate nesting dimensions.
A closing block brace must leave the lexer in expression mode; only the matching
interpolation brace resumes string-text scanning. Nested templates require this
brace count for each active interpolation rather than one global brace depth.

## What is the context?

Block expressions added braces inside `${...}`. The earlier lexer used a counter
of active interpolations and resumed string scanning at every `}` while that
counter was nonzero. Reading that code establishes why it cannot distinguish
a new block's closing brace from the enclosing interpolation's delimiter.

## What evidence supports the finding?

The source-driven `eval_returns_block_values_and_semicolons_discard_them` test
in `embedded_engine.rs` evaluates nested blocks and nested templates, including
`"value ${{ { true } }}"` and `"${{ "inner ${ { false } }" }}"`, and checks their
exact runtime string values. These cases pass with a stack of brace depths,
one entry per active interpolation. The block spec includes an executable example.

The prior limitation is derived from the implementation at commit `f634bb3`;
it is not a claim that the old parser supported block expressions. The new
regressions establish the integrated lexer/parser/runtime behavior.

## What references apply?

- Internal: [Template lexing](../../crates/parser/src/lexer/lex.rs),
  [embedded regression cases](../../crates/engine/src/embedded_engine.rs),
  [executable block examples](<../spec/02.04 Expressions - Blocks.md>), and
  [DR-015](<../decisions/DR-015 Use block values before expression conditionals.md>).
- External: No applicable external source; this finding concerns the repository's lexer states.
