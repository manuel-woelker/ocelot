# What is `ocelot`?

`ocelot` is an experimental statically typed programming language implemented in Rust.
The current interpreter supports strings, booleans, template strings, typed function
parameters, nominal effects, file modules, imports, and test declarations.
Functions currently return unit; return types and return statements are not implemented.

# How is the implementation organized?

- `crates/base` provides shared utilities, source locations, and diagnostics.
- `crates/pal` abstracts filesystem access, processes, clocks, and other platform behavior.
- `crates/ast` and `crates/parser` model and parse source code, including trivia for formatting.
- `crates/semantic` holds symbols, types, runtime values, and native functions.
- `crates/resolver` resolves declarations, checks arguments, and propagates effects.
- `crates/interpreter` executes resolved syntax trees.
- `crates/engine` loads module trees and coordinates execution, tests, and persistent embedded sessions.
- `crates/formatter` formats parsed source.
- `crates/cli` exposes file execution, test filtering, formatting, and a REPL.
- `crates/spec_validation` runs executable examples from `docs/spec`.

# Where is language behavior documented?

`docs/spec` describes the implemented language through prose and executable examples.
`docs/TESTING.md` documents verification expectations, and `docs/PLANS.md` defines the
workflow for implementation plans. This remains a work in progress.

The [documentation guide](README.md) links these references and
[development learnings](learnings/README.md), which preserve reusable findings
with their context, evidence, and sources.

# What principles guide the project?

The goals in `README.md` are to make programming fun, friendly, and fast.
They apply to the language, its diagnostics, and the contributor feedback loop.
