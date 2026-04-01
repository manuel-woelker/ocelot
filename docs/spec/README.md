# What lives in `docs/spec`?

`docs/spec` is the canonical home for the `ocelot` language specification.
Each numbered chapter describes one focused part of the language and includes short examples.

# How are spec chapters numbered?

Spec chapters use a two-part numeric prefix:

```text
NN.MM Topic.md
```

The numbering is intended to mean:

- `NN`: a major chapter such as expressions, statements, types, declarations, modules, effects, or runtime behavior
- `MM`: a subchapter within that major chapter

# What top-level chapter numbers are planned?

The exact outline may evolve, but the current intended top-level numbering is:

- `01`: Expressions
- `02`: Statements
- `03`: Types
- `04`: Declarations
- `05`: Functions
- `06`: Modules
- `07`: Effects
- `08`: Runtime behavior
- `09`: Standard library

# How should examples be written?

Each example should use a visible heading, exactly one `ocelot` fenced block, and an explicit `### Output` section.

Example:

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

This keeps the spec readable for humans and makes it realistic to extract examples with a future Markdown-driven conformance harness.

# What chapters exist today?

No numbered spec chapters are checked in yet.

The directory structure and example conventions are in place so real chapters can be added later without redesigning the format.
