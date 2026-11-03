# DR-004: Use filesystem modules and explicit script extensions

- Status: Accepted
- Date: 2026-11-03
- Record: Retrospective; the date records acceptance of this account, not the original implementation date.

## What is the decision?

Map `.ocelot` files to module names relative to the execution root, joining
path segments with `::`. Eagerly load module files beneath the entry directory.
Export module functions and support file-local single and grouped `use` imports.
Distinguish `.ocelot-script` top-level execution from declaration-only `.ocelot`
files; direct module execution calls a module-local, zero-argument `main()`.

## Why was this chosen?

Filesystem-shaped names provide a small namespace model without package
configuration. Distinct extensions make executable top-level behavior visible
and prevent non-entry module statements from looking executable while remaining inert.

## What is the context?

The module, import, and script-separation plans established these choices.
The current spec separates module discovery and lookup from file startup.
Later entrypoint validation enforces zero parameters for direct module execution.

The linked implementation and documentation establish the choice. Rationale and
alternative comparisons are retrospective assessment, not a verbatim record of
original deliberations.

- [Module rules](<../spec/25.01 Modules - File modules.md>)
- [Import rules](<../spec/25.02 Modules - Imports.md>)
- [Execution rules](<../spec/28.01 Runtime behavior - Scripts.md>)
- [Discovery and compilation](<../../crates/engine/src/compile_program.rs>)
- [Script/module tradeoffs](<../plans/completed/2026-04-04-script-module-separation-plan.md>)

## What are the consequences?

- File moves can change public module paths; eager loading can expose unrelated errors.
- Module files reject top-level statements, while normal execution skips test bodies.
- Imports do not re-export names; aliases, wildcard imports, package manifests,
  and directory index modules are not part of the current model.
- A function named `main` may have parameters when called ordinarily rather than as an entrypoint.

## What alternatives were considered?

- **One extension with an inferred executable role:** Leaves behavior dependent
  on how a file was selected rather than visible in its filename.
- **A package manifest or directory-as-module index:** Adds configuration beyond
  the current file-oriented requirements.
- **Demand-only module loading:** Changes eager discovery and diagnostic behavior;
  it would require a separate design rather than an incidental optimization.
