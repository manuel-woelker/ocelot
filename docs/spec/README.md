# Where should I start?

This directory specifies the implemented Ocelot language through rules and executable
examples. Start with file execution, then functions and expressions. Use the table
below as the reference index; unimplemented features belong in plans rather than
in the language rules.

| Topic | Rules owned by this chapter |
| --- | --- |
| [Comments](./01.01%20Lexical%20structure%20-%20Comments.md) | Placement, nesting, and unterminated comments |
| [Strings](./01.02%20Lexical%20structure%20-%20Strings.md) | Literal delimiters and unterminated strings |
| [Function calls](./02.01%20Expressions%20-%20Function%20calls.md) | Argument counts, types, and call results |
| [Prefix negation](./02.02%20Expressions%20-%20Prefix%20negation.md) | Boolean negation and precedence |
| [Template strings](./02.03%20Expressions%20-%20Template%20strings.md) | Interpolation and evaluation order |
| [Blocks](./02.04%20Expressions%20-%20Blocks.md) | Tail values, semicolons, and nested returns |
| [Booleans](./10.01%20Types%20-%20Booleans.md) | Boolean literals |
| [Any](./10.02%20Types%20-%20Any.md) | Native signature escape hatch |
| [Tests](./15.01%20Declarations%20-%20Test%20items.md) | Declarations and behavior during normal execution |
| [Function definitions](./15.02%20Declarations%20-%20Function%20definitions.md) | Syntax, parameters, and declaration scope |
| [Return values](./20.01%20Functions%20-%20Return%20values.md) | Explicit result types, returns, and fallthrough checks |
| [File modules](./25.01%20Modules%20-%20File%20modules.md) | Module discovery, paths, and exported functions |
| [Imports](./25.02%20Modules%20-%20Imports.md) | File-local imports and import errors |
| [Core module](./25.03%20Modules%20-%20Core%20module.md) | Builtin names and lookup precedence |
| [Effects](./27.01%20Effects%20-%20Nominal%20effect%20declarations.md) | Declarations, propagation, and prohibitions |
| [File execution](./28.01%20Runtime%20behavior%20-%20Scripts.md) | Script statements and module entrypoints |
| [Embedded evaluation](./28.03%20Runtime%20behavior%20-%20Embedded%20eval.md) | Persistent sessions, results, and failures |
| [println](./30.01%20Standard%20library%20-%20println.md) | Printed output |
| [assert](./30.02%20Standard%20library%20-%20assert.md) | Boolean assertions |

# How are chapters organized?

Filenames use `NN.MM Topic.md`: `NN` groups a topic and `MM` orders chapters
within it. Gaps are intentional; existing chapters need not be renumbered when
others are merged. Each rule has one canonical home, with links from related topics.

# How do I write an executable example?

Use `## Example: description`, labeled `ocelot` fences, and exactly one expectation
heading followed by a `text` fence. Sources must match the Ocelot formatter's output;
examples demonstrating parse errors are exempt when they cannot be parsed.

| Mode | Source labels | Expectation |
| --- | --- | --- |
| File execution | `main.ocelot-script:` or `main.ocelot:`, plus optional helper files | `### Output` or `### Error` |
| Embedded evaluation | `execute:` and `eval:`, plus optional helper files | `### Eval result` or `### Error` |

File examples must declare exactly one of the two main filenames. Embedded examples
must include at least one `eval:` block. Steps share one session within an example;
separate examples are isolated. An eval-result expectation checks the last returned
value, not printed output. An error expectation checks normalized error text,
including diagnostic excerpts where rendered. Keep full diagnostic expectations
until the validator explicitly supports another format.

~~~markdown
## Example: print a greeting

main.ocelot-script:

```ocelot
println("hello");
```

### Output

```text
hello
```
~~~

An empty `text` block under `### Output` asserts successful execution with no output.
Use one example per distinct rule; avoid repeating introductory examples across chapters.
Keep question headings for explanatory sections and the exact headings above for
machine-readable examples.

# How are examples checked?

`crates/spec_validation` extracts examples from numbered chapters and executes them
through the engine. Run the harness directly with:

```bash
cargo run --package ocelot-spec-validation --bin ocelot-spec-validation
```

Run `nao check` for formatting, build, lint, tests, and spec validation together.
