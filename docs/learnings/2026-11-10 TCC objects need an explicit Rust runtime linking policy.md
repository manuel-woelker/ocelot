# Why does the TCC adapter use a system linker?

- Learned: 2026-11-10

## What is the finding?

With the pinned V-provided TCC archive, compiling C to an object works, but linking
the Rust static runtime requires an explicit compatible linker policy. GNU cc
links the objects and archive, provided its optional unwind lookup header is
disabled for the tested TCC-generated cleanup helpers. Without that option, ld
reported overlapping FDE ranges. This is an observed compatibility issue for this
toolchain/input, not proof that every TCC version or Rust archive is incompatible.

## What is the context?

Ocelot's C backend uses TCC 0.9.28rc, build 2026-07-24 HEAD@85ba3ae8, distributed
in vlang/tccbin commit 60d606e37e6afe1ac5801fd88c55036a56b7caae. The probe used
Rust 1.98.0 on x86-64 GNU/Linux. The initial direct TCC link reported missing
libtcc1 and unresolved unwind symbols with the supplied search configuration;
we did not establish that every possible direct-link configuration fails.

## What evidence supports the finding?

A C ABI probe created, printed, and released a runtime string. Compiling it with
TCC and linking the object with cc initially failed with:

```text
.eh_frame_hdr refers to overlapping FDEs
final link failed: bad value
```

Adding `-Wl,--no-eh-frame-hdr` linked and ran correctly. Generated C never unwinds
through its frames. The production adapter uses this explicit path, with
`--gc-sections` to discard unused runtime code, instead of a silent linker fallback.
Native spec execution and the source-driven engine regressions exercise this path.
Generated C also passes an optimized Clang sanitizer/leak smoke test.

The installed-layout smoke test exposed another assumption: tool-tool resolves its
configuration from its working directory. The adapter now sets that directory
explicitly and passes absolute source/artifact paths through PAL, preserving the
caller's output location even when invoked outside the repository.

## What references apply?

- Internal: [TCC adapter](../../crates/engine/src/tcc_toolchain.rs),
  [atomic build staging](../../crates/engine/src/build_native.rs),
  [native regression tests](../../crates/engine/src/native_source.rs), and
  [tool configuration](../../.tool-tool/tool-tool.v2.kdl).
- External: [pinned binary distribution](https://github.com/vlang/tccbin/tree/60d606e37e6afe1ac5801fd88c55036a56b7caae)
  and [Rust mixed-language linkage](https://doc.rust-lang.org/reference/linkage.html).
  No external source establishes this particular overlapping-FDE observation;
  the reproduction and linked tests are its evidence.
