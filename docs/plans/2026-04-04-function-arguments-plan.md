# Why are typed function arguments needed now?

User-defined functions currently stop at the zero-argument slice.
The parser accepts only `fun name() { ... }`, the resolver only checks call argument types against native function metadata, and the interpreter rejects any arguments passed to a user-defined function.

That leaves an obvious hole in the language surface:

- functions cannot accept inputs
- call sites cannot be checked for arity against user-defined declarations
- identifiers inside function bodies cannot refer to parameters
- the existing `FunctionDefinition.argument_types` field is underused for source-defined functions

Adding a first slice of typed function arguments now keeps the function model coherent before more features pile on top of the zero-argument limitation.

# What should this slice include?

This slice should include:

- parsing parameter lists on function items, for example `fun greet(name: string) { ... }`
- requiring an explicit type annotation for every parameter
- supporting only the currently available primitive parameter types: `string` and `bool`
- recording parameter names and parameter types in the AST and shared function metadata
- resolving identifiers that refer to parameters inside the function body
- validating user-defined function call arity in the resolver
- validating user-defined function call argument types in the resolver
- executing user-defined function calls by evaluating arguments in the caller and copying the resulting values into the callee environment
- spec and example updates for function definitions and function calls

This slice should not yet include:

- default arguments
- named arguments
- variadics
- return values
- mutable locals beyond parameter bindings
- nested functions or closures

# What is the current implementation gap?

Today:

- [`Parser::parse_function_item()`](/data/projects/ocelot/crates/parser/src/parser.rs) requires `fun name()` and rejects any parameter syntax
- [`FunctionItem`](/data/projects/ocelot/crates/ast/src/function_item.rs) stores only the function name, effect clauses, body, and span
- [`FunctionDefinition::user_defined()`](/data/projects/ocelot/crates/ast/src/function_definition.rs) always initializes `argument_types` to an empty list
- the resolver can already validate call argument types against [`FunctionDefinition.argument_types`](/data/projects/ocelot/crates/ast/src/function_definition.rs), but it does not currently populate that metadata for user-defined functions or enforce user-defined arity
- identifier expressions inside function bodies still fail at runtime as unresolved identifiers
- [`Interpreter::evaluate_user_defined_call()`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) rejects all arguments with `user-defined functions do not accept arguments yet`

So the project has most of the plumbing for typed call checking, but it still lacks the declaration syntax, parameter metadata, and local binding model that make user-defined arguments real.

# What AST and function metadata should represent parameters?

The implementation should add an explicit parameter node instead of storing raw tuples on functions.

Recommended shape:

- add a dedicated AST type such as `FunctionParameter`
- store:
  - the parameter identifier
  - the declared type name or resolved `TypeIndex`
  - the full parameter span
- extend [`FunctionItem`](/data/projects/ocelot/crates/ast/src/function_item.rs) with `parameters: Vec<FunctionParameter>`
- extend [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) so user-defined functions copy their declared parameter types into `argument_types`

This keeps function declarations readable in the tree and lets the resolver reuse the existing function-signature metadata instead of inventing a second source of truth.

# How should function parameter syntax parse?

The parser should continue to treat parameters as part of the function declaration header, but it should now accept one or more typed parameters inside the parentheses.

Recommended syntax rules:

- parameters are comma-separated
- each parameter is `name: type`
- parameter names are identifiers
- parameter types are required
- empty parameter lists remain valid
- trailing commas should stay unsupported unless call argument lists already rely on them elsewhere

Example target syntax:

```ocelot
fun greet(name: string, excited: bool) {
  println(name);
  assert(excited);
}
```

Recommended parser work:

1. Add a dedicated parameter parser used from `parse_function_item()`.
2. Parse zero or more `identifier : identifier` entries until `)`.
3. Report stable diagnostics for:
   - missing parameter name
   - missing `:`
   - missing parameter type
   - malformed separators such as `fun greet(name: string,) {}`
4. Decide whether the parser should reject unknown type names immediately or leave that to the resolver. Leaving it to the resolver is usually cleaner because type names are semantic, not syntactic.

# How should name and type resolution change?

The resolver needs two related upgrades: user-defined signature validation and parameter-name lookup inside function bodies.

Recommended behavior:

1. During function registration, resolve each declared parameter type name through the shared type table.
2. Populate the user-defined [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) with the resolved `argument_types`.
3. Reject unknown parameter types with resolver diagnostics that point at the type annotation span.
4. Reject duplicate parameter names within the same function declaration.
5. When resolving a function body, seed a function-local value scope containing that function's parameters.
6. Resolve identifier expressions that match parameter bindings so they stop failing as runtime-only unresolved names.
7. Extend call resolution to report an arity error when the caller supplies too few or too many arguments for a user-defined function.
8. Reuse the existing argument-type validation path for user-defined functions once their signature metadata is populated.

The important constraint is that parameter bindings are local to the current function body only.
This slice should not overreach into general local variables or broader lexical scope rules.

# How should runtime argument passing work?

The runtime model should stay boring and explicit: arguments are evaluated in the caller, then copied into a fresh callee-local environment before the body executes.

Recommended behavior:

- evaluate call arguments left to right in the caller interpreter
- store the resulting runtime values in parameter order
- create a fresh local binding map for the callee invocation
- copy each evaluated value into that map under the declared parameter name
- execute the callee body against that local environment
- keep parameters immutable for this slice

This matches the user requirement directly and avoids premature complexity around references, shared mutable state, or closure capture.

# What runtime representation should local bindings use?

The interpreter currently has no local value environment, so this change should add the smallest useful one.

Recommended shape:

- extend [`Interpreter`](/data/projects/ocelot/crates/interpreter/src/interpreter.rs) with an optional or default-empty local binding map keyed by identifier name
- resolve identifier expressions by checking local bindings before producing the current unresolved-identifier runtime error
- construct child interpreter instances for user-defined calls with a fresh local binding map populated from parameters

This is enough to support parameter reads inside function bodies without committing the whole runtime to a large scope system too early.

# What diagnostics should this slice produce?

At minimum, this work should define stable diagnostics for:

- malformed function parameter syntax
- unknown parameter type names
- duplicate parameter names
- calling a user-defined function with the wrong number of arguments
- calling a user-defined function with an argument of the wrong type

Resolver diagnostics should continue to point at the most specific span:

- parameter type errors should point at the type annotation
- duplicate parameter errors should point at the later parameter and, if practical, reference the original parameter
- call arity and call type errors should point at the offending call-site argument list or argument expression

# How should the spec and examples change?

This work should update the existing declarations and expressions chapters rather than creating a disconnected parallel description.

Recommended scope:

- update [`15.02 Declarations - Function definitions`](/data/projects/ocelot/docs/spec/15.02%20Declarations%20-%20Function%20definitions.md) to describe typed parameter lists instead of empty-only parameter lists
- update [`02.01 Expressions - Function calls`](/data/projects/ocelot/docs/spec/02.01%20Expressions%20-%20Function%20calls.md) with examples that call parameterized user-defined functions
- add at least one error example covering wrong arity or wrong argument type
- make the examples executable through the existing spec validation flow

The docs should also reconcile the current `bool` versus `boolean` naming split so the public surface, seeded type table, and diagnostics all agree.

# What implementation order keeps this work manageable?

1. Add an active plan for typed function arguments.
2. Introduce AST support for function parameters.
3. Extend parser support from `fun name()` to `fun name(param: type, ...)`.
4. Add parser tests for valid and invalid parameter lists.
5. Resolve declared parameter types during function registration and populate user-defined `argument_types`.
6. Add resolver diagnostics for unknown parameter types and duplicate parameter names.
7. Add function-local parameter bindings during function-body resolution so identifier expressions can resolve to parameters.
8. Extend user-defined call resolution to enforce arity before type validation.
9. Add the minimal interpreter local-binding environment needed to evaluate parameter references.
10. Update user-defined call execution to evaluate arguments in the caller and copy them into the callee environment.
11. Add resolver, interpreter, engine, and spec-validation coverage.
12. Run `nao check`.

# What verification should this work include?

Verification should include:

- colocated parser tests for:
  - one typed parameter
  - multiple typed parameters
  - mixed zero-argument and parameterized functions
  - malformed parameter syntax diagnostics
- colocated resolver tests for:
  - resolving declared parameter types for user-defined functions
  - duplicate parameter-name failures
  - unknown parameter-type failures
  - resolving parameter references inside function bodies
  - wrong user-defined call arity failures
  - wrong user-defined call argument-type failures
- colocated interpreter tests for:
  - executing a one-parameter user-defined function
  - executing a multi-parameter user-defined function
  - proving arguments are copied by value into the callee environment
  - nested user-defined calls that pass parameters onward
- engine and spec-validation tests covering successful examples and stable error output
- running `nao check`

# What assumptions, risks, and open questions should stay explicit?

- The spec chapter for booleans already uses `bool`, but the current internal type table still seeds `"boolean"`. This plan assumes that parameter syntax should follow the spec-facing `bool` spelling and that implementation names/diagnostics will be aligned as part of this work.
- This plan assumes copied runtime values are sufficient for both supported primitive argument types. If future runtime values become reference-like, the call semantics will need to stay explicit.
- This plan assumes parameter bindings are immutable and function-local only. Adding assignment or shadowing rules in the same change would overengineer the slice.
- The resolver currently does not have a general value-resolution model. This work should add only the minimum local parameter lookup needed for function bodies rather than trying to solve full lexical name resolution.
- If call arity errors already have a preferred wording for native functions, user-defined diagnostics should align with that style instead of inventing a new voice.

# What concrete tasks should track this plan?

- [ ] Add AST support for typed function parameters.
- [ ] Extend parser support to accept `fun name(param: type, ...)` declarations and report stable parameter-syntax diagnostics.
- [ ] Populate user-defined `FunctionDefinition.argument_types` from declared parameters during resolution.
- [ ] Add resolver diagnostics for unknown parameter types and duplicate parameter names.
- [ ] Add minimal function-local parameter binding resolution inside function bodies.
- [ ] Enforce user-defined call arity and argument types in the resolver.
- [ ] Add interpreter support for evaluating call arguments in the caller and copying them into a fresh callee environment.
- [ ] Update spec chapters and executable examples for typed function parameters and calls.
- [ ] Add parser, resolver, interpreter, engine, and spec-validation tests for the new behavior.
- [ ] Run `nao check`.
