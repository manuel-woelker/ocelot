# DR-016: Implement conditionals as expressions with compatible branch results

- Status: Accepted
- Date: 2026-11-06

## What is the decision?

Implement `if condition { ... } else { ... }` as an expression using the block
results established by [DR-015](<DR-015 Use block values before expression conditionals.md>).
Require boolean conditions and braced branches; support else-if chains. Evaluate
the condition once and only execute the selected branch. Match normal branch
types, treating a branch that always returns as compatible with the other branch.
When both branches return, the conditional cannot fall through.

Omitted else supplies unit, so the then branch must be unit-valued or always
return. Keep all expressions usable as statements by writing a discarding `;`.
Discarding the final value does not suppress branch-type checks. Preserve the
existing semicolon rule rather than introducing a separate if statement node.
Statically check all branches and conservatively propagate their effects,
including for constant conditions. Propagate returns from conditions and branches.

## Why was this chosen?

One expression form composes into function tails, arguments, templates, and
ordinary statements. Unit defaults preserve effect-oriented conditional use
without inventing a value for an unselected branch. Non-fallthrough compatibility
supports early returns while accurately checking function completion paths.

## What is the context?

The user requested if expressions and expression statements after block results
were implemented. Existing expression statements already discard values with a
semicolon, and the resolver and interpreter already represent non-fallthrough
expressions and explicit return control. This feature completes that foundation
without replacing typed function signatures or the normal statement wrapper.

- [Conditional rules and executable examples](<../spec/02.05 Expressions - Conditionals.md>)
- [Parser](../../crates/parser/src/parser.rs)
- [Branch type checking](../../crates/resolver/src/resolver.rs)
- [Selected-branch evaluation](../../crates/interpreter/src/interpreter.rs)

## What are the consequences?

- If expressions produce usable values or can be explicitly discarded with `;`.
- A non-unit conditional needs an else branch, including when used as a statement.
- Both-returning branches satisfy non-fallthrough checks; one branch alone does not.
- Constant conditions do not hide invalid types, names, or prohibited effects.
- `if` and `else` become reserved keywords.
- Optional statement semicolons and implicit value coercions remain outside the language.

## What alternatives were considered?

- **Separate statement and expression conditionals:** Duplicates syntax and semantics
  and conflicts with the chosen expression-based design.
- **Ignore type mismatches when values are discarded:** Makes expression typing
  depend on context and hides incompatible API results.
- **Assign arbitrary values to a missing else:** Makes false-path behavior unclear;
  the established empty-block value is unit.
- **Prune static checks for literal conditions:** Would change name, return, and
  effect checking depending on optimization instead of the source structure.
