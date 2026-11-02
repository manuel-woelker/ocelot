# What problems does this plan address?

The initial review found unchecked unit-valued call arguments, eval accepting multiple
statements, entrypoints with unbound parameters, inconsistent filesystem roots, and stale
architecture documentation.

# What will be implemented and in what order?

- [x] Track unit as the result type of resolved calls and reject invalid nested arguments.
- [x] Validate eval input as exactly one expression before resolution or execution.
- [x] Reject parameterized module entrypoints before executing their bodies.
- [x] Preserve absolute walk roots and relative walk roots consistently; cover real filesystem execution.
- [x] Refresh the overview and affected language specifications.
- [x] Add colocated regression tests for each behavioral fix and run `nao check`.
- [x] Record verification and move this plan into `completed`.

# What assumptions and risks apply?

All currently supported functions return unit. This work does not introduce return syntax
or expose unit as a parameter type. Eval rejects declarations and multiple statements;
the REPL continues to use execute for statement input. Only execution as a module entrypoint
requires a zero-argument main; ordinary functions named main may have parameters.
Directory walks should return paths in the same absolute/relative coordinate system as
their requested root. Existing relative-path behavior must remain compatible.

# How will the changes be verified?

Use colocated tests for nested calls, eval rejection without side effects and retained
session state, entrypoint arity, and real filesystem walks and engine execution under
an absolute temporary root. Run `nao check`, including specification validation, and
repeat the original CLI reproductions after rebuilding.

# What was verified?

`nao check` passed after implementation, including build, formatting, clippy,
colocated tests, and executable specification validation. CLI reproductions now
reject the invalid nested argument before printing, reject the parameterized
entrypoint explicitly, and report malformed multi-statement REPL input instead
of silently discarding it. Valid multi-statement REPL input executes once.
Real filesystem regressions verify module execution and embedded eval under an
absolute temporary root, as well as absolute and relative directory walks.
The overview and repository guidance now describe the implemented workspace.

No planned work remains. General return types and unknown-identifier checking
remain outside this plan.
