# Why add `assert()` now?

The language already has boolean literals and `assert_eq`, but it still lacks the simplest assertion form: checking that one boolean condition is true.
Without `assert()`, even straightforward test expectations have to be expressed indirectly or by comparing to `true` with `assert_eq`.

That is clunkier than it needs to be.
The first boolean-aware test API should include the direct form.

This slice should stay narrow:

- add a builtin `assert(condition)`
- require exactly one argument
- require that argument to evaluate to a boolean
- fail with an assertion-style error when the condition is `false`

No custom assertion message and no richer predicate API are needed yet.

# What is the current gap in the implementation?

The current interpreter supports:

- `println(value)`
- `assert_eq(expected, actual)`

`assert_eq` already produces structured assertion failures through [`AssertionError`](/data/projects/ocelot/crates/base/src/assertion_error.rs), but there is no direct boolean assertion builtin.

That leads to two awkward outcomes:

- `assert_eq(condition, true)` becomes the de facto replacement for `assert(condition)`
- test intent is noisier than necessary in both examples and future specs

Since booleans now exist as a primitive type, the missing builtin is easy to feel.

# How should `assert()` behave?

`assert()` should be a builtin function that takes exactly one boolean argument.

Recommended behavior:

- `assert(true)` succeeds and returns unit
- `assert(false)` fails with an assertion-style error
- `assert()` with the wrong arity is a type error
- `assert("hello")` or any non-boolean argument is a type error

This should mirror the repo's current native-function style: explicit arity checking first, then value-type validation.

# What failure shape should `assert(false)` use?

`assert(false)` should use the same structured assertion-error path as `assert_eq`.
That keeps CLI output, test summaries, and engine behavior consistent.

The cleanest approach is:

- reuse [`AssertionError`](/data/projects/ocelot/crates/base/src/assertion_error.rs)
- use the asserted expression span as the highlighted source region
- use a stable summary such as `assert condition was false`
- do not render an `expected` / `actual` diff block for this builtin

That produces the right user experience.
`assert_eq` benefits from a diff because it compares two values.
`assert(false)` does not.

This likely means either:

- extending [`AssertionError`](/data/projects/ocelot/crates/base/src/assertion_error.rs) so the diff block can be omitted when it adds no value
- or introducing a closely related assertion-style rendering path for unary assertions

The implementation should optimize for clean user-facing output, not for forcing every assertion form through the exact same rendering shape.

# How should the interpreter implement `assert()`?

The work belongs in [`crates/interpreter/src/interpreter.rs`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs), next to the existing builtin dispatch for `assert_eq` and `println`.

Recommended implementation steps:

1. Extend builtin dispatch so `assert` is recognized.
2. Add `evaluate_assert_call(...)`.
3. Check that exactly one argument is present.
4. Evaluate the argument expression.
5. Require a boolean runtime value.
6. Return unit when the value is `true`.
7. Produce an `AssertionError` when the value is `false`.

This should share as much shape as possible with `evaluate_assert_eq_call(...)`.
If the two builtins drift into separate ad hoc styles, the assertion API will get messy fast.

# What runtime support is required?

The runtime value layer already supports booleans, equality, and assertion rendering.
That means no new value variant is needed.

The only additional runtime helper that may be worth adding is a boolean expectation helper on [`RuntimeValue`](/data/projects/ocelot/crates/interpreter/src/runtime_value.rs), analogous to `expect_string(...)`.

That helper is recommended if it keeps builtin type checking consistent and readable.
If it would be a one-off wrapper with no reuse, local matching in the interpreter is fine.

# What spec chapters should document `assert()`?

`assert()` belongs in the standard-library chapter set, alongside `println()`, and it should also influence runtime-facing examples.

The recommended spec work is:

- add `30.02 Standard library - assert`
- describe `assert()` as a builtin that requires one boolean argument
- include success, false-condition failure, and wrong-type or wrong-arity examples as appropriate

The first version should keep the contract small and explicit:

- `assert(true)` succeeds
- `assert(false)` fails the current test or script execution with an assertion error
- `assert()` does not take a custom message yet

If `assert_eq` also deserves a dedicated spec chapter later, that can follow.
This plan should not block on redesigning the whole assertion chapter layout.

# What examples should the repository add or update?

The repository should add or update small examples that use `assert()` directly.

Recommended example work:

- add a focused file such as `examples/assert.ocelot`, or update an existing test-oriented example
- show `assert(true)` in a test item
- optionally show `assert(false)` in a failing example file if the repository wants a manual repro case

The examples should make the intended usage obvious without faking future features like custom messages or boolean operators.

# What tests should verify the feature?

Verification should stay colocated and focus on observable behavior.

Interpreter tests in [`crates/interpreter/src/interpret_script.rs`](/data/projects/ocelot/crates/interpreter/src/interpret_script.rs) should cover:

- `assert(true);` succeeds
- `assert(false);` returns an assertion error
- `assert(false);` does not report an `expected` / `actual` block
- `assert();` fails with the exact arity error
- `assert("hello");` fails with a type error for a non-boolean argument

Engine tests in [`crates/engine/src/engine.rs`](/data/projects/ocelot/crates/engine/src/engine.rs) should cover:

- test execution renders `assert(false)` failures in the same style as `assert_eq`
- test summaries preserve the assertion output

Spec validation should cover:

- successful `assert(true)` examples
- failing `assert(false)` examples if a spec error example is added

# What implementation order is recommended?

1. Add builtin dispatch for `assert`.
2. Implement `evaluate_assert_call(...)` using the existing assertion-error path.
3. Add any small runtime helper needed for boolean type validation.
4. Add interpreter coverage for success, false-condition failure, wrong arity, and wrong type.
5. Add engine coverage for rendered assertion failures.
6. Add a spec chapter and example files for `assert()`.
7. Run `nao check`.

This keeps the work honest and small.
Starting with docs before the builtin exists would just create churn.

# What assumptions and open questions should stay explicit?

- This plan assumes `assert()` is a builtin function, not new syntax.
- The first version intentionally does not support a second message argument such as `assert(condition, "message")`.
- The first version assumes `assert()` should stay leaner than `assert_eq` and should not emit an expected/actual diff block for `assert(false)`.
- If the repository later wants richer assertion APIs, `assert()` should remain the smallest direct boolean assertion rather than being replaced by more abstract helpers.

# What concrete tasks should track this plan?

- [ ] Add builtin dispatch and interpreter support for `assert(condition)`.
- [ ] Reuse the structured `AssertionError` path for false-condition failures.
- [ ] Add interpreter coverage for successful, failing, wrong-arity, and wrong-type `assert()` calls.
- [ ] Add engine coverage proving `assert()` failures render cleanly through test execution.
- [ ] Add a `30.02 Standard library - assert` spec chapter with executable examples.
- [ ] Add or update example `.ocelot` files that demonstrate `assert()` usage.
- [ ] Run `nao check`.
