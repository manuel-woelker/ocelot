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
- `91`: Lexer errors

# How should examples be written?

Each example should use a visible heading, exactly one `ocelot` fenced block, and exactly one explicit expectation section.

Use:

- `### Output` for examples that should execute successfully
- `### Error` for examples that should fail with a stable error message

Example:

~~~markdown
## Example: integer addition

```ocelot
println("hello");
```

### Output

```text
hello
```
~~~

This keeps the spec readable for humans and makes it realistic to extract examples with a future Markdown-driven conformance harness.

# What chapters exist today?

The first numbered spec chapters are:

- [04.01 Declarations - Test items](./04.01%20Declarations%20-%20Test%20items.md)
- [08.01 Runtime behavior - Scripts](./08.01%20Runtime%20behavior%20-%20Scripts.md)
- [08.02 Runtime behavior - Test items](./08.02%20Runtime%20behavior%20-%20Test%20items.md)
- [09.01 Standard library - println](./09.01%20Standard%20library%20-%20println.md)
- [91.01 Lexer errors - Unterminated strings](./91.01%20Lexer%20errors%20-%20Unterminated%20strings.md)

Additional chapters can be added later without redesigning the format.
