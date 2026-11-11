# DR-023: Use checked signed 64-bit integers

- Status: Accepted
- Date: 2026-11-11

## What is the decision?

Add `int` as a signed 64-bit value with decimal literals, unary minus, checked
`+`, `-`, `*`, `/`, ordering, and matching-type equality. Division truncates toward
zero. Overflow and division by zero are source-located language runtime errors in
every build. Arithmetic and negation conservatively contribute `panic`; ordering
and equality add no effects. Extend existing signatures, bindings, formatting,
display, interpolation, assertions, and session values without implicit coercions.

Apply [DR-022](<DR-022 Require explicit grouping between different operators.md>)
to all new operators. Homogeneous arithmetic chains evaluate left to right;
comparisons cannot chain. Prefix forms need grouping with binary operations,
including `(-3) - 4`. Repeating one prefix operator is allowed; mixing prefixes
requires parentheses.

Store decimal magnitudes in the AST, allowing 2^63 only directly under unary
minus. Interception of that node pair produces minimum int in each backend;
positive 2^63 is never a valid standalone expression. Native values store integer
bits unboxed in a new tag. Decode high unsigned payloads using defined arithmetic
and guard before signed C operations. Use one shared diagnostic builder.

## Why was this chosen?

Exact integers enable useful calculations without introducing float rounding or
multiple numeric types. Fixed width maps directly to the target machine and C
runtime. Uniform checked behavior prevents Rust profile differences and C undefined
behavior from becoming language semantics. Existing effects communicate possible
failure without adding a new mechanism. Conservative effects keep validity stable
across constant folding and other future optimizations.

The minimum integer spelling needs special handling because its positive magnitude
exceeds the type's range. An unsigned AST magnitude preserves explicit unary syntax
and permits the familiar spelling while retaining strict positive literal checks.

## What is the context?

The interpreter and native C/TCC backend already agree on typed expressions,
returns, equality, grouping, and short-circuit boolean operations. Integers extend
those foundations. The user requested a plan followed by implementation of the
recommended integer/arithmetic step.

- [Implementation plan](../plans/completed/2026-11-11-integers-and-arithmetic-plan.md)
- [Integer specification](<../spec/10.03 Types - Integers.md>)
- [Operation specification](<../spec/02.09 Expressions - Integer operations.md>)
- [Checked C implementation](../../crates/codegen_c/src/native_integer.c)

## What are the consequences?

Calculations remain exact and agree across backends. Native integers require no
allocation, although embedded error diagnostics still use the existing string
ownership mechanism. Even constant arithmetic prevents `cannot panic`; range
analysis could refine this later only through a deliberate language decision.
There are no floats, coercions, remainder, alternate bases, loops, or mutation.

## What alternatives were considered?

- **Floating point first:** Convenient but loses exactness for large whole numbers.
- **Wrapping or saturating operations:** Fast or predictable, but silently changes
  ordinary mathematical results rather than reporting mistakes.
- **Profile-dependent checks:** Produces different language behavior in development
  and production; C overflow remains undefined without explicit guards.
- **Arbitrary precision:** Adds allocation and implementation complexity before
  evidence that programs need values outside 64 bits.
- **Treat signed literals as indivisible tokens:** Hides unary operator semantics
  and complicates a consistent no-precedence rule.
- **TCC-specific overflow builtins:** Couples semantics to a replaceable toolchain;
  portable C99 guards work with TCC and optimized sanitizer builds.
