# What problems does this plan address?

The initial review found unchecked unit-valued call arguments, eval accepting multiple
statements, entrypoints with unbound parameters, inconsistent filesystem roots, and stale
architecture documentation.

# What will be implemented and in what order?

- [ ] Track unit as the result type of resolved calls and reject invalid nested arguments.
- [ ] Validate eval input as exactly one expression before resolution or execution.
- [ ] Reject parameterized module entrypoints before executing their bodies.
- [ ] Preserve absolute walk roots and relative walk roots consistently; cover real filesystem execution.
- [ ] Refresh the overview and affected language specifications.
- [ ] Add colocated regression tests for each behavioral fix and run `nao check`.
- [ ] Record verification and move this plan into `completed`.

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
