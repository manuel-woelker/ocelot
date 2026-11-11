# What will be added?

Add signed 64-bit `int` with decimal literals, typed bindings/parameters/results,
unary negation, binary `+`, `-`, `*`, `/`, and integer ordering `<`, `<=`, `>`, `>=`.
Extend existing equality, display, interpolation, assertions, embedding, formatting,
and C emission to integers. Do not add floats, coercions, remainder, loops, mutation,
alternate bases, or digit separators.

# What semantics will be shared by both backends?

- Range is -9223372036854775808 through 9223372036854775807. Reject out-of-range
  literals during parsing, including arbitrarily long digit strings.
- Minus remains an explicit prefix operator. The direct spelling
  `-9223372036854775808` is valid; positive `9223372036854775808` is not. Keep an
  unsigned literal magnitude in the AST solely to represent that direct minimum
  spelling, with checked parser context and explicit handling in both evaluators.
- Preserve the no-precedence rule. Negative operands need grouping in binary
  expressions, e.g. `(-3) * 4` and `x - (-y)`. Repeated unary minus is valid;
  prefix forms are grouped when combined with any binary operator. Homogeneous
  arithmetic chains evaluate left to right; ordering/equality cannot chain.
- Operands must be integers; arithmetic produces int, ordering produces bool.
  Equality still requires matching types; there are no implicit conversions.
- Evaluate operands once, left to right, propagating early returns and errors.
  Check all source paths and effects conservatively, even in skipped operands.
- Arithmetic overflow, negation of minimum int, division by zero, and minimum
  int divided by -1 produce source-located language errors in every build profile.
  Division truncates toward zero. Native C must not execute undefined signed
  overflow or rely on implementation-defined unsigned-to-signed conversions.
- Arithmetic and unary negation conservatively carry the existing `panic` effect,
  including literal operations. Ordering and equality do not add that effect.
  Use one shared diagnostic builder and identical error messages in both backends.

# In what order will implementation proceed?

- [x] Add integer literal, integer binary operator/expression, and unary negation
  AST nodes; lex digits and longest-match operator tokens without breaking `->`,
  comments, templates, or parameter syntax.
- [x] Extend the flat parser, signed-minimum handling, grouping validation, spans,
  and formatter; reject malformed/out-of-range literals and mixed operators.
- [x] Add the canonical int type and value; support signatures, bindings, native
  labels, equality, printed/interpolated/asserted/eval values, and checked static
  typing/effect propagation, including grouped and non-fallthrough operands.
- [x] Implement checked interpretation and a shared source diagnostic contract.
  Extend the spec runner's structured runtime-error rendering if needed.
- [x] Add an unboxed integer tag to the C runtime, exact decimal rendering, checked
  arithmetic/negation helpers, safe payload conversion, and sequenced C lowering.
- [x] Add source-driven parser/formatter/engine/session tests and executable spec
  chapters. Cover all operators, signed limits, division signs, intermediate
  overflow, skipped failures, effect prohibitions, return/evaluation order, and
  mixed-type/operator errors in both backends.
- [x] Expand strict C99 ASan/UBSan/leak tests and direct C boundary tests; record
  DR-023 and update implemented-language/native/spec documentation.
- [x] Run `nao check`, both complete spec passes, source-free/native smoke tests,
  sanitizer checks, links, and `git diff --check`. Record results and move the
  completed plan before committing the implemented feature.

# What risks need explicit checks?

Minimum int needs contextual literal handling; ordinary positive magnitudes must
never silently become negative. Unary minus must not bypass the grouping rule.
Checked C multiplication and division must guard before evaluating risky signed
operations. Integer formatting inside concatenation needs independent live buffers
for each operand. Arithmetic effects must be recorded directly, not through a
fictional native call. Existing diagnostics listing allowed types and old malformed
numeric fixtures need deliberate updates rather than broad snapshot weakening.

# How will correctness be established?

The same executable examples must pass independently through interpretation and
C/TCC. Compare exact values, diagnostics, failure ordering, and source spans.
Exercise edge arithmetic with optimized Clang plus UBSan, not only TCC. Keep the
minimal native-binary size regression and existing string-ownership checks.
Implementation is complete. `nao check` passes: 394 Rust tests and all 141
examples across 26 spec chapters independently pass interpretation and C/TCC.
The strict optimized C99 ASan/UBSan/leak smoke script passes, including valid
boundary operand pairs against compiler overflow builtins and each integer
failure path. The builtin oracle is enabled only in the Clang check; the shared
C fixture remains compilable by TCC. Source-free execution and arithmetic
diagnostics, the minimal-binary size guard, and `git diff --check` pass.
No planned work remains; floats, coercions, mutation, and loops remain outside scope.
