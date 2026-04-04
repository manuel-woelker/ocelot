# Why add module imports now?

The current module system works, but it is already too ceremony-heavy for ordinary cross-file use.
Every imported function call must spell the full module path such as `math::add::sum()`, even when one file uses the same helper repeatedly.

Adding `use` declarations now keeps the existing filesystem-shaped module system intact while making multi-file code more readable:

- `use other_module::a_function;` should allow `a_function()`
- `use a_module::{a, b, c};` should allow `a()`, `b()`, and `c()`
- fully qualified calls should continue to work unchanged

This is the smallest useful ergonomics slice on top of the existing file-module design.

# What user-visible behavior should this slice define?

This plan should establish the following semantics:

- `use` is a top-level item in both `.ocelot` modules and `.ocelot-script` files
- `use other_module::a_function;` imports exactly one exported function into the current file
- `use a_module::{a, b, c};` imports multiple exported functions from the same module
- imported functions can be referenced by their final segment as ordinary unqualified call targets
- fully qualified names such as `a_module::a()` remain valid even when the same function is imported
- import order should not matter within one file
- imports affect only the current file; they do not re-export names to other modules

Recommended scope limits for this slice:

- no aliasing syntax such as `use math::sum as add`
- no wildcard imports such as `use math::*`
- no nested brace groups such as `use math::{add::sum, trig::sin}`
- no relative import syntax such as `use self::helper`
- imports apply to function names only in this slice

That last scope limit is deliberate.
The current language has module-qualified functions, while effects remain globally named and user-defined types are not yet file-module exports in the same sense.
Trying to solve all named-thing imports at once would be premature.

# What implementation gaps does this work need to close?

Today the parser has no `use` item, the lexer does not reserve `use` as a keyword, and the resolver only knows two ways to resolve a call target:

- an unqualified local-or-builtin function name
- a fully qualified module function path

There is no file-local import table that can participate in name resolution, and there are no user-facing diagnostics for import conflicts or invalid imported names.

# What syntax and AST changes should represent imports cleanly?

Recommended approach:

- reserve `use` as a keyword in the lexer
- add a dedicated AST item such as `UseItem`
- represent the imported module path separately from the imported member list
- keep grouped imports explicit in the AST instead of lowering brace syntax into raw strings in the parser

One pragmatic shape would be:

- `UseItem { module_path: QualifiedIdentifier, imported_names: Vec<Identifier>, span }`
- `ItemKind::Use(UseItem)`

This keeps parsing straightforward and gives the resolver structured data for diagnostics and later extensions like aliasing.

# How should name resolution treat imported functions?

The clean behavior is to resolve imports before ordinary statement and function-body resolution, then let imported names participate in unqualified lookup.

Recommended resolution order for an unqualified call target:

1. a function defined in the current module
2. an imported function for the current file
3. a builtin native function such as `println`

Recommended resolver behavior:

- each source file gets its own import table
- each imported name is validated against an already loaded module and exported function
- the import table maps the local binding name to the fully qualified function index
- grouped imports are equivalent to repeated single-name imports
- function bodies inherit the import table from their defining file

This avoids AST rewriting and keeps the existing resolved-function-index pipeline intact.

# What diagnostics should this slice provide?

Imports need to fail clearly and early in the resolver.

Recommended diagnostics:

- importing from an unknown module should report `unknown module \`...\``
- importing a missing function from a known module should report `module \`...\` has no function \`...\``
- importing the same local name twice in one file should report a duplicate import error
- importing a local name that conflicts with a function defined in the current module should report a conflict error instead of silently shadowing
- grouped imports should point diagnostics at the specific imported name when possible

Recommended duplicate/conflict rule:

- file-local function declarations win conceptually because they are the file's own API surface
- conflicting imports should therefore be rejected, not shadowed

That rule is more predictable than letting import order or registration order decide behavior.

# What implementation order keeps this work manageable?

1. Add a spec chapter under `docs/spec` that defines `use` imports, grouped imports, and their scope limits.
2. Add lexer support for the `use` keyword.
3. Add AST types for `use` items and grouped imported-name lists.
4. Extend the parser to accept `use module::name;` and `use module::{a, b, c};` as top-level items.
5. Extend the loaded-program or resolver state with a file-local import table keyed by source file and local imported name.
6. Register imports for each file before resolving statements and function bodies in that file.
7. Validate imported names against the already loaded module/function table and emit dedicated resolver diagnostics for unknown modules, unknown functions, and duplicate/conflicting imports.
8. Update unqualified function resolution so imported names participate between module-local functions and builtins.
9. Add or update parser, resolver, engine, and spec-validation tests.
10. Run `nao check`.

# What verification should this work include?

Verification should include colocated tests for:

- parsing `use helper::greet;`
- parsing `use math::{sum, product, quotient};`
- allowing imported names to be used from top-level script statements
- allowing imported names to be used inside functions defined in the importing file
- preserving existing fully qualified call behavior
- resolving a local function in preference to an imported function with the same final name
- reporting duplicate imported names in one file
- reporting conflicts between imported names and local function declarations
- reporting unknown modules in `use` items
- reporting unknown functions in `use` items
- proving grouped imports behave the same as repeated single-name imports
- spec-validation success coverage for single and grouped imports
- running `nao check`

# What assumptions and open questions should stay explicit?

- This plan assumes imports are file-local and non-transitive. `use` should not implicitly re-export names.
- This plan assumes imports only target exported functions in this slice. Importing effects, types, modules, or future values should be a separate follow-up if needed.
- This plan assumes `use` items are only valid at the top level. Allowing imports inside function bodies would complicate parsing and scoping for little benefit right now.
- This plan assumes import resolution can rely on eager module loading that already exists today.
- Open question: should `use helper::greet;` be allowed in the same file that defines module `helper`, or should self-imports be rejected as redundant?
- Open question: should unused imports remain accepted for now, or should the resolver eventually report them as warnings once warning infrastructure matters more?

# What concrete tasks should track this plan?

- [ ] Add a spec chapter documenting `use` imports and grouped imports.
- [ ] Reserve `use` in the lexer.
- [ ] Add AST support for top-level `use` items.
- [ ] Parse single-name and grouped imports.
- [ ] Introduce resolver state for file-local imported function bindings.
- [ ] Register and validate imports before resolving statements and function bodies.
- [ ] Update unqualified function resolution to consult imports between local functions and builtins.
- [ ] Add resolver diagnostics for unknown modules, unknown imported functions, and duplicate/conflicting imports.
- [ ] Add or update parser, resolver, engine, and spec-validation tests.
- [ ] Run `nao check`.
