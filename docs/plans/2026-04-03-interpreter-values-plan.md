# Why is another interpreter value plan needed now?

The interpreter now has a minimal [`RuntimeValue`](/data/projects/ocelot/crates/interpreter/src/runtime_value.rs) enum, but it was introduced only to unblock function-call execution.
That first slice is enough for `println`, yet it does not clearly define how the interpreter should grow once the language adds booleans, numbers, or richer callable behavior.

If value representation remains accidental, future work will keep re-litigating basic questions:

- which values are immediate versus heap-backed
- where type-specific helpers should live
- how native functions should validate and convert arguments
- whether performance-motivated representations such as NaN-boxing are even worth the complexity

This plan should turn the current placeholder into an intentional runtime value model.

# What should the near-term scope include?

The near-term scope should stay deliberately small:

- keep string values as the first real runtime data type
- keep a unit-like value for expression statements and native calls with no useful result
- define the shape of the value API so adding booleans and numbers later is mechanical
- improve native-function-facing helpers so call sites do not manually pattern-match every value

This plan should not yet implement:

- arithmetic
- boolean logic
- user-defined functions or closures
- garbage collection
- object models, arrays, or maps
- representation-level micro-optimizations

# What should the runtime value API grow toward?

The runtime value type should remain an ordinary Rust enum for now, but its API should start looking like a real interpreter value layer rather than a two-variant placeholder.

Recommended direction:

- keep `RuntimeValue` as an enum
- use [`SharedString`](/data/projects/ocelot/crates/base/src/shared_string.rs) for string payloads
- add small helpers such as:
  - constructors for common values
  - accessors like `as_string()`
  - expectation helpers for native calls such as `expect_string()`
  - predicates or formatting helpers when they become useful
- keep representation concerns hidden behind the `RuntimeValue` module so future changes do not leak throughout the interpreter

This keeps the interpreter readable while still making later evolution cheap.

# What value variants should be planned for even if they do not all land immediately?

The first implementation can stay narrow, but the plan should reserve conceptual room for:

- `String`
- `Unit`
- `Boolean`
- `Number`

The exact number representation can remain open for now.
If the language expects one obvious numeric type early, `f64` is the easiest short-term fit for interpreter work even if the eventual language spec becomes more precise later.

# How should native functions interact with values?

Native functions should not each reinvent value conversion and validation.

Recommended direction:

- move argument validation toward helper methods on `RuntimeValue` or a small native-call utility layer
- return `RuntimeValue` from native functions, even when the result is usually `Unit`
- keep error messages stable and user-facing

This avoids a pile of repeated match expressions once the standard library grows past `println`.

# How difficult would a NaN-boxing representation be in Rust?

NaN-boxing is possible in Rust, but it is a poor default for this repository at the current stage.

The main issues are:

- it is substantially more complex than an enum-based value type
- it tends to require `unsafe` code, bit-casting, and careful invariants around payload layout
- heap-backed values such as strings still need pointer management, so NaN-boxing does not remove the hard ownership questions
- it makes debugging, testing, and maintenance worse unless profiling proves the enum representation is actually a bottleneck
- Rust’s enums are already efficient enough for an early tree-walking interpreter unless the project is chasing VM-level performance immediately

So the plan should explicitly assume:

- use a Rust enum now
- keep representation changes encapsulated
- revisit NaN-boxing only after there is profiler evidence that value-tag overhead matters

If the project eventually builds a bytecode VM with a numeric-heavy hot path, NaN-boxing could become reasonable.
For the current tree-walking interpreter, it is probably overengineered and not where the real performance wins live.

# What implementation order keeps this manageable?

1. Expand [`RuntimeValue`](/data/projects/ocelot/crates/interpreter/src/runtime_value.rs) into a small interpreter value module with constructors and typed access helpers.
2. Refactor native call code in [`interpreter.rs`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) to use those helpers instead of ad hoc matching.
3. Add the next planned value variants only if a concrete language feature needs them; do not add dead variants speculatively.
4. Add tests that lock down value helper behavior and native-call argument validation.
5. Document the representation choice in code comments or plan follow-up notes if future contributors are likely to try premature low-level optimizations.
6. Run `nao check`.

# What verification should this work include?

Verification should include:

- colocated tests for `RuntimeValue` helper methods
- interpreter tests covering successful string extraction and type-mismatch failures
- native-function tests that demonstrate argument validation through shared helpers rather than one-off matching
- running `nao check`

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes the interpreter stays tree-walking for now. A future VM could justify different value-representation tradeoffs.
- This plan assumes strings remain heap-backed and should prioritize ergonomic ownership over clever tagging schemes.
- Adding `Boolean` and `Number` too early without language features that use them would be cargo-cult architecture.
- If the language later needs precise integer semantics, choosing `f64` too aggressively could create churn; the first numeric variant should follow actual language design, not habit.
- NaN-boxing should be treated as an optimization strategy, not as the baseline architecture.
- A short in-code rationale comment may be worthwhile once the value module grows, so future changes do not reintroduce premature representation cleverness.

# What concrete tasks should track this plan?

- [ ] Add an active plan for intentional interpreter value representation.
- [ ] Expand `RuntimeValue` into a small value API with constructors and typed access helpers.
- [ ] Refactor native-function code to use shared value helpers.
- [ ] Add tests for value helpers and native-call argument validation.
- [ ] Document the current representation choice and explicitly defer NaN-boxing unless profiling justifies it.
- [ ] Run `nao check`.
