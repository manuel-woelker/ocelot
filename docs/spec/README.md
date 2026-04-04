# What lives in `docs/spec`?

`docs/spec` is the canonical home for the `ocelot` language specification.
Each numbered chapter describes one focused part of the language and includes short examples.

# How are spec chapters numbered?

Spec chapters use a two-part numeric prefix:

```text
NN.MM Topic.md
```

The numbering is intended to mean:

- `NN`: a major chapter such as lexical structure, expressions, statements, types, declarations, modules, effects, or runtime behavior
- `MM`: a subchapter within that major chapter

# What top-level chapter numbers are planned?

The exact outline may evolve, but the current intended top-level numbering is:

- `01`: Lexical structure
- `02`: Expressions
- `05`: Statements
- `10`: Types
- `15`: Declarations
- `20`: Functions
- `25`: Modules
- `27`: Effects
- `28`: Runtime behavior
- `30`: Standard library
- `91`: Lexer errors

# How should examples be written?

Each example should use a visible heading, one or more named `ocelot` fenced blocks, and exactly one explicit expectation section.

Use:

- a visible filename label such as `main.ocelot-script:` or `helper.ocelot:` immediately before each `ocelot` block
- `### Output` for examples that should execute successfully
- `### Error` for examples that should fail with a stable error message

Example:

~~~markdown
## Example: integer addition

main.ocelot-script:

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

- [01.01 Lexical structure - Comments](./01.01%20Lexical%20structure%20-%20Comments.md)
- [02.01 Expressions - Function calls](./02.01%20Expressions%20-%20Function%20calls.md)
- [02.02 Expressions - Prefix negation](./02.02%20Expressions%20-%20Prefix%20negation.md)
- [10.01 Types - Booleans](./10.01%20Types%20-%20Booleans.md)
- [15.01 Declarations - Test items](./15.01%20Declarations%20-%20Test%20items.md)
- [15.02 Declarations - Function definitions](./15.02%20Declarations%20-%20Function%20definitions.md)
- [25.01 Modules - File modules](./25.01%20Modules%20-%20File%20modules.md)
- [28.01 Runtime behavior - Scripts](./28.01%20Runtime%20behavior%20-%20Scripts.md)
- [28.02 Runtime behavior - Test items](./28.02%20Runtime%20behavior%20-%20Test%20items.md)
- [30.01 Standard library - println](./30.01%20Standard%20library%20-%20println.md)
- [30.02 Standard library - assert](./30.02%20Standard%20library%20-%20assert.md)
- [91.01 Lexer errors - Unterminated strings](./91.01%20Lexer%20errors%20-%20Unterminated%20strings.md)
- [91.02 Lexer errors - Unterminated block comments](./91.02%20Lexer%20errors%20-%20Unterminated%20block%20comments.md)

Additional chapters can be added later without redesigning the format.
