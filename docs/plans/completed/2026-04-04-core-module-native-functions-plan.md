# Why replace seeded builtins with a core module?

`ocelot` currently seeds builtin effects, builtin types, and native functions directly inside [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs).
That gets the implementation moving, but it is too magical now that the language has real modules, imports, function declarations, and effect declarations.

The current model has a few problems:

- builtin functions such as `println`, `assert`, and `assert_eq` do not exist as ordinary declarations in source
- builtin effects such as `write_stdout` and `panic` appear implicitly instead of being declared in one obvious place
- the `any` type already exists globally even though the user-visible reason for it is tied to native entrypoints like `println`
- native registration is partly split between AST/runtime tables and hardcoded enum variants, rather than being driven by declared fully qualified function names

Moving these definitions into a dedicated core module makes the language model more honest: native things are still compiler-backed, but they are declared in a module-shaped way instead of appearing from nowhere.

# What module name should this plan recommend?

The recommended module name is `core`.

Why `core` instead of something fancier:

- it reads cleanly in qualified names such as `core::println`
- it works well as an always-available fallback module without overloading the name itself with `prelude` mechanics
- it is clearer than `intrinsics`, which usually implies lower-level compiler-only operations rather than user-callable APIs like `println`
- it leaves room for a future `std` library without pretending this first slice already has one

Recommended first examples:

- `core::println`
- `core::assert`
- `core::assert_eq`
- core effects `core::write_stdout` and `core::panic`

This plan recommends that `core` itself is auto-imported as a fallback layer.
That keeps ordinary calls ergonomic without pretending the declarations are not part of a real module.

# What user-visible behavior should this slice define?

This plan should establish the following semantics:

- a dedicated `core` module declares compiler-provided effects and native functions
- functions may be prefixed with `native`
- a `native fun ...` declaration must omit its body
- non-native functions must continue to require a body
- native function implementations are registered in the compiler by fully qualified function name
- resolution treats native functions as ordinary declared functions once the core module has been loaded
- builtin effects are declared in the core module rather than injected as free-floating names
- the `core` module is auto-imported into every file as a fallback, so `println()`, `assert()`, and `assert_eq()` continue to work without explicit `use` items
- `any` exists to support native function signatures such as `core::println(value: any)`
- `any` may only appear in native function declarations

Recommended scope limits for this slice:

- no user-defined native functions outside the compiler-provided core module yet
- no general-purpose top type semantics for `any` outside native boundaries
- no overloading or variadic native functions
- no general prelude mechanism beyond auto-importing the `core` module

# What current implementation gaps does this work need to close?

Today the implementation still relies on direct seeding:

- [`ProgramEnvironment::new()`](/data/projects/ocelot/crates/ast/src/program_environment.rs) seeds `write_stdout`, `panic`, `any`, `string`, `bool`, and native functions
- [`FunctionDefinition::native()`](/data/projects/ocelot/crates/ast/src/function_definition.rs) creates compiler-backed definitions without a matching source declaration
- [`NativeFunction`](/data/projects/ocelot/crates/ast/src/native_function.rs) is keyed only by enum variants, not by declared fully qualified names
- the function-definition spec currently requires every function to have a block body
- the effects spec explicitly says builtin effects exist without declarations
- `any` already exists in [`TypeKind`](/data/projects/ocelot/crates/ast/src/type_kind.rs), but there is no parser or resolver rule that limits its use to native signatures

That means the work is not just parser sugar.
It needs a deliberate shift from seeded declarations to a compiler-supplied core module source or equivalent module-loading path.

# How should core declarations be modeled?

Recommended approach:

- store the core module as one checked-in internal `.ocelot` source file that the compiler loads as a synthetic module
- declare builtin effects with ordinary `effect` declarations inside that module
- declare builtin functions with `native fun ...;` syntax inside that module
- register runtime implementations in a native-function registry keyed by fully qualified function name such as `core::println`

One pragmatic shape is:

- keep builtin scalar types like `string` and `bool` seeded in the type table for now, because they are language primitives rather than library declarations
- stop seeding builtin effects and builtin functions directly into the environment
- keep the source file in an internal location such as `crates/engine/resources/core.ocelot`
- load that file as a compiler-provided synthetic core module before user modules are registered so its declarations are available for imports and resolution
- make core functions available as fallback resolution candidates before resolving user items
- resolve each native declaration against the compiler registry after function registration and report an internal/compiler error if a declared builtin has no implementation

This keeps the language surface module-based without forcing primitive types to masquerade as library items.
It also gives the project one authoritative `.ocelot` source file that doubles as implementation-adjacent documentation for the language-provided surface area.

# How should syntax and AST support native functions?

Recommended parser and AST changes:

- reserve `native` as a keyword
- extend function items with an `is_native` flag or a dedicated native-function item representation
- require `native fun name(...) ...;` to end with `;` and have no body
- continue to parse ordinary `fun name(...) { ... }` definitions unchanged
- reject `native` functions with bodies
- reject non-native functions without bodies

The cleanest first syntax is:

```ocelot
native fun println(value: any) can write_stdout;
```

That keeps effect declarations on native functions aligned with ordinary function headers while making the missing body explicit in syntax instead of by convention.

# How should the compiler resolve native implementations?

Recommended design:

- replace implicit enum-only registration with a registry from fully qualified function name to native implementation descriptor
- keep the existing runtime dispatch enum or function pointer mechanism behind that registry if it remains convenient internally
- after core and user functions are registered, resolve each native declaration to a registry entry
- store the resolved native implementation handle in [`FunctionDefinition`](/data/projects/ocelot/crates/ast/src/function_definition.rs) so the interpreter can dispatch without string lookups at runtime
- use `core` functions only as the final fallback for unqualified calls, after local declarations and explicit imports have both failed to resolve

Recommended registry entries should carry:

- the fully qualified name, for example `core::println`
- the implementation handle used by the interpreter
- any invariants that must match the declaration, such as expected arity

Matching by fully qualified name is the important shift here.
It makes native linkage explicit and compatible with the module system.

# How should `any` be constrained?

`any` should remain a real type-table entry, but the resolver should enforce where it may appear.

Recommended rules:

- `any` is legal only in native function parameter types for this slice
- user-defined functions may not declare parameters of type `any`
- if return types are introduced later, `any` should remain restricted there too unless the language intentionally broadens it
- ordinary expressions should not infer to `any`
- calls to native functions typed with `any` should accept arguments of any currently resolvable type

For the immediate `println` goal, this keeps the type system simple:

- `core::println(any)` can accept `string` and `bool` without special overloads
- `assert_eq(any, any)` can keep broad equality coverage while the type system is still small
- users cannot start threading `any` through their own APIs and silently erase type checking

# How should core effects fit into the module model?

The core module should declare builtin effects the same way user modules declare nominal effects.

Recommended behavior:

- `effect write_stdout;` and `effect panic;` live in the `core` module
- effect clauses on builtin native functions refer to those declared effects
- user code should reference builtin effects by whatever effect-name form the language already permits for cross-module effects

One design point needs to stay explicit:

- if effect names are still globally resolved today, this slice may need to preserve that temporarily for compatibility
- if effect names are meant to become module-qualified, builtin effects should move toward `core::write_stdout` and `core::panic`

The implementation plan should avoid baking in more global-effect magic than necessary.

# What implementation order keeps this work manageable?

1. Document the core-module model in new or updated spec chapters for function declarations, effects, types, and standard-library items.
2. Add parser support for the `native` keyword and bodyless native function declarations.
3. Extend the AST and function-definition model to represent declared native functions distinctly from user-defined functions.
4. Add a checked-in internal `core.ocelot` source file and load it as a compiler-provided synthetic module named `core`.
5. Stop seeding builtin functions and builtin effects directly in [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs); keep only primitive type seeding that still belongs in the core language.
6. Add a native implementation registry keyed by fully qualified function name and resolve declared native functions against it during compilation.
7. Enforce that `any` may only appear in native function signatures.
8. Update resolution and effect propagation so core effects and functions behave like ordinary declarations after the core module is loaded, while preserving `core` as a fallback-only lookup tier.
9. Update interpreter dispatch to execute resolved native implementations through the new registry-backed handle.
10. Add or update parser, resolver, engine, interpreter, and spec-validation tests.
11. Run `nao check`.

# What verification should this work include?

Verification should include colocated tests for:

- parsing `native fun println(value: any) can write_stdout;`
- rejecting a native function declaration with a body
- rejecting a non-native function declaration without a body
- loading the core module before user-module resolution
- parsing and resolving the checked-in `core.ocelot` file as the authoritative declaration source for native functions and effects
- resolving `core::println`, `core::assert`, and `core::assert_eq` through declared native functions rather than seeded environment entries
- preserving unqualified `println()`, `assert()`, and `assert_eq()` calls through fallback lookup into the auto-imported `core` module
- proving local declarations and explicit imports both outrank fallback lookup into `core`
- reporting an internal/compiler error when a declared native function has no registry implementation
- enforcing that `any` is accepted in native signatures and rejected in user-defined function signatures
- preserving `println`, `assert`, and `assert_eq` runtime behavior through native dispatch
- propagating core effects through declared native functions
- validating updated spec examples and docs
- running `nao check`

# What assumptions and open questions should stay explicit?

- This plan assumes the core module is loaded from a checked-in internal source file rather than from the user's module search path. That keeps startup deterministic and avoids bootstrapping problems while still keeping the declarations visible in real `ocelot` syntax.
- This plan assumes auto-importing `core` as a fallback tier is the right compatibility and ergonomics choice for early language development.
- This plan assumes primitive types like `string` and `bool` should remain language-level seeded types rather than being declared in a library module.
- This plan assumes `any` is a narrowly scoped escape hatch for native signatures, not the start of a dynamic type system.
- Open question: should fallback lookup into `core` apply only to functions for now, or should future core-provided names participate in the same mechanism?
- Open question: should core effects become fully qualified in source, or should effect names remain globally unique for now even though they originate in a module?
- Open question: should the parser allow `native fun` only inside the compiler-provided core module, or should user code be allowed to write such declarations but receive a resolver error because no implementation registry entry exists?
- Open question: should `assert_eq(any, any)` continue accepting all pairs immediately, or should native-call argument compatibility still reject obviously incomparable future types once more types exist?

# What landed from this plan?

This slice landed a checked-in compiler-provided `core` module plus declared native functions:

- the lexer now reserves `native`
- function parsing now supports bodyless `native fun ...;` declarations and rejects native bodies
- [`FunctionItem`](/data/projects/ocelot/crates/ast/src/function_item.rs) now tracks whether a declaration is native
- [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) now seeds primitive types plus a fully qualified native implementation registry instead of seeding builtin functions and effects directly
- the engine now loads [`core.ocelot`](/data/projects/ocelot/crates/engine/resources/core.ocelot) as a compiler-provided synthetic module before user modules
- the resolver now links native declarations against the fully qualified native registry, rejects native declarations outside `core`, and rejects user-defined `any` parameters
- unqualified function resolution now treats `core` as a fallback tier after local declarations and explicit imports
- the engine now rejects user-defined modules named `core`
- the spec now documents the `core` module, `native fun`, and the constrained `any` type
- `cargo test -p ocelot-ast`
- `cargo test -p ocelot-parser`
- `cargo test -p ocelot-resolver`
- `cargo test -p ocelot-interpreter`
- `cargo test -p ocelot-engine`
- `cargo test -p ocelot-spec-validation`
- `cargo run -p ocelot-spec-validation`
- `nao check`

# What concrete tasks should track this plan?

- [x] Add or update spec chapters to describe core-module declarations, `native fun`, builtin effects, and the constrained `any` type.
- [x] Reserve `native` in the lexer.
- [x] Extend function parsing and AST modeling for bodyless native declarations.
- [x] Add a checked-in internal `core.ocelot` file and load it as a synthetic `core` module in the engine/compiler loading pipeline.
- [x] Add fallback resolution for auto-imported `core` functions after local declarations and explicit imports.
- [x] Stop seeding builtin functions and builtin effects directly in [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs).
- [x] Add a fully-qualified native implementation registry and link declared native functions against it.
- [x] Enforce that `any` may only appear in native function signatures.
- [x] Update resolver and interpreter behavior to use declared core functions/effects.
- [x] Add or update parser, resolver, engine, interpreter, and spec-validation tests.
- [x] Run `nao check`.
