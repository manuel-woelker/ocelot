# Why is a module system needed now?

`ocelot` currently treats each `.ocelot` file as an isolated compilation unit.
That is enough for single-file examples, but it breaks down as soon as behavior should be split across files or organized into directories.

The requested first slice is intentionally small:

- every `.ocelot` file defines one module
- the module name is the file path relative to the execution root, without the `.ocelot` extension
- nested path segments are separated with `::`
- every function defined in a file is exported from that module

This gives the language a usable namespacing model without adding imports, visibility modifiers, or a package system yet.

# What user-visible behavior should this slice define?

This slice should establish the following semantics:

- `foo.ocelot` defines module `foo`
- `math/add.ocelot` defines module `math::add`
- functions are referenced by fully qualified module path plus function name, for example `math::add::sum()`
- top-level statements in the entry script still execute as today
- user-defined functions from non-entry files are available to resolution and interpretation through their module-qualified names
- all functions remain exported implicitly; there is no private visibility in this slice

Recommended scope limits:

- no `import` syntax yet
- no wildcard or aliasing support
- no module-level values
- no re-export syntax
- no directory-as-module index files

# What current implementation gaps does this work need to close?

Today the implementation assumes one source file at a time:

- the parser only sees one [`SourceFile`](/data/projects/ocelot/crates/base/src/source_file.rs)
- the resolver stores function names in [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs) as unqualified strings like `greet`
- call resolution currently assumes the callee is a single identifier, not a `::`-separated path
- the engine compiles only the entry file and has no module graph loading step
- spec validation writes exactly one virtual file named `spec-test.ocelot` for each example in [`execute_spec_example.rs`](/data/projects/ocelot/crates/spec_validation/src/execute_spec_example.rs)

That means the module feature needs both language changes and a new multi-file compilation path.

# What syntax and AST changes should carry module-qualified calls?

The parser should stop treating callable names as a single identifier token when `::` appears.

Recommended approach:

- add lexer support for `::`
- add a `QualifiedIdentifier` AST node that stores a `Vec<Identifier>`
- allow call callees to reference either:
  - an unqualified local function name in the current file
  - a fully qualified module path ending in a function name
- keep this slice restrictive by continuing to reject arbitrary expression callees

Pragmatically, the cleanest shape is likely:

- keep [`Identifier`](/data/projects/ocelot/crates/ast/src/identifier.rs) for single segments
- add a [`QualifiedIdentifier`](/data/projects/ocelot/crates/ast/src/identifier.rs) sibling type for `foo::bar::baz`
- update [`CallExpression`](/data/projects/ocelot/crates/ast/src/call_expression.rs) resolution metadata so it can point at functions defined in any loaded module

This keeps the parser honest and avoids overloading plain identifier strings with path semantics.

# How should the compiler and runtime model modules?

The implementation should introduce an explicit multi-file program model instead of pretending the entry file is the whole world.

Recommended design:

- introduce a program-level structure that owns all loaded source files plus the shared [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs)
- compute each module name from the file path relative to the execution root
- register user-defined functions under fully qualified names such as `math::add::sum`
- keep native functions unqualified for now, so `println()` stays unchanged
- resolve unqualified calls within a file against that file's own module first
- resolve qualified calls against the global module-qualified symbol table
- detect duplicate fully qualified function names as resolver errors

The engine pipeline should become:

1. load the entry file
2. eagerly load every `.ocelot` file under the execution root
3. parse all loaded files
4. register module-qualified functions
5. resolve calls across the whole loaded program
6. execute only the entry file's top-level statements or requested test body

# How should eager module loading work without overengineering it?

This should stay boring and explicit.

Recommended behavior:

- treat the directory containing the entry file as the execution root
- eagerly discover every `.ocelot` file under that root before parsing or resolving
- derive each module name from the file's relative path, for example `foo/bar.ocelot` -> `foo::bar`
- keep a normalized path-to-module mapping so duplicate or alias-like paths do not create confusing double loads
- resolve qualified calls against the already loaded program instead of triggering file I/O during resolution

This avoids building a module graph loader in the resolver and keeps module discovery deterministic for tests and spec validation.

# How should spec examples describe multiple files?

The current spec-example format only permits one anonymous `ocelot` block.
That is too narrow once examples need helper modules.

Recommended markdown format:

- allow one or more fenced `ocelot` blocks per example
- require each source block to be introduced by a visible filename label line of the form `path/to/file.ocelot:`
- keep exactly one expectation section with one `text` block

Recommended example shape:

~~~markdown
## Example: a script can call a sibling module function

main.ocelot:

```ocelot
math::greet::hello();
```

math/greet.ocelot:

```ocelot
fun hello() {
    println("hello");
}
```

### Output

```text
hello
```
~~~

This is simple to parse, keeps filenames adjacent to source content, and scales from one file to many.

# How should spec validation change to support that format?

The spec validation crate should stop storing just one source string per example.

Recommended changes:

- replace `SpecExample.source` with a small collection of named source files
- require exactly one file block named `main.ocelot` for executable examples
- teach the markdown loader to associate each `ocelot` block with the nearest preceding filename label line
- preserve strong malformed-example diagnostics for:
  - missing filename label lines
  - duplicate file names
  - missing `main.ocelot`
  - zero `ocelot` blocks
- update execution so it creates every declared file in the virtual PAL before running the engine on `main.ocelot`
- update rendered expected diagnostics so file paths in errors reflect the declared filenames instead of the hardcoded `spec-test.ocelot`

This is the minimum change that makes spec examples realistic for module features.

# What implementation order keeps this work manageable?

1. Document the module semantics in a new spec chapter under `docs/spec`, including multi-file examples.
2. Extend the lexer and parser to represent `::`-qualified call targets.
3. Add `QualifiedIdentifier`, module-path data structures, and fully qualified function naming to the AST and program environment.
4. Introduce engine support for loading and compiling multiple `.ocelot` files for one run.
5. Update the resolver to register and resolve module-qualified functions across loaded files.
6. Update the interpreter as needed so already-resolved calls continue to dispatch correctly regardless of source file.
7. Extend spec-example parsing to accept visible filename-labeled multi-file `ocelot` blocks.
8. Update spec-example execution to initialize the PAL with all declared files and run `main.ocelot`.
9. Add or update tests across parser, resolver, engine, and spec validation.
10. Run `nao check`.

# What verification should this work include?

Verification should include colocated tests for:

- lexing and parsing `::` in qualified call expressions
- resolving a same-directory module call such as `helper::run()`
- resolving a nested module call such as `math::trig::sin()`
- reporting a resolver error for a missing module file
- reporting a resolver error for a missing exported function in an existing module
- preserving current behavior for unqualified calls within the same file
- eagerly discovering all `.ocelot` files under the execution root before resolution
- executing an entry script that calls functions defined in sibling and nested module files
- spec-validation parsing of one-file and multi-file examples
- spec-validation malformed-example diagnostics for missing filename label lines, duplicate filenames, and missing `main.ocelot`
- spec-validation execution proving that declared files are written into the PAL and diagnostics mention their real filenames
- running `nao check`

# What assumptions and open questions should stay explicit?

- This plan assumes only functions participate in modules for now. If top-level constants or types arrive soon, the symbol model should be revisited instead of bolting them on awkwardly.
- This plan assumes the entry file is `main.ocelot` for spec validation examples only, not for the language generally.
- This plan assumes module lookup is rooted at the executed file's directory. If the project later needs package roots or workspace-level module resolution, that should be a separate design slice.
- This plan assumes eager loading of all `.ocelot` files under the execution root is acceptable for the current repository scale. If projects become large, the loading strategy may need to become more selective later.
- The requested behavior does not specify whether non-entry files may contain top-level statements or test items. The recommended first behavior is to allow parsing them but execute only the entry file's top-level statements; module files primarily contribute exported functions.
- Native functions should remain available without module qualification in this slice. Forcing `std::println()` now would add ceremony without benefit.
- The filename-label format is intentionally outside the fenced code block so rendered examples stay readable without pretending the label is valid `ocelot` syntax.

# What concrete tasks should track this plan?

- [ ] Add a spec chapter describing the first module-system semantics and examples.
- [ ] Add lexer support for `::`.
- [ ] Add `QualifiedIdentifier` as a `Vec<Identifier>`-backed AST node for qualified call target paths.
- [ ] Extend the parser to accept `::`-qualified call targets while keeping unsupported callee forms rejected.
- [ ] Introduce program/module data structures that track loaded source files and each file's module name.
- [ ] Register user-defined functions under fully qualified names in [`ProgramEnvironment`](/data/projects/ocelot/crates/ast/src/program_environment.rs).
- [ ] Teach the engine to eagerly load all `.ocelot` files relative to the entry file's directory before parsing and resolution.
- [ ] Update the resolver to resolve local and module-qualified function calls across loaded files.
- [ ] Add resolver diagnostics for missing module files and missing functions within loaded modules.
- [ ] Update interpreter and engine tests for multi-file execution.
- [ ] Replace single-source spec examples with named file collections in [`SpecExample`](/data/projects/ocelot/crates/spec_validation/src/spec_example.rs).
- [ ] Extend markdown example parsing to read filename label lines that precede fenced `ocelot` blocks.
- [ ] Update spec-example execution to initialize the PAL with every declared file and run `main.ocelot`.
- [ ] Add spec-validation tests for multi-file success and malformed-example failures.
- [ ] Run `nao check`.
