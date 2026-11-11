# How do I build a native executable?

On x86-64 Linux with libc development files and a GNU-compatible `cc` driver:

```sh
cargo build --workspace
./t tcc -v
target/debug/ocelot build examples/hello_world.ocelot-script -o target/hello-world
target/hello-world
```

`./t` provisions checksum-pinned TCC and its support files through tool-tool.
TCC compiles C to an object; system `cc` links the executable. The complete
C99 runtime is included in generated source. Native builds do not invoke Cargo,
link Rust libraries, or require a runtime archive. The compiler is still Rust.

Native binaries need compatible system libraries, but no TCC, Rust, Ocelot, or
original sources installed. Assertion excerpts are embedded. GNU program
compatibility depends on the build host's libc/toolchain; a musl-linked Ocelot
compiler can produce GNU programs through that host toolchain.

Scripts and module `main()` follow existing entrypoint rules. Normal builds
do not run test items.

# How can I inspect C or configure tools?

```sh
ocelot build main.ocelot --emit=c
ocelot build main.ocelot -o main.c --emit=c
ocelot build main.ocelot --emit=c -o - | less
cc -std=c99 -O2 main.c -o main
ocelot build main.ocelot -o main --tcc /path/to/tcc --linker /usr/bin/cc
```

Emitted C includes runtime definitions, ABI declarations, and cleanup helpers.
It can be compiled independently, with no Ocelot headers or libraries.
With `--emit=c`, omitting `-o` or specifying `-o -` writes only generated C to
stdout; `-o <file>` writes it to a file. Emission does not invoke TCC or execute
the source program. Language errors go to stderr without partial C output.
The old `--runtime` archive option has been removed.

Paths are single arguments, including spaces. `--tcc` selects a direct TCC
executable; the development default uses the repository's `./t tcc` wrapper.
If that wrapper is absent in an installed layout, the default command is `tcc`.
`--linker` selects a GNU-compatible linker driver, not another C compiler.
Toolchain replacement belongs behind `NativeToolchain`; emission has no TCC flags.

# How does the C runtime manage values?

ABI v1 uses an unboxed tag/payload pair. Only strings allocate handles, using
a C reference count, explicit byte length, and immutable bytes. Embedded NUL
and Unicode bytes are preserved. Calls borrow parameters and return owned
results; generated cleanup releases temporary strings, locals, and early returns.
Reference counts are non-atomic because native execution is single-threaded.
Allocation-size arithmetic and reference-count growth are checked for overflow.

Display, equality, interpolation, assertions, and eval reports are implemented
in C. The frontend embeds diagnostic text as data; its Rust renderer and
dependency tree are absent from the executable. Runtime I/O goes through
`native_io.c`, a small libc boundary; compiler platform operations still use PAL.

# How are builds and failures handled?

Compilation and linking each have a 30-second deadline. On Linux, PAL terminates
the process group and reaps the direct child on timeout. Staging is isolated
beside the output, published by rename after linking succeeds. A failed build
preserves an existing executable and retains its staging artifacts with a path
in the error. Successful staging is removed.

Language runtime failures exit 101 with diagnostics; runtime infrastructure
failures, such as allocation overflow or report I/O failure, exit 102. The spec
runner requires a private versioned report, independent of stdout/stderr, to
classify language errors and typed eval results.

The pinned TCC emits unwind ranges incompatible with GNU ld's optional lookup
header even for pure C objects. The adapter retains `--no-eh-frame-hdr`;
no exception unwinding or Rust runtime is involved.

# How is the complete specification verified?

```sh
nao check
cargo run -p ocelot-spec-validation -- --backend=all
bash scripts/check-native-portability.sh
```

Every numbered spec example is checked independently in interpreter and native
modes. Valid programs compile and execute; frontend-invalid examples must fail
the build with the written diagnostic. Runtime-error examples must build and
then fail as specified. A tool failure, crash, timeout, missing result, or
malformed protocol cannot satisfy an expected language error.

Embedded examples use compiled scenario drivers with persistent root slots.
Initializers run once, eval locals do not persist, and the first runtime failure
wins over a later frontend error. There is no interpreter fallback or public JIT.

Successful native workspaces are removed; failed artifacts remain under the
temporary spec-validation root. Direct runtime tests cover retained handles,
empty strings, NUL/Unicode contents, rendering, and allocation overflow. A
hello-world size budget guards against accidentally linking Rust dependencies.

The portability script compiles the same runtime and generated source with
Clang (or `NATIVE_CC=gcc`), strict warnings, optimization, ASan, UBSan, and leak
detection. LeakSanitizer requires permission to inspect the process.

# What remains outside native support?

Other architectures, static/musl program output, custom registered native
functions, public native embedding, concurrency, full debugger integration,
and alternate production toolchain adapters remain future work.

# How are integer operations kept consistent with interpretation?

Signed 64-bit integers are unboxed in the existing value payload. Conversion
between payload bits and signed values uses defined C99 arithmetic. Checked helpers
guard before addition, subtraction, multiplication, division, and negation, including
minimum int divided by -1. Overflow and zero division use source-located language
errors rather than infrastructure failures. Exact decimal display uses separate
caller-owned buffers for concatenation operands.

The executable [integer operations](<spec/02.09 Expressions - Integer operations.md>)
run through both backends. The sanitizer smoke script also tests integer boundaries,
valid operand pairs against independent compiler overflow builtins, and each failure
path under optimized Clang with ASan/UBSan. Those builtins are test-only; emitted C
uses portable guards. See [DR-023](<decisions/DR-023 Use checked signed 64-bit integers.md>).
