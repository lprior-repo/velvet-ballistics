# velvet-ballistics/fuzz

This crate hosts two distinct categories of harnesses. The categories exist
because **they are not equivalent evidence** (see bead vb-4b0mk).

## `fuzz_targets/` — libFuzzer harnesses (real fuzz coverage)

Each file uses `#![no_main]` and `libfuzzer_sys::fuzz_target!`. These are
the canonical fuzz targets. Run with:

```bash
cargo fuzz run <target> -- -max_len=65536 -runs=100000
```

They mutate input via libFuzzer, maintain a corpus, and report coverage.
**Only these count as fuzz coverage for an obligation.**

## `stdin_smoke/` — stdin-fed static binaries (NOT fuzz coverage)

Each file is a thin wrapper around `fuzz_lib::bin_common::run_with_stdin`.
They take input from stdin and call the same harness body as the
corresponding `fuzz_targets/` entry, but they do NOT use libFuzzer
mutation, do NOT maintain a corpus, and do NOT report coverage.

They are useful for:

- Reproducing a single saved input without spinning up a libFuzzer session.
- Smoke-testing the harness body in CI without the libFuzzer toolchain.

**They are NOT fuzz coverage.** CI must NOT report a stdin_smoke binary
run as evidence that an obligation has been fuzzed.

## Layout

```text
fuzz/
├── Cargo.toml
├── fuzz_targets/   # libFuzzer harnesses (real fuzz coverage)
├── stdin_smoke/    # stdin wrappers (NOT fuzz coverage)
├── corpus/         # libFuzzer corpus (used only by fuzz_targets/)
└── src/            # shared harness bodies (`fuzz_lib`)
```