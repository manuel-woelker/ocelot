# Why is an `assert_eq` builtin needed now?

The language can already execute tests, but test bodies still have no first-class assertion primitive.
Right now the only way to fail a test is indirectly, for example by triggering a missing identifier or a native-function type error.

That is too crude for actual test authoring.
A test assertion should:

- compare two runtime values directly
- fail intentionally instead of relying on unrelated interpreter errors
- point at the assertion call site with a source diagnostic
- show the expected and actual rendered values in a useful diff-like form

Without that, tests exist structurally but are not yet pleasant or trustworthy to write.

# What should the first `assert_eq` slice include?

The first slice should support:

- a builtin call shaped like `assert_eq(expected, actual);`
- value equality for the runtime values that currently exist
- sourcediagnostic-style failure rendering that points at the `assert_eq(...)` line
- a diff of the two string representations when the values differ

This slice should not yet support:

- custom failure messages
- deep structured diffs for future collection types
- snapshot assertions
- approximate numeric comparison
- special parser syntax for assertions

The simplest correct model is a normal builtin function with special failure formatting for tests.

# What is the current implementation gap?

Today:

- the interpreter can dispatch native builtins such as `println`
- test execution failures are wrapped in generic `test \`...\` failed` message errors in [`Engine::run_test()`](/data/projects/ocelot/crates/engine/src/engine.rs) and [`Engine::run_tests()`](/data/projects/ocelot/crates/engine/src/engine.rs)
- there is no dedicated structured error kind for assertion failures
- there is no diff rendering helper for runtime values

That means adding only a new builtin branch in the interpreter would be insufficient.
The repository also needs a failure-reporting path that can carry source-oriented assertion diagnostics through test execution.

# What should equality mean in the first version?

The first version should use straightforward runtime-value equality:

- strings compare by exact contents
- unit compares equal to unit
- mismatched value kinds compare unequal

That matches the current [`RuntimeValue`](/data/projects/ocelot/crates/interpreter/src/runtime_value.rs) scope and keeps behavior unsurprising.
If booleans or numbers land later, they can extend the same equality operation.

# How should assertion failures be represented?

For now, assertion failures should become a first-class `OcelotError` category rather than a separate interpreter-only failure transport.

Recommended direction:

- add `ErrorKind::AssertionError(...)` in [`error.rs`](/data/projects/ocelot/crates/base/src/error.rs)
- back that variant with a small structured payload that captures:
  - the source file path
  - the source span of the `assert_eq` call
  - a stable summary such as ``assert_eq` values differ`
  - the rendered expected value
  - the rendered actual value
- convert that payload into a user-facing source diagnostic plus diff text when test execution reports the failure

The key design point is still separation of concerns:

- the interpreter should know what failed and where
- the error payload should preserve that structure
- the engine or a small shared renderer should know how to format that failure for users

That is not quite as decoupled as a separate interpreter-failure type, but it is still much better than building a single large error string inside the builtin implementation.

# How should the failure output look?

The failure output should have two parts:

1. a sourcediagnostic pointing at the `assert_eq(...)` call site
2. a short diff section showing expected and actual rendered values

A good first output shape is:

```text
error: assert_eq values differ
  ╭▸ examples/tests.ocelot:2:5
  │
2 │     assert_eq("a", "b");
  ╰╴    ━━━━━━━━━━━━━━━━━━━ assertion failed here

expected:
  "a"
actual:
  "b"
```

If a slightly more explicit diff format is easy, even better, for example:

```text
diff:
- "a"
+ "b"
```

The first slice does not need a sophisticated line-oriented diff algorithm.
It just needs output that is unambiguous and pleasant to read.

# Where should the source diagnostic data come from?

The parser already preserves expression spans, so the interpreter should use the call-expression span for `assert_eq(...)` failures.

Recommended direction:

- use the enclosing call-expression span as the primary annotation
- reuse the existing source-diagnostic rendering infrastructure rather than inventing a second callout format
- keep assertion diagnostics separate from compilation diagnostics even if they share rendering helpers

This is important because assertion failures are runtime test failures, not parser or resolver errors.

# How should builtin dispatch change?

Builtin dispatch should stay generic.
`assert_eq` should become another native function alongside `println`, not a parser special case.

Recommended interpreter behavior:

- require exactly two arguments
- evaluate both arguments in source order
- compare the resulting runtime values
- return unit on success
- raise an `OcelotError` whose kind is `AssertionError` on mismatch

If the wrong arity is supplied, the interpreter can use the same user-facing style as other native builtin argument errors.

# What helper APIs will make this manageable?

This change will be cleaner if it adds small reusable helpers instead of burying logic inside one builtin branch.

Useful helpers include:

- `RuntimeValue::equals(&self, other: &Self) -> bool` or equivalent trait impl use
- `RuntimeValue::render_for_assertion()` or equivalent display helper
- a small assertion-error renderer that combines a `SourceDiagnostic` with expected/actual or diff text

That keeps later assertion-style builtins from duplicating formatting logic.

# What implementation order keeps this manageable?

1. Extend the interpreter native builtin dispatch with `assert_eq`.
2. Add runtime-value helpers for equality and stable user-facing rendering.
3. Add an `AssertionError` variant to `ErrorKind` with structured payload for source location plus rendered values.
4. Add a renderer that formats assertion errors as a source diagnostic followed by expected/actual or diff text.
5. Update engine test execution paths so assertion errors render through that structured path instead of generic wrapped errors.
6. Add tests covering successful assertions, mismatches, wrong arity, and aggregated `run_tests()` reporting.
7. Add spec documentation for `assert_eq` if the repository is ready to make its output shape stable.
8. Run `nao check`.

# What verification should this work include?

Verification should include:

- colocated interpreter tests for:
  - successful `assert_eq("a", "a");`
  - failing `assert_eq("a", "b");`
  - wrong-arity `assert_eq("a");`
- engine tests for:
  - `run_test()` reporting a sourcediagnostic and diff for one failing assertion
  - `run_tests()` including the assertion failure in the failed-test summary
- spec-validation updates if a new `assert_eq` chapter or examples are added
- running `nao check`

# What landed from this plan?

This change added the first assertion-oriented builtin and failure path:

- `assert_eq(expected, actual)` now exists as a native builtin function
- [`RuntimeValue`](/data/projects/ocelot/crates/interpreter/src/runtime_value.rs) now exposes equality and stable assertion rendering helpers
- [`ErrorKind`](/data/projects/ocelot/crates/base/src/error.rs) now includes a boxed `AssertionError` payload with source diagnostic data and rendered values
- assertion failures render as a source diagnostic plus a short diff through [`render_assertion_error`](/data/projects/ocelot/crates/base/src/assertion_error.rs)
- engine test execution now reports assertion mismatches with source context and diff output instead of treating them like generic wrapped errors
- interpreter, engine, and base formatting coverage were updated
- `nao check` passes

Spec documentation was intentionally deferred in this change.
The builtin behavior is stable enough in code, but the repository does not yet have a clean spec-validation path for test-oriented execution output, so locking the exact assertion-reporting shape into `docs/spec` would be premature.

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes `assert_eq` is primarily intended for test bodies, even if the builtin technically exists during ordinary script execution too.
- The first diff should optimize for clarity, not cleverness. Pulling in a sophisticated diff engine now would be overkill.
- The exact argument order needs to stay explicit. This plan assumes `assert_eq(expected, actual)`.
- Putting assertion failures into `ErrorKind` is a pragmatic short-term choice; if more runtime-specific failure kinds appear later, the repository may still want a dedicated structured runtime-failure layer.
- If assertion failures are rendered as plain message strings too early, it will be harder to preserve structured source information later.
- The exact wording of the summary line and diff headers should be stabilized before writing spec examples that lock the output down.

# What concrete tasks should track this plan?

- [x] Add an active plan for the `assert_eq` builtin and its failure-reporting path.
- [x] Implement `assert_eq(expected, actual)` as a native builtin function.
- [x] Add runtime-value helpers for equality and stable assertion rendering.
- [x] Add an `AssertionError` variant to `ErrorKind` with structured assertion payload data.
- [x] Render assertion mismatches as a source diagnostic plus expected/actual or diff output during test execution.
- [x] Add interpreter and engine coverage for successful assertions, mismatches, and wrong arity.
- [x] Decide whether to add spec documentation now; defer it until test-oriented output has a stable spec-validation path.
- [x] Run `nao check`.
