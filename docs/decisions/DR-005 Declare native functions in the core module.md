# DR-005: Declare native functions in the core module

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Expose compiler-backed functions through source declarations in the reserved
`core` module and implementations registered by fully qualified name. Resolve
unqualified calls through local functions, explicit imports, then the core fallback.
Restrict native declarations and the `any` parameter type to native core signatures;
ordinary user-defined functions use supported concrete parameter types.

## Why was this chosen?

Source declarations make builtin names and effects visible to ordinary parsing
and resolution. A native boundary permits initial polymorphic operations without
introducing a general dynamic type or special-casing each builtin call in the resolver.

## What is the context?

The core-module plan replaced free-floating seeded builtin declarations.
The current core source declares println, assertions, and builtin effects; the
declaration indexer checks registered native arity and type compatibility.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Core declarations](<../../crates/engine/resources/builtin/core.ocelot>)
- [Native registration checks](<../../crates/resolver/src/declaration_indexer.rs>)
- [Core and lookup rules](<../spec/25.03 Modules - Core module.md>)
- [Restricted any type](<../spec/10.02 Types - Any.md>)
- [Core-module plan](<../plans/completed/2026-04-04-core-module-native-functions-plan.md>)

## What are the consequences?

- Core names remain directly callable with qualification even when local names shadow them.
- Native implementations may restrict runtime values beyond `any`; println currently
  accepts strings and booleans, not every runtime value.
- Labels and positional omission policy come from the source declaration.
- User-defined native bindings, overloads, and a general prelude are outside this model.

## What alternatives were considered?

- **Seed builtins without declarations:** Hides part of the callable interface
  from the same language pipeline used by ordinary functions.
- **Special-case native names at each call site:** Duplicates lookup and signature policy.
- **Make `any` a general dynamic type:** Expands the type model beyond the native boundary.
