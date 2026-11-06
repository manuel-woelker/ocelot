# DR-018: Compare matching value types with explicit equality operators

- Status: Accepted
- Date: 2026-11-06

## What is the decision?

Implement `==` and `!=` for matching string, boolean, and unit operands, producing
boolean results without coercion. Compare strings by exact contents. Evaluate
operands once, left to right, propagating returns and errors. Resolve both operands
and track their effects even when one cannot complete normally. A non-fallthrough
operand makes the comparison non-fallthrough.

Calls and prefix `not` bind more tightly than equality. Reject ungrouped chained
comparisons; existing block expressions can group nested comparisons. Preserve
`=` solely for initialization. Represent comparisons with a dedicated equality
expression and operator enum, without introducing a general operator framework.

## Why was this chosen?

Equality makes [local bindings](<DR-017 Use immutable local bindings and lexical scopes.md>)
useful in conditional decisions. Matching types prevent silent mistakes such as
comparing a boolean with its printed string. Familiar symbolic operators keep
initialization and comparison visually distinct. Rejecting chains avoids surprising
boolean-to-boolean comparisons and leaves room for a deliberate future chaining
design. Existing block grouping avoids expanding this feature into new grouping syntax.

## What is the context?

The user requested equality expressions after immutable local bindings were added.
Runtime values already support equality for native assertions, and the parser has
call and prefix precedence levels. This extends expression resolution and evaluation
using the return and block semantics from
[DR-015](<DR-015 Use block values before expression conditionals.md>) and
[DR-016](<DR-016 Implement conditionals as expressions with compatible branch results.md>).

- [Rules and executable examples](<../spec/02.06 Expressions - Equality.md>)
- [Parser](../../crates/parser/src/parser.rs)
- [Type checking](../../crates/resolver/src/resolver.rs)
- [Evaluation](../../crates/interpreter/src/interpreter.rs)

## What are the consequences?

Comparisons work in bindings, calls, templates, block results, and conditions.
Mixed types fail before effects execute; a discarded result does not relax checking.
Unit comparisons still evaluate both operands and their effects. Negating an entire
comparison currently uses a block. Ordering, logical operators, user-defined equality,
parenthesized grouping, and implicit Unicode normalization remain outside this change.

## What alternatives were considered?

- **Allow mixed types and return false:** Hides accidental type mismatches.
- **Only expose assertion or comparison functions:** Makes ordinary conditions
  cumbersome and does not satisfy the requested expression syntax.
- **Left-associative chains:** A chain can silently compare a previous boolean
  result instead of expressing the intended relationship between values.
- **Mathematical comparison chains:** Requires additional evaluation semantics
  without a current need.
- **General binary operator infrastructure:** Adds abstraction before more operators
  establish concrete requirements.
