# Why does the CLI need a dedicated `fmt` command now?

The repository now has a formatter crate, but there is no user-facing command that applies it to source files.
That leaves formatting available only as a library concern and forces tests and documentation to rely on indirect validation instead of a normal developer workflow.

The next slice should add a CLI `fmt` command that reformats source files in the current directory tree and updates files only when the formatted output differs from the existing contents.

# What behavior should `ocelot fmt` provide?

The first version should be intentionally simple:

- `ocelot fmt` scans the current directory recursively for `*.ocelot` and `*.ocelot-script`
- each matching file is read and parsed in memory
- if formatting produces identical output, the file is left untouched
- if formatting produces different output, the file is replaced atomically
- parser or formatter failures should fail the command with the normal CLI error rendering

This plan does not add path arguments, `--check`, diff output, or partial formatting.
The command is just "format the current tree safely."

# How should the CLI surface this command?

[`crates/cli/src/main.rs`](/data/projects/ocelot/crates/cli/src/main.rs) currently treats the first positional argument as either `test` or a path to execute.
That parser should grow a dedicated `Fmt` command variant instead of overloading the existing path behavior.

Recommended CLI shape:

- `ocelot fmt`
- `ocelot test [source-file...]`
- `ocelot <source-file>`

The usage text should be updated to make `fmt` visible.
If extra positional arguments are passed to `fmt`, the command should reject them for now instead of silently ignoring them.

# How should files be discovered and reformatted?

The CLI already has file discovery logic for tests in [`crates/cli/src/main.rs`](/data/projects/ocelot/crates/cli/src/main.rs).
The formatter command should reuse the same glob set and current-directory root:

- `*.ocelot`
- `*.ocelot-script`

Recommended formatting flow per file:

1. Read the file contents through `Pal`.
2. Parse the file with [`ocelot_parser::parse_compilation_unit`](/data/projects/ocelot/crates/parser/src/parse_compilation_unit.rs).
3. Format the resulting compilation unit with [`format_compilation_unit`](/data/projects/ocelot/crates/formatter/src/format_compilation_unit.rs).
4. Compare formatted output to the original source text.
5. If unchanged, do nothing.
6. If changed, write the formatted output to a temporary sibling path and atomically rename it over the original file.

Doing the parse and format entirely in memory before any write keeps the command honest and reduces the chance of leaving partial file contents behind.

# Why does the PAL need a rename operation?

Atomic replacement is a filesystem behavior, so it belongs in the PAL instead of being open-coded in the CLI.
Using only `write_file()` would risk partially written files if the process dies mid-write and would not give the CLI a clean abstraction for atomic replacement.

The PAL should add a dedicated rename operation, for example:

```rust
fn rename(&self, from: &FilePath, to: &FilePath) -> OcelotResult<()>;
```

Implementation expectations:

- [`PalReal`](/data/projects/ocelot/crates/pal/src/pal_real.rs) should delegate to the platform rename primitive
- [`PalMock`](/data/projects/ocelot/crates/pal/src/pal_mock.rs) should update its in-memory file map and record the effect for tests
- rename should be defined for files in the same filesystem location, which is compatible with the temporary-sibling strategy used by `fmt`

The plan should keep the CLI responsible for choosing the temporary file path, while the PAL remains responsible for the actual rename behavior.

# What temporary-file strategy keeps replacement atomic?

The formatter command should write formatted content to a temporary file in the same directory as the destination file, then rename that temporary file over the original.
Keeping the temporary file in the same directory is important because ordinary rename-based replacement is only reliably atomic within one filesystem.

Recommended constraints:

- derive a deterministic temporary file name that does not collide with tracked source names in normal usage
- ensure the parent directory already exists before writing
- only rename after the full formatted output has been written successfully
- avoid deleting the original file first

The exact temp-file naming scheme can stay implementation-local as long as it is stable and clearly outside the normal language file suffixes.

# What implementation order keeps the change small?

1. Add `rename()` to the [`Pal`](/data/projects/ocelot/crates/pal/src/pal.rs) trait.
2. Implement and test `rename()` in [`PalReal`](/data/projects/ocelot/crates/pal/src/pal_real.rs) and [`PalMock`](/data/projects/ocelot/crates/pal/src/pal_mock.rs).
3. Add a small CLI helper that discovers formatable files in the current directory.
4. Add a formatting helper in the CLI that reads, parses, formats, compares, writes a temporary sibling file, and renames it atomically when needed.
5. Extend command parsing and usage text to support `fmt`.
6. Add CLI tests covering command parsing, no-op formatting, rewritten formatting, and failure behavior.
7. Run `nao check`.

# How should this work be verified?

Verification should cover both command behavior and filesystem semantics.

Required coverage:

- PAL tests for rename success and effect logging in [`crates/pal/src/pal_mock.rs`](/data/projects/ocelot/crates/pal/src/pal_mock.rs)
- PAL tests or focused coverage showing real rename behavior is wired correctly in [`crates/pal/src/pal_real.rs`](/data/projects/ocelot/crates/pal/src/pal_real.rs) where practical
- CLI parsing tests for `fmt` and invalid extra arguments
- CLI tests showing `fmt` discovers `*.ocelot` and `*.ocelot-script` files from the current directory
- CLI tests showing already formatted files are not rewritten
- CLI tests showing misformatted files are rewritten through temporary-file write plus rename
- CLI tests showing parser failures surface as normal command failures
- `nao check`

# What assumptions, risks, and open questions should stay explicit?

- This plan assumes recursive discovery from the current directory is the right first default for `fmt`, matching the user's request and the repository's existing test discovery pattern.
- The first version intentionally does not add `fmt <paths...>` or `fmt --check`. If those are wanted later, the command parser should expand deliberately rather than guessing now.
- Atomic replacement depends on writing the temporary file in the same directory as the destination. A temp file somewhere else would undercut the rename guarantee.
- The formatter currently only supports the existing language surface and restricted comment placement. `fmt` should rely on parser failures rather than pretending every source file is formatable.
- Temporary files should not be left behind on the success path. Cleanup on failure is desirable but secondary to never corrupting the original file.

# What concrete tasks should track this plan?

- [ ] Add `rename()` to [`crates/pal/src/pal.rs`](/data/projects/ocelot/crates/pal/src/pal.rs).
- [ ] Implement `rename()` in [`crates/pal/src/pal_real.rs`](/data/projects/ocelot/crates/pal/src/pal_real.rs).
- [ ] Implement `rename()` and effect logging in [`crates/pal/src/pal_mock.rs`](/data/projects/ocelot/crates/pal/src/pal_mock.rs).
- [ ] Add PAL tests covering rename behavior.
- [ ] Extend [`crates/cli/src/main.rs`](/data/projects/ocelot/crates/cli/src/main.rs) with a `Fmt` command variant and updated usage text.
- [ ] Add CLI helpers to discover `*.ocelot` and `*.ocelot-script` files from the current directory for formatting.
- [ ] Implement in-memory parse and format for each discovered file.
- [ ] Write changed files through a temporary sibling file and replace originals with `Pal::rename()`.
- [ ] Add CLI tests for `fmt` parsing, no-op files, rewritten files, and parse-failure handling.
- [ ] Run `nao check`.
