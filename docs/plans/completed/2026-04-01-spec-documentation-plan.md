# Why do we need a dedicated spec documentation plan?

`ocelot` now has basic repository infrastructure, but it does not yet have a stable home for language specification work.
Without a clear structure, spec writing will drift into ad hoc notes, examples will become inconsistent, and later executable examples will be harder to retrofit.

This plan defines how to introduce `docs/spec` as the canonical location for the language spec, how files should be organized, and how spec examples should be written so they can later serve both as documentation and as conformance inputs.

# What should the spec directory structure look like?

The spec should live under `docs/spec`.

Each spec file should represent one focused topic and use a two-part numeric chapter prefix in the filename so the ordering is explicit in source control and stable for readers.

The intended filename pattern is:

```text
docs/spec/NN.MM Topic.md
```

Examples:

```text
docs/spec/01.01 Expressions - Binary operators.md
docs/spec/01.02 Expressions - Unary operators.md
docs/spec/02.01 Statements - Let bindings.md
docs/spec/03.01 Types - Primitive types.md
```

This numbering scheme should be treated as:

- `NN`: a major chapter such as expressions, statements, types, declarations, modules, effects, or runtime behavior
- `MM`: a subchapter inside that major chapter

The directory should also include an index document that explains chapter ordering and links to the individual files.

# What should each spec file contain?

Each spec file should describe one language feature in prose and include short examples.
The prose should focus on observable language behavior rather than parser or compiler implementation details.

A good spec chapter should usually include:

- a short statement of the feature or rule
- the syntax shape when relevant
- semantic rules or evaluation behavior
- edge cases or constraints when they matter
- short examples that make the rule concrete

Examples should stay small.
They should illustrate one rule at a time instead of turning each spec chapter into a tutorial or a kitchen-sink test file.

# How should examples be written so they can later drive verification?

The spec files should include executable-looking examples with explicit headings and fenced source blocks.
Those headings and code blocks should be structured so a later harness can extract them in the same spirit as `../holo/tests/conformance-tests`.

The first version should use a visible, Markdown-native structure such as:

~~~markdown
## Example: integer addition

```ocelot
fn main() -> i64 { 1i64 + 2i64; }
```

### Output

```text
3i64
```
~~~

For examples that are expected to fail, use a visible heading that makes the expectation explicit, for example:

~~~markdown
## Example: mismatched operand types

```ocelot
fn main() -> i64 { 1i64 + 2.0f64; }
```

### Output

```text
type error: arithmetic operands must have the same type
```
~~~

That gives the spec three useful properties:

- humans can read it as documentation
- examples remain easy to diff and review
- a future harness can locate example names, source blocks, and expected outputs without inventing a second fixture format

# What conventions should the example format adopt from `holo`?

The `holo` testing docs and conformance fixtures show a useful pattern: Markdown fixtures are readable first, machine-driven second.
`ocelot` should borrow that instinct without blindly copying `holo`'s exact case format.

The spec examples should adopt these principles:

- use headings to delimit cases clearly
- use exactly one language fenced block for the source under test
- use a dedicated output heading followed by a `text` block for expected output
- keep success and failure cases both visible in plain Markdown
- prefer tiny examples over broad omnibus examples
- keep expected output stable and text-based

Unlike `holo`'s conformance files, the spec chapters are organized by language topic rather than by testing phase.
That means one chapter may eventually contain parser-facing, typechecker-facing, and runtime-facing examples together if that best explains the language rule.

# How should the first implementation be staged?

The work should land in a few clear steps so the documentation format does not get overengineered before there is real spec content.

1. Create `docs/spec` and add a short index file describing the chapter numbering scheme.
2. Add a small number of initial chapter files to prove the filename convention and writing style.
3. Define and document the exact Markdown example contract for source and output blocks.
4. Add a lightweight validator or test harness only after there are enough real spec chapters to justify automation.

This order matters.
Starting with the harness would be premature; starting with prose but no consistent example shape would create migration churn later.

# What automation should eventually validate spec examples?

The medium-term goal should be a data-driven harness that reads `docs/spec/**/*.md`, extracts examples, and runs them as conformance-style cases.
That harness does not need to be built in the first slice, but the Markdown structure should be chosen now so that future work is straightforward.

The eventual harness should be able to:

- discover spec chapters in chapter order
- extract example headings as stable case names
- extract `ocelot` fenced blocks as input programs
- extract `### Output` `text` blocks as expected results
- distinguish successful output examples from diagnostic examples
- normalize output formatting so expectations stay reviewable

# What should be considered out of scope for the first slice?

This plan does not require:

- a complete language spec
- a finished executable spec harness
- final wording for every diagnostic message
- a promise that the current chapter numbers will never change

The first slice should establish structure and conventions, not freeze the entire language design.

# How should this work be verified?

Verification for the first slice should include:

- confirming that `docs/spec` exists and contains an index plus initial numbered chapter files
- reviewing that filenames follow the `NN.MM Topic.md` convention
- reviewing that each chapter includes prose, source examples, and explicit output blocks
- updating repository documentation if the spec directory becomes part of standard contributor workflow
- running `nao check`

If a follow-up slice adds a Markdown-driven harness, that follow-up should add focused automated tests for fixture discovery, parsing, and output comparison.

# What assumptions or open questions should stay explicit?

- The spec example fence should likely be `ocelot`, but that should be confirmed once editor tooling or syntax highlighting expectations are clearer.
- Output examples may eventually need more than one heading kind, such as `### Output`, `### Parsing error`, or `### Type error`; the first slice should start simple unless that immediately becomes awkward.
- Some examples may eventually need multi-file context, module layout, or explicit entrypoint rules; the initial format should optimize for single-file examples first.
- If the language gains nondeterministic or environment-dependent behavior, those examples should stay out of the spec harness unless normalization rules are defined.

# What concrete tasks should track this plan?

- [x] Create `docs/spec/`.
- [x] Add a spec index document that explains chapter numbering and links to chapters.
- [x] Add initial numbered chapter files that prove the naming convention.
- [x] Define the standard spec example shape using headings, `ocelot` fenced blocks, and explicit output blocks.
- [x] Ensure the initial chapters contain short prose plus short examples.
- [x] Update any higher-level repository documentation that should reference the spec location.
- [x] Run `nao check`.
