# Why separate scripts from module files now?

The current module system already gives `.ocelot` files two different effective roles:

- the selected entry file executes top-level statements as a script
- every other loaded file mainly contributes declarations such as functions and tests, and may now also be executed through a conventional `main()` function

That split is useful, but it is still implicit.
As a result, non-entry module files can contain top-level statements even though those statements do not execute automatically.
That is a poor UX because the source looks executable but is silently inert.

# What is the recommended way to distinguish scripts from modules?

The recommended approach is to distinguish them by file extension so the role is visible directly in the filename.

Recommended rule:

- module files use the existing `.ocelot` extension
- script files use a separate executable extension `.ocelot-script`
- only script files may contain top-level statements
- module files may contain declarations such as functions and tests, but not ordinary top-level statements
- the engine discovers only `.ocelot` files when loading modules around a script entrypoint
- when the user runs a `.ocelot` module directly, the engine resolves and executes that module's `main()` function

This makes the distinction obvious in editors, directory listings, code review, and documentation examples.
It also avoids the current confusing situation where a non-entry file can look script-like even though its statements never run, while still preserving an ergonomic way to execute a module directly.

# Why prefer a file extension over an inferred compilation role?

An explicit extension is a better fit for the stated goal because the distinction is visible before the file is opened.

Advantages of the extension-based design:

- readers can tell whether a file is executable or declaration-only from the filename
- tooling can apply script-specific and module-specific behavior without guessing
- it reduces ambiguity in examples and future project layouts
- it makes the statement restriction feel natural instead of implicit
- it supports two execution styles cleanly: top-level script execution and module entrypoint execution

Tradeoffs to accept explicitly:

- scripts and modules no longer share the exact same filename convention
- running a module file directly now depends on a conventional `main()` function instead of top-level statements
- existing multi-file examples and tests will need renaming if they currently use one extension for everything

This extension split is slightly asymmetric, but it is pragmatic:

- modules keep the shorter and more common extension because they are the reusable building blocks
- scripts get the more explicit extension because top-level statement execution is the exceptional behavior worth calling out

# What implementation shape keeps this change simple?

Use one shared AST shape for source files, then validate extension-specific constraints after parsing.

Recommended implementation outline:

1. Introduce an explicit source-file kind concept such as `Script` and `Module`.
2. Derive that kind from the file extension during discovery and loading.
3. Keep parsing into the existing top-level item model so the parser remains mostly unchanged.
4. Add a validation step that reports an error when a module file contains `ItemKind::Statement`.
5. Continue allowing test items in module files unless the language wants tests separated too.
6. Add module-entrypoint execution for directly run `.ocelot` files by resolving `main()` in the selected module.
7. Update the runtime and docs to describe module files as declaration-oriented and statement-free, but still runnable through `main()`.

This is simpler than building separate AST roots immediately, while still making the language behavior explicit.

# What user-visible behavior should this plan define?

This plan should establish the following semantics:

- executing `foo.ocelot-script` makes `foo.ocelot-script` the entry script in that run
- sibling and nested module files are loaded from `.ocelot` files under the execution root
- a module file with a top-level statement produces a compile-time error
- function and test declarations in module files remain valid
- the entry script may still define functions in addition to top-level statements
- module-qualified names are still derived from relative paths, with the module extension removed
- executing `foo.ocelot` directly resolves the module defined by `foo.ocelot`, finds its `main()` function, and invokes it
- running a module file without a `main()` function produces a compile-time or startup error with a dedicated diagnostic

Recommended runtime rule:

- `ocelot path/to/tool.ocelot-script` executes that file's top-level statements
- `ocelot path/to/tool.ocelot` executes `tool::main()` if the module defines it

This avoids an awkward "modules are never runnable" limitation while keeping module files free of statement syntax.

Recommended diagnostic direction:

- report that top-level statements are only allowed in the entry script
- mention the offending module path in the error
- point at the first invalid top-level statement span

# What implementation order keeps the work manageable?

1. Update the language spec to describe the distinction between the entry script and loaded module files.
2. Document the extension split: `.ocelot` for modules and `.ocelot-script` for executable scripts.
3. Add or rename internal concepts so the engine can track script and module files explicitly.
4. Update CLI and engine entrypoint selection so `.ocelot-script` runs top-level statements and `.ocelot` runs module `main()`.
5. Update module discovery so script or module execution loads `.ocelot` module files and does not treat sibling `.ocelot-script` files as modules.
6. Add a validation pass or resolver check that rejects top-level statements in module files.
7. Add resolution and runtime support for invoking a module's `main()` function when a `.ocelot` file is executed directly.
8. Update engine tests to cover successful script execution, successful module `main()` execution, missing `main()` failures, and failure for statements in module files.
9. Update fixtures, examples, and spec validation inputs to use the new extension split.
10. Run `nao check`.

# What verification should this work include?

Verification should include colocated tests for:

- executing an entry file that contains top-level statements still works
- loading a sibling module file with only functions still works
- loading a sibling module file with a top-level statement reports a compile-time error
- loading a nested module file with a top-level statement reports a compile-time error with the correct file path
- test items in non-entry module files remain discoverable and do not trigger the statement restriction
- module discovery ignores `.ocelot-script` files when looking for sibling modules
- module names are derived correctly after removing the module extension
- executing a `.ocelot` module file directly invokes its `main()` function
- executing a `.ocelot` module file without `main()` reports a dedicated error
- executing a `.ocelot` module file with a non-callable or ambiguous `main` reports a dedicated error
- spec validation examples continue to pass under the new rule
- running `nao check`

# What assumptions and open questions should stay explicit?

- This plan assumes modules keep `.ocelot` and executable scripts move to `.ocelot-script`.
- This plan assumes tests are declarations, not ordinary executable top-level statements.
- This plan assumes the restriction should be enforced after parsing rather than by splitting the parser into two grammars immediately.
- This plan assumes `main()` is the conventional module entrypoint name and does not require a special annotation.
- Open question: should module `main()` be required to have a specific signature, such as zero arguments and no return value, in this slice?
- Open question: if a `.ocelot-script` file also defines a `main()` function, should that have any special meaning or remain an ordinary function?
- Open question: should spec examples keep using visible filenames with mixed extensions, or should there be a higher-level notation for executable file versus module file?

# What concrete tasks should track this plan?

- [ ] Document the script-versus-module distinction in `docs/spec`.
- [ ] Document `.ocelot` for modules and `.ocelot-script` for executable scripts.
- [ ] Introduce an explicit internal source-file kind for scripts and modules.
- [ ] Update CLI and engine entrypoint selection for script files versus module files.
- [ ] Update module discovery to load the module extension.
- [ ] Reject top-level statements in non-entry module files with a source diagnostic.
- [ ] Execute `.ocelot` files by resolving and invoking module `main()`.
- [ ] Rename or update examples, fixtures, and spec-validation inputs to use the module extension.
- [ ] Add or update engine, resolver, and spec-validation tests for the new restriction.
- [ ] Run `nao check`.
