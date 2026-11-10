# How do I build a native executable?

On x86-64 Linux with libc development files and a GNU-compatible `cc` driver:

```sh
cargo build --workspace
./t tcc -v
target/debug/ocelot build examples/native-smoke.ocelot-script -o /tmp/hello
/tmp/hello
```

`./t` provisions checksum-pinned TCC and its support files through tool-tool.
TCC compiles C to an object; system `cc` links the Rust runtime archive. The archive
is built once, not by each Ocelot invocation. Installed releases keep
`libocelot_native_runtime.a` beside `ocelot` and require TCC and the linker.
Release GNU runtime archives are built on Ubuntu 24.04; Ubuntu 24.04 or newer is
the supported distribution baseline. Locally built archives use the local libc.
The compiler itself can be musl-linked independently of its GNU program output.

Native binaries contain machine code and their runtime. Running them requires
compatible system libraries, but not TCC, Rust, Ocelot, or original sources.
Assertion excerpts are embedded. Scripts and module `main()` follow existing
entrypoint rules; normal builds do not run test items.

# How can I inspect C or configure tools?

```sh
ocelot build main.ocelot -o main.c --emit=c
ocelot build main.ocelot -o main --tcc /path/to/tcc --linker /usr/bin/cc --runtime /path/to/libocelot_native_runtime.a
```

Paths are single arguments, including paths with spaces. `--tcc` selects a direct
TCC executable; the development default uses the repository's `./t tcc` wrapper.
If that wrapper is absent in an installed layout, the default command is `tcc`.
`--linker` selects a GNU-compatible linker driver, not a different C compiler.
Toolchain replacement belongs behind `NativeToolchain`; the emitter has no TCC
flags. Generated C includes its ABI declarations and frame helpers.

ABI v1 represents values as an unboxed tag/payload pair; only strings allocate
handles. Calls borrow parameters and return owned results. Generated cleanup
releases temporaries, block locals, and early-return paths. Canonical identities
are mangled into C symbols and expression sequencing is explicit statements.

# How are builds and failures handled?

Compilation and linking each have a 30-second deadline. On Linux, PAL terminates
the process group and reaps the direct child on timeout. Staging is isolated beside
the output, which is published by rename only after successful linking.
Failed builds preserve an existing executable and retain staging artifacts with
a diagnostic pointing to their directory. Successful staging is removed.

Language runtime errors exit 101 and print diagnostics; other tool/process errors
remain separate. The spec runner uses private versioned reports, independent of
stdout/stderr, to classify errors and typed eval results. Rust panics cannot unwind
through the C ABI. The pinned TCC emits unwind ranges incompatible with GNU ld's
optional lookup header, so this adapter explicitly uses `--no-eh-frame-hdr`.
Generated frames are not used for exception unwinding.

# How is the complete specification verified?

```sh
nao check
cargo run -p ocelot-spec-validation -- --backend=all
bash scripts/check-native-portability.sh
```

`nao check` requires interpreter and native spec passes. `--backend=interpreter`
and `--backend=c-tcc` select individual passes. Native validation requires real
TCC, linker, runtime archive, and files; missing prerequisites fail setup.

Every numbered spec example has an independent result in each mode. Valid file
programs compile and execute. Frontend-invalid examples must fail the build with
the written language diagnostic; runtime-error examples must compile and then
fail as specified. A crash, timeout, missing result, compiler/linker failure, or
malformed protocol cannot count as an expected language error.

Embedded examples compile finite scenario drivers with retained root slots and
separate step functions. Initializers run once, eval locals do not persist, and
the first runtime failure wins over a later frontend error. There is no interpreter
fallback. These drivers are test artifacts, not a public compiled REPL or JIT.

Each native example has a real isolated workspace. Successful artifacts are
removed; failed artifacts stay under the temporary spec-validation root.
The portability smoke test compiles the same C with Clang (or `NATIVE_CC=gcc`),
strict warnings, optimization, AddressSanitizer, UndefinedBehaviorSanitizer,
and leak detection. LeakSanitizer requires permission to inspect the process.

# What remains outside native support?

Other architectures, static/musl program output, custom registered native
functions, public native embedding, full debugger integration, and alternate
production toolchain adapters remain future work. TCC prioritizes compilation
speed; no numerical throughput or optimization advantage is promised.
