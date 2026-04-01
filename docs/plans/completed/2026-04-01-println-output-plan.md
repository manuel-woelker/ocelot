# Why start with `println()` for program output?

`ocelot` does not yet have an executable language implementation, but output is one of the first visible capabilities users expect from a language.
A minimal output model gives the spec something concrete, gives future parser and runtime work a stable target, and keeps early examples from being purely expression-oriented.

Starting with `println()` is the smallest useful slice:

- it gives programs a simple way to produce observable output
- it avoids inventing richer I/O abstractions too early
- it provides a natural first end-to-end example shape

# What should the first output design include?

The first slice should define:

- script-style execution for executable files
- a `println()` function for line-oriented standard output
- the basic runtime effect of writing output in source order

The first slice should not yet define:

- general stream APIs
- file I/O
- formatted interpolation syntax
- variadic formatting
- stderr handling

# Should `println()` be syntax or a function?

The first version should treat `println()` as a function-like builtin rather than special syntax.

That keeps the surface model simple:

- it looks like an ordinary call
- it composes naturally with statement-oriented code
- it leaves room for a future standard library story

The intended shape is:

```ocelot
println("hello");
```

# What argument model should `println()` use first?

The first version should accept exactly one text argument and append a trailing newline.

This is intentionally narrow.
Adding formatting placeholders, interpolation, or multiple arguments before the language has stable strings and function calls would be premature.

The first slice should therefore assume:

- one argument
- text output
- one newline appended automatically
- a unit-like return value

# What should executable programs look like?

The first version should specify that executable files run as scripts without a surrounding `main` function.

The spec should make it obvious that:

- top-level statements are the program body
- top-level statements execute in order
- each `println()` call contributes one line to stdout

# How should this land in the spec?

This feature should be documented with the first concrete numbered spec chapters in `docs/spec`:

- one chapter for script execution
- one chapter for `println()`

Each chapter should include:

- short prose
- small `ocelot` examples
- explicit `### Output` sections

# What examples should the first spec chapters include?

The first examples should stay tiny and deterministic.

Good initial examples include:

- one `println()` call producing one line
- two `println()` calls producing two lines in source order
- a top-level `println()` call in a script
- a wrong-arity or wrong-type example showing the expected failure shape

# How should this work be verified?

Verification should include:

- checking in the new plan
- adding the first numbered spec chapters under `docs/spec`
- updating `docs/spec/README.md` to link to those chapters
- running `nao check`

# What assumptions or open questions should remain explicit?

- String literal syntax is still provisional; the examples can use quoted text now, but the full string chapter may refine the exact rules later.
- The first spec can describe `println()` as builtin behavior now and defer the deeper question of whether it eventually lives in a standard prelude or library module.
- The exact wording of output-related diagnostics is still provisional and should not become more specific than necessary yet.

# What concrete tasks should track this plan?

- [x] Add a plan for initial program output via `println()`.
- [x] Add a numbered spec chapter for script execution.
- [x] Add a numbered spec chapter for `println()`.
- [x] Update `docs/spec/README.md` to reference the new chapters.
- [x] Run `nao check`.
