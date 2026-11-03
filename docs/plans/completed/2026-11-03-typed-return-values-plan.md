# What behavior will be added?

Functions may declare a result after their parameter list, before effect clauses:
`fun greeting(name: string) -> string { return "Hello ${name}!"; }`.
Supported result types are `string`, `bool`, and `()`. Omitting the annotation
means unit. Return types are explicit rather than inferred from function bodies.

# What rules apply to returns and execution?

- `return expression;` evaluates and returns a value; `return;` returns unit.
- Returns are allowed only in function bodies, not top-level scripts or test bodies.
- Every return must match its function's declared type, including unit functions.
- A non-unit function must contain a return. With the current straight-line
  statement language this prevents fallthrough; future control flow needs path analysis.
- A body's last expression statement is not an implicit return.
- Return exits only the current function and skips subsequent statements at runtime.
- All statements remain statically checked, even after a return; effects remain
  conservatively inferred across the full body.
- Calls carry their declaration's result type, including forward, recursive,
  imported, native, and qualified calls. Argument rules remain unchanged.
- `any` and unknown return types are rejected. Native declarations must match
  their registered implementation's return type; existing builtins still return unit.
- Module entrypoints keep their zero-argument rule. Their returned value is
  discarded during direct execution, with no exit-code conversion.
- Embedded eval returns the runtime result; execute retains only successful source.

# What will be implemented?

- [x] Add AST return annotation/type metadata and a return-statement payload.
- [x] Parse `->` result types and return statements; preserve them in formatting.
- [x] Resolve signatures before bodies, annotate calls, and validate return types,
  return placement, and non-unit fallthrough without weakening value-name checking.
- [x] Add native result signature metadata and validate it against source declarations.
- [x] Propagate return values through the interpreter and module-entrypoint execution.
- [x] Add colocated regressions for parsing, malformed syntax, format round trips,
  type errors, effects, early returns, calls across modules, and embedded evaluation.
- [x] Update the spec and overview, add executable examples and a decision record,
  and migrate statements in tests to handle the new AST variant.
- [x] Run `nao check`, inspect representative diagnostics, record results, and
  move this plan into `completed` before committing the feature.

# What assumptions and tradeoffs are intentional?

No new runtime value kinds, implicit result inference, conditional statements,
first-class functions, local variables, default arguments, or function overloading
are introduced. Use explicit return syntax and declaration-order argument evaluation.
The existing resolved AST and symbol table remain the canonical semantic model.
The earlier unresolved-value fix is already committed separately as `c16e96e`.

# How will this be verified?

Use data-driven source fixtures and colocated tests covering valid bool/string/unit
results, bare and early returns, fallthrough, wrong and unknown types, forbidden
return locations, nested and forward calls, recursive signature resolution without
executing infinite recursion, imported/qualified functions, positional parameters,
native signature mismatches, effect propagation through returned calls, and persistent
eval results. Verify formatting idempotence and parse errors, executable spec
snapshots, and the complete build/lint/Rust/spec check graph with `nao check`.

# What was implemented and verified?

AST declarations retain explicit result spelling and the resolved result index.
Signature indexing resolves result types before any body is analyzed; calls and
return checks read that canonical metadata. Return statements carry an optional
expression, with bare returns producing unit. The interpreter propagates normal
return values separately from errors and stops the current invocation only.
Module execution explicitly discards its entrypoint's result. Eval's input guard
continues to require one expression, now excluding the new return statement variant.

Colocated regressions cover string, bool, and unit results; default unit behavior;
forward and recursive signatures; imported and qualified calls; nested-call argument
evaluation order; early returns; missing, misplaced, and mismatched returns;
unknown and any result types; native declaration/signature mismatches; formatting
round trips and malformed syntax; unreachable static errors and effects; and
persistent embedded sessions after rejected source. No new value kind or dependency
was needed. DR-014 records the accepted design.

`nao check` passed, including build, formatting, clippy, Rust tests, and all 66
examples across 18 spec chapters. Representative CLI diagnostics for fallthrough,
wrong result types, and script returns were inspected and included as executable
spec snapshots. `git diff --check` and local documentation link checks passed.
An isolated REPL smoke test returned a typed string, reported an invalid typed
definition, and then successfully called the earlier retained function again.
No planned work remains; future branching requires path-sensitive return analysis.
