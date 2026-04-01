# What is `ocelot`?

`ocelot` is an experimental statically typed programming language project.
It currently focuses on building a clean Rust workspace foundation for language infrastructure and tooling.

This document reflects the current direction of the repository and will evolve as the implementation takes shape.

# What is the current goal of the repository?

The immediate goal is to create a maintainable base for the language implementation:

- shared Rust utilities in `crates/base`
- a platform abstraction layer in `crates/pal`
- repository tooling for checks, CI, and planning

That keeps the project ready for parser, compiler, runtime, or tooling crates without forcing those decisions too early.

# What principles guide the project?

The project goals are also captured in `README.md`, but the short version is:

- programming should be fun
- programming should be friendly
- feedback loops should be fast

Those values should shape both the language design and the contributor experience around it.
