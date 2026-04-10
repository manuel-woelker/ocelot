# Why does the CLI need a REPL command now?

The repository now has a persistent [`EmbeddedEngine`](/data/projects/ocelot/crates/engine/src/embedded_engine.rs) that can execute and evaluate code across multiple calls.
That is enough runtime capability for a basic interactive REPL, but the CLI still only supports:

- `ocelot fmt`
- `ocelot <source-file>`
- `ocelot test [filter-expression]`

Without a REPL command, the new embedding functionality is only available to Rust callers.
That is a waste: the project should be able to dogfood embedded eval interactively from the terminal.

# What should the first REPL command do?

The first version should stay deliberately small and boring.

Recommended user-facing shape:

- add a `repl` subcommand to the existing CLI parser
- default the REPL `base_path` to the current working directory
- optionally accept one explicit base path argument if that keeps the CLI shape simple, for example `ocelot repl examples`
- read one line at a time from standard input
- evaluate or execute that line against one persistent [`EmbeddedEngine`](/data/projects/ocelot/crates/engine/src/embedded_engine.rs) session
- print successful eval results using the stable [`RuntimeValue`](/data/projects/ocelot/crates/semantic/src/runtime_value.rs) eval-result rendering
- print compilation and runtime failures using the existing diagnostic rendering path instead of ad hoc debug strings

The first slice does not need line editing, history, multiline input, or shell-like meta commands.
Trying to make it “nice” too early would just hide the real integration work under terminal fluff.

# How should one REPL line be interpreted?

The REPL should use the simplest rule that matches the current language and engine shape.

Recommended direction:

- first try to treat the line as an expression and run `EmbeddedEngine::eval()`
- if expression parsing or resolution fails because the input is not a valid expression form, fall back to `EmbeddedEngine::execute()`
- keep the embedded session persistent across both successful `eval` and `execute` calls

This lets common interactive use stay ergonomic:

- `not false` prints `true`
- `"hello"` prints `"hello"`
- `fun greet() { println("hi"); }` defines a function without requiring a prefix
- `greet()` can then evaluate in a later line

The fallback should be narrow and intentional.
If the command cannot confidently distinguish “not an expression” from “real eval failure,” the plan should prefer explicit rules over clever guessing.

# How should REPL errors be rendered?

The REPL should not invent a second diagnostics style.

Recommended direction:

- pretty-print parser and resolver failures using the same rendered source diagnostics already attached to compilation errors
- pretty-print runtime failures through the same CLI error formatting used elsewhere in [`main.rs`](/data/projects/ocelot/crates/cli/src/main.rs)
- keep ANSI styling consistent with the existing CLI output path

That means the implementation should reuse existing infrastructure such as:

- [`format_cli_error()`](/data/projects/ocelot/crates/cli/src/main.rs) at the CLI boundary
- [`render_source_diagnostics()`](/data/projects/ocelot/crates/base/src/render_source_diagnostics.rs) through the existing engine error sources

The REPL should print errors and continue running instead of exiting on the first bad line.

# How should the CLI parser and command execution change?

The CLI is currently a small handwritten parser in [`crates/cli/src/main.rs`](/data/projects/ocelot/crates/cli/src/main.rs).
The first REPL command should extend that parser directly instead of introducing a new command framework just for one subcommand.

Recommended refactor:

- add `CliCommand::Repl { base_path: Option<String> }`
- extend `parse_command()` to recognize `repl`
- update `print_usage()` to document the new command
- add one `run_repl()` helper that owns the interactive loop
- keep the main command dispatch in [`try_execute_command()`](/data/projects/ocelot/crates/cli/src/main.rs) easy to read

If the parser starts getting noisy after this change, that is a separate refactor conversation.
It would be overengineered to front-load that work into this slice.

# What input and exit behavior should the first REPL support?

Recommended first behavior:

- print a short prompt, for example `> `
- stop on EOF
- optionally support one explicit exit word such as `.exit` or `:quit`
- ignore empty lines

The plan should avoid requiring a dependency like `rustyline` unless a real blocker appears.
Standard input plus a prompt is enough for a basic first cut.

# What implementation order keeps this manageable?

1. Add an active plan for the REPL CLI work.
2. Extend the CLI command parser in [`crates/cli/src/main.rs`](/data/projects/ocelot/crates/cli/src/main.rs) with a `repl` subcommand and updated usage text.
3. Add a small `run_repl()` loop that reads lines from stdin, owns one persistent [`EmbeddedEngine`](/data/projects/ocelot/crates/engine/src/embedded_engine.rs), and prints a prompt.
4. Decide and implement the line-handling rule for `eval` first with `execute` fallback, or another deliberately simple alternative if testing shows that fallback is too ambiguous.
5. Reuse the existing CLI and diagnostic rendering infrastructure so parse, resolve, and runtime failures are pretty-printed and do not terminate the session.
6. Print successful eval results using stable eval-result rendering and keep successful execute-only lines quiet unless a prompt-level acknowledgment is clearly needed.
7. Add colocated CLI tests for command parsing, prompt/session behavior, persistent definitions across lines, pretty-printed errors, and clean REPL exit.
8. Run targeted CLI and engine tests plus `nao check`.

# How should this work be verified?

Verification proved both the CLI surface and the embedded runtime behavior.

- [x] Add CLI tests proving `parse_command()` recognizes `repl` and optional base-path arguments.
- [x] Add CLI tests proving the REPL retains definitions across multiple input lines.
- [x] Add CLI tests proving expression inputs print eval results using stable rendering.
- [x] Add CLI tests proving declaration or statement inputs update session state without crashing the loop.
- [x] Add CLI tests proving parse, resolver, and runtime failures are pretty-printed and the REPL continues reading later lines.
- [x] Add CLI tests proving EOF or the chosen exit command ends the REPL cleanly.
- [x] Run `cargo test -p ocelot -p ocelot-engine`.
- [x] Run `nao check`.

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes the current PAL abstraction can supply stdin lines or that the CLI can use standard library input directly without conflicting with repository conventions. If PAL-backed stdin support is expected, the implementation may need to extend the PAL instead of reading stdin directly.
- The “eval first, execute fallback” rule is convenient, but it may blur real eval errors with “that line was meant as a statement.” If that becomes confusing in practice, the REPL may need explicit prefixes or a stricter parse strategy.
- The current language does not have multiline blocks that are pleasant to enter interactively one line at a time. The first REPL should stay honest about that limitation instead of faking multiline support badly.
- Pretty-printing should reuse the existing diagnostic rendering path, not duplicate it inside the REPL loop. If the current error layering makes that awkward, the implementation should fix the layering rather than adding another custom renderer.
- A REPL rooted at the current directory can accidentally load unrelated `.ocelot` modules. If that turns out to be too surprising, requiring an explicit base path may be the better CLI contract.
- This plan intentionally does not include history, completion, or editor-like behavior. Those can be follow-up work once the core REPL semantics are stable.

# What concrete tasks should track this plan?

- [x] Add an active plan for the REPL CLI command.
- [x] Add a `repl` CLI command and document it in usage output.
- [x] Create a basic interactive loop that owns one persistent `EmbeddedEngine`.
- [x] Support a base path for REPL module loading.
- [x] Implement the chosen line interpretation strategy for eval versus execute.
- [x] Pretty-print errors using the existing diagnostics rendering infrastructure without terminating the REPL session.
- [x] Add colocated CLI tests for parsing, persistence, output, errors, and exit behavior.
- [x] Run `cargo test -p ocelot -p ocelot-engine`.
- [x] Run `nao check`.

# What landed from this plan?

This work added a basic interactive REPL to the CLI:

- [`crates/cli/src/main.rs`](/data/projects/ocelot/crates/cli/src/main.rs) now supports `ocelot repl [base-path]`
- the REPL owns one persistent [`EmbeddedEngine`](/data/projects/ocelot/crates/engine/src/embedded_engine.rs) session rooted at the selected base path
- each line is handled by trying `eval()` first and falling back to `execute()` on parser or resolver compilation failures
- successful eval inputs print stable rendered results, while execute-only inputs stay quiet
- pretty-printed errors reuse the existing CLI diagnostic rendering path and do not terminate the session
- colocated CLI tests cover parsing, session persistence, eval output, execute fallback, error continuation, and clean exit on EOF or `.exit`
- `cargo test -p ocelot -p ocelot-engine` and `nao check` pass
