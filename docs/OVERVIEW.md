# What is `ocelot`?

`ocelot` is an experimental statically typed programming language implemented in Rust.
The current interpreter supports strings, booleans, template strings, typed function
parameters, required argument labels with explicit positional opt-in, nominal effects,
file modules, imports, and test declarations.
Functions support declared string, boolean, and unit return types, block result
expressions, and explicit `return` statements. A final expression without a semicolon
supplies a block's value. An omitted function result annotation still means unit.

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
[Decision records](decisions/README.md) explain significant choices and their tradeoffs.

# Where is the architecture rationale?

Start with the decisions on the [PAL](<decisions/DR-001 Isolate platform interactions behind the PAL.md>),
[canonical semantic model](<decisions/DR-003 Resolve programs into one canonical semantic model.md>),
[filesystem modules and scripts](<decisions/DR-004 Use filesystem modules and explicit script extensions.md>),
and [embedded sessions](<decisions/DR-009 Reuse source backed embedded sessions for the REPL.md>).
The [argument-label decision](<decisions/DR-011 Require argument labels with explicit positional opt in.md>)
records the latest public calling-convention tradeoffs.
The [block-value decision](<decisions/DR-015 Use block values before expression conditionals.md>)
explains body results, declared types, and early-return semantics.

# What principles guide the project?

The goals in `README.md` are to make programming fun, friendly, and fast.
They apply to the language, its diagnostics, and the contributor feedback loop.
