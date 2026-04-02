# Why do we need a dedicated plan for language-level test support?

`ocelot` currently treats an executable file as a sequence of top-level statements.
That is enough for the first `println()` examples, but it does not leave a clean place for test definitions.

Tests should not be disguised as ordinary functions.
That would blur runtime code and test-only code, force the language to invent special function semantics, and make future tooling more awkward than it needs to be.

This plan defines a first-class item model for source files and recommends a concrete way to represent tests as their own top-level items.

# What design goal should guide the representation?

The design should optimize for three things first:

- tests are explicit top-level declarations with their own syntax and AST node
- normal script execution can ignore test items without ambiguity
- future tooling can discover, list, filter, and run tests without reverse-engineering conventions

That means the representation should make `test` visible in parsing, AST shape, and runtime entrypoints rather than smuggling it through a function declaration with magical attributes or naming rules.

# What source-file model should the language move toward?

The parser should stop modeling a file as just `Vec<Statement>`.
Instead, it should model a file as a list of top-level items.

The recommended direction is:

- `SourceFileAst` or the existing `Script` type evolves to contain `Vec<Item>`
- `Item` carries a source span and an `ItemKind`
- `ItemKind` starts small, with only the cases the language actually supports

For the first test-support slice, the useful item kinds are:

- executable top-level statements
- test items

That can be represented either as one item per statement or as a single script-body item that owns a statement list.
The simpler near-term choice is one item per top-level statement because it keeps parsing incremental and avoids inventing an extra wrapper concept too early.

# How should tests be represented in source?

The key question is what a test declaration should look like at the syntax level.
These are the main viable options.

## Option: `test foo() { ... }`

This looks familiar, but it is the wrong shape for the stated goal.
It reads like a function declaration, invites function-call semantics, and makes it too tempting to implement tests as “functions with a bit on them.”

This option should be rejected.

## Option: `@test fn foo() { ... }`

This is common in annotation-heavy languages, but it has the same core problem as the special-function approach.
The primary declaration is still a function, and `@test` becomes metadata that tools must interpret.

This option should also be rejected.

## Option: `test foo { ... }`

This is a strong option.
It makes `test` the declaration form itself, keeps names stable for filtering and reporting, and does not imply that tests are callable functions.

It does, however, force test names to be identifier-shaped.
That is fine if the language wants test names to behave like ordinary symbols.

## Option: `test "prints one line" { ... }`

This is also a strong option.
It treats test names as user-facing labels instead of symbol names, which is nice for diagnostics and CLI output.
It also avoids future questions about whether tests live in the same namespace as functions or types.

The downside is that string-literal names are slightly less convenient for refactoring and symbol-oriented tooling.

# What representation should the first plan recommend?

The recommended first syntax is:

```ocelot
test "prints one line" {
    println("hello");
}
```

This is the cleanest match for the design goal.
It says “this is a test” directly, it does not pretend to be a function, and it gives the test runner a human-readable name without inventing a separate display-name mechanism.

For the first slice, a test item should contain:

- a required name string
- a body block containing statements
- a source span covering the whole item

The AST should make that explicit with a dedicated `TestItem` node rather than encoding tests as a variant of function declaration later.

# Why prefer a string name over an identifier name first?

String names fit the current maturity of the language better.
They solve the immediate UX problem without dragging in namespace design, symbol resolution rules, or questions about whether duplicate test names are allowed across files.

They also produce nicer output for a future `ocelot test` runner:

- `PASS prints one line`
- `FAIL println rejects zero arguments`

If the language later wants symbol-like test names, it can still add them deliberately.
Going the other direction is messier because a function-like or identifier-only model tends to leak into tooling and semantics.

# How should test items interact with normal script execution?

Normal script execution should only run executable top-level statements.
Test items should be discovered but ignored by `run_script`.

That split keeps production execution simple:

- `run_script(path)` executes only non-test top-level statements in source order
- `run_tests(path)` or a future project-level test command executes only test items

This avoids the very bad pattern where adding tests to a file accidentally changes the behavior of `ocelot run`.

# How should the AST and parser change?

The implementation should introduce first-class item parsing before adding test execution.

A reasonable first-step AST shape is:

```text
File
  items: Vec<Item>

Item
  kind: ItemKind
  span: Span

ItemKind
  Statement(Statement)
  Test(TestItem)
```

With:

```text
TestItem
  name: SharedString
  body: Vec<Statement>
  span: Span
```

Using `SharedString` for the test name matches repository guidance and keeps the representation aligned with future immutable metadata use.

The parser should:

1. parse top-level items instead of only statements
2. treat `test` as a reserved keyword when it starts an item
3. parse a string literal name after `test`
4. parse a block body containing statements
5. preserve the existing statement parser for test bodies and script-level statements

# What lexer and grammar support will this require?

The current lexer only supports identifiers, strings, parentheses, and semicolons.
The first test-support slice will therefore need to add at least:

- `{`
- `}`
- a way to distinguish the `test` keyword from a generic identifier

That is still a small grammar expansion.
It is much cheaper than adding full function syntax just to piggyback test support onto it.

# How should test execution behave in the first slice?

The first slice should keep semantics narrow.
A test passes if its body executes successfully.
A test fails if executing its body returns an error.

That means the language can ship useful test discovery and execution before it has assertion syntax.
Early tests can still verify behavior through:

- successful execution without errors
- expected output comparison in the test runner, if the runner captures stdout

Longer term, the language will likely want proper assertion constructs, but that should be a follow-up feature rather than a prerequisite for first-class test items.

# What should the runner and engine API look like?

The engine currently exposes `run_script`.
Once item parsing exists, the engine should grow explicit entrypoints instead of overloading one path with mode flags.

The recommended direction is:

- keep `run_script(path)` for normal execution
- add `discover_tests(path)` or equivalent file-level discovery API
- add `run_test(path, test_name)` and/or `run_tests(path)`

The exact public API can stay small at first, but the behavior boundary should be explicit.
Tests are a different execution mode, not a side effect of running a script.

# What should be out of scope for the first slice?

The first slice should not attempt to solve every future testing feature.
It should explicitly leave these out of scope:

- function declarations
- attributes or decorators
- project-wide test package discovery
- snapshot update workflows
- assertions beyond “body executed successfully”
- parallel test execution
- fixtures, setup hooks, or teardown hooks
- namespace rules for test names across modules

Trying to solve all of that now would overengineer the language before its item model is even real.

# What implementation order makes sense?

The work should land in a few sharp steps:

1. Introduce a file/item AST model so top-level syntax is no longer hardcoded as only statements.
2. Extend the lexer with braces and a reserved `test` keyword path.
3. Add parsing for `test "name" { ... }` items.
4. Update script execution to ignore test items and continue executing normal top-level statements in order.
5. Add engine APIs for test discovery and test execution.
6. Add a small CLI surface for running tests once the engine behavior is stable.
7. Add spec chapters that document test item syntax and runtime behavior.

This order keeps the core model honest.
If the repository skips straight to a test runner without first introducing items, the implementation will likely end up with hard-to-undo parser debt.

# How should this work be verified?

Verification should include:

- colocated lexer tests for braces and `test` keyword handling
- colocated parser tests for valid and invalid test item syntax
- AST-shape tests showing files can contain both statements and test items
- engine tests proving `run_script` ignores tests
- engine tests proving test discovery returns stable names in source order
- engine tests proving a passing test succeeds and a failing test reports the right test name
- running `nao check`

The tests should stay black-box where possible.
In particular, engine tests should use `PalMock` and verify observable behavior rather than poking at internal parser state more than necessary.

# What assumptions and open questions should stay explicit?

- The current language has no block syntax yet, so test items are likely the first feature that introduces `{ ... }` statement blocks.
- String-literal test names are recommended for the first slice, but identifier names remain a plausible later alternative if symbol-oriented tooling becomes more important.
- The exact CLI shape, such as `ocelot test path/to/file.ocelot`, should follow the engine model rather than lead it.
- The first slice assumes tests can live in executable source files without affecting normal execution because the runtime will explicitly ignore test items.
- If the language later adds modules, tests should remain item-level declarations within a module or file rather than attached metadata on functions.

# What concrete tasks should track this plan?

- [x] Introduce a file-level item AST with explicit `Item` and `ItemKind` nodes.
- [x] Update the parser to parse top-level items instead of only statements.
- [x] Add lexer support for `{`, `}`, and `test` item parsing.
- [x] Add a dedicated `TestItem` AST node with a string-literal name and statement body.
- [x] Add parser coverage for valid test items and malformed test declarations.
- [x] Update the interpreter and engine so normal script execution ignores test items.
- [x] Add engine APIs for test discovery and test execution.
- [x] Add colocated engine tests using `PalMock` for mixed script-plus-test files.
- [x] Add initial CLI support for running tests.
- [x] Add spec chapters documenting test item syntax and behavior.
- [x] Run `nao check`.
