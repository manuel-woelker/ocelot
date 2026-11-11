# DR-022: Require explicit grouping between different operators

- Status: Accepted
- Date: 2026-11-11
- Supersedes: [DR-018](<DR-018 Compare matching value types with explicit equality operators.md>)
  for its precedence and grouping choices; equality's type and evaluation rules remain.

## What is the decision?

Operators have no implicit relative precedence. At each expression level, mixing
different operators requires explicit parentheses. This includes prefix `not`:
write `(not a) == b` or `not (a == b)`, never `not a == b`.

Add `(expression)` as a retained AST grouping node and add short-circuit `and` and
`or`. Homogeneous boolean chains are allowed and evaluate left to right. Equality
remains non-chainable, including repeated `==` or `!=`; nested comparisons need
grouping. Repeated `not` remains valid. Calls are syntactic operands rather than
operators, and argument/block/condition/interpolation boundaries contain their
own expressions. Grouping preserves scopes, values, types, and control flow.
Grouped identifiers can be direct callees without introducing first-class functions.

Boolean operators require bool operands (or internal non-fallthrough expressions).
Evaluate left once; skip right on false for `and`, true for `or`. Resolve both
operands and conservatively track their effects. Only a non-fallthrough left
guarantees non-fallthrough for the entire operation; do not add constant-truth
analysis. Retain all explicit parentheses during formatting.

## Why was this chosen?

The user explicitly specified no operator precedence and mandatory parentheses
between different operators. The syntax exposes intent without requiring readers
to remember a precedence table. Keeping equality non-chainable avoids misleading
mathematical-looking comparisons, while homogeneous boolean chains are useful
and have a clear evaluation sequence.

Short-circuiting composes with the existing effect and return machinery. Explicit
guarded C lowering preserves Ocelot behavior without relying on C operator order.
Keeping groups in the AST prevents formatting from erasing required syntax.

## What is the context?

[DR-018](<DR-018 Compare matching value types with explicit equality operators.md>)
previously made `not` bind more tightly than equality. This decision deliberately
rejects those ungrouped mixtures; their fixtures now state the intended grouping.
The rest of the language's type/effect/return and C-runtime decisions remain intact.

- [Plan](../plans/completed/2026-11-11-parentheses-and-boolean-operators-plan.md)
- [Grouping rules](<../spec/02.07 Expressions - Parentheses.md>)
- [Boolean rules](<../spec/02.08 Expressions - Boolean operators.md>)
- [Flat operator parser](../../crates/parser/src/parser.rs)
- [Static checking](../../crates/resolver/src/resolver.rs)
- [Interpreter](../../crates/interpreter/src/interpreter.rs)
- [Guarded C lowering](../../crates/codegen_c/src/c_emitter.rs)

## What are the consequences?

- Previously accepted unparenthesized `not`/equality mixtures are syntax errors.
- Boolean keywords become reserved, and empty `()` remains a type spelling rather
  than a new unit literal. Existing calls and type annotation syntax remain valid.
- Skipped code may still cause compile-time name, type, return, or effect errors.
- Returning from the right operand does not satisfy a function's unconditional
  completion check when the left can skip it, including literal-left cases.
- Comments inside expressions follow the existing restrictions.
- Future operators must follow the same grouping rule rather than adding a
  precedence tier.

## What alternatives were considered?

- **Conventional precedence:** Explicitly contrary to the user's language rule.
- **Left-associate all mixed operators:** Predictable mechanically but still hides
  an implicit interpretation instead of requiring the intended structure.
- **Allow `not` as a precedence exception:** Makes the global rule inconsistent.
- **Erase parentheses after parsing:** Loses syntax needed to validate/format the
  no-precedence rule reliably.
- **Permit equality chains:** Retains the ambiguity rejected in DR-018.
- **Skip static checks on skipped operands:** Makes language validity depend on
  literal values and future optimization rather than the complete source.
