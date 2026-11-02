# AGENTS.md

This file provides guidance to human developers and AI Agents when working with code in this repository.

## Project Overview

`ocelot` is an experimental statically typed programming language project.
The Rust workspace includes parsing, semantic resolution, an interpreter, formatting, a CLI and REPL, executable specification validation, shared utilities, and a platform abstraction layer.

Note: All developer documentation should be written in English.

## Tech Stack

- Implementation Language: Rust
- Use `ocelot_base::shared_string::SharedString` instead of plain `String` for string fields in structs when shared immutable strings are a good fit
- Use `ocelot_pal::pal::Pal` for platform interaction such as filesystem access, clocks, file watching, and process execution
- Prefer `ocelot_pal::pal_mock::PalMock` in tests when code depends on platform behavior

## Documentation Strategy

Consult `docs/PLANS.md` when creating, updating, or completing plan documents in `docs/plans`.
Consult `docs/spec` when changing or documenting language behavior.

### Planning

Use `docs/PLANS.md` for plan structure, naming, verification expectations, assumptions, and completion workflow.

### Development learnings

Consult [development learnings](docs/learnings/README.md) when investigating unfamiliar behavior or working in an area with recorded findings.
Create or update a learning in `docs/learnings/` alongside work that establishes a reusable finding, especially a complex or surprising discovery, disproved assumption, or debugging, performance, or platform pitfall. Routine changes do not require an entry.

Follow the guide and template. Name records `YYYY-MM-DD Finding title.md`, using the date the finding was established.
Include the finding, context, evidence, and references. Distinguish observations from inference, link relevant internal and external sources when available, and explicitly state when either has no applicable source.
Update findings as evidence changes, preserving useful context. Link related plans or decisions rather than duplicating them.

### Question driven documentation

When writing any documentation, prefer writing the headings in the form of questions, which should be answered in the following paragraphs.
This helps with writing since the questions should be answered.
It also makes it easiers for readers to determine if a section is relevant.

### Hyperlit in-code comments
Use hyperlit comment markers ("📖") only for non-standard rationale that is not well covered by standard API docs. This ensures that:

- **Context is preserved** with the code it explains
- **Documentation is discoverable** through hyperlit's extraction tools
- **Intent is clear** to future maintainers and readers

Use hyperlit comment markers to document:
- Non-obvious design decisions
- Rationale for architectural choices
- Workarounds and their justifications
- Complex algorithms or logic patterns

Format these comments as markdown.

Always use a heading as the first line of the comment.

Prefer to formulate the heading as a question ("Why ..."). This makes it easier to search for specific documentation.

Example:
```rust
/* 📖 # Why use Arc<Mutex<T>> for the app state?
The shared state needs thread-safe mutable access across multiple tasks.
Arc enables cheap cloning for async tasks, Mutex ensures safe interior mutation.
*/
let state = Arc::new(Mutex::new(data));
```

Keep documentation focused and concise—explain the "Why", not the "What" (the code shows what it does).

### Function, Interface, struct and class documentation

Functions, interfaces, structs and classes should be documented using the standard language syntax (e.g. JsDoc/TsDoc or RustDoc).
Use this standard documentation style by default.
The question-driven documentation style does not apply to RustDoc comments.
RustDoc should be written classically, telling the reader what the item does.
Fields on interfaces, structs, and classes should also be documented (including private/internal fields where useful for maintenance).

## Testing strategy

Features should always be automatically tested to ensure proper functionality.
Consult `docs/TESTING.md` when writing tests.

Tests should be colocated with the code, i.e. in the same file.

Use snapshot tests where appropriate.

Prefer data driven tests to reduce code duplication.

Prefer black box testing and try to avoid mocking as much as possible.

## Checks and formatting

When completing a unit of work run `nao check` to verify everything is green.

## Commit messages

Commit message should be in the "Conventional Commits" format, e.g. "feat(UI): Add about button to see version and build date".

Below the first line include detail information about the changes made.

**Important:** Append all user prompts included in this commit to the commit message body under a `User Prompts:` section.
Also include the agent model identifier used for the commit in a `Model:` section in the commit message body.
Always run `git add` and `git commit` as separate commands.

Always commit useful units of work after `nao check` passes. Do not leave completed work uncommitted.
Make separate commits for different logical changes. Keep each commit coherent and independently reviewable.
For minor follow-up changes to the latest commit from the current task, rerun `nao check` and amend that commit instead of creating another commit.

Write multiline commit messages to a temporary file and use `git commit -F <message-file>` (or `git commit --amend -F <message-file>`).
Do not use heredocs or heredoc delimiters such as `EOF` to supply commit messages.

Never push code or ask to push code.

## File naming and organization

Prefer small source files. If not otherwise instructed, put each struct, enum, trait, etc. in its own file.

Only use lib.rs files to declare modules, do not put any structs, traits or functions there.

Each and every struct, enum, trait, and their associated impl blocks should be in its own file.

Choose descriptive names for files. Avoid names like "index.ts" or "types.ts".
Do not bulk export items using "export * from 'submodule'".
