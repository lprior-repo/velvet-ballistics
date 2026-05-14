# Fuzz Target Layout

This package intentionally exposes cargo-fuzz targets as explicit `[[bin]]`
entries under `src/bin/*.rs`. `cargo fuzz list` is the source of truth for the
active target names.

The master plan names `fuzz_targets/*.rs` as the logical target set. In this
repository, that contract is satisfied by these cargo-fuzz binaries:

- `src/bin/yaml_events.rs`
- `src/bin/expression.rs`
- `src/bin/ipc_frame.rs`
- `src/bin/journal_event.rs`
- `src/bin/compiled_ir.rs`
- `src/bin/generated_compare.rs`

`fuzz_targets.rs` remains a compatibility module for callable harness bodies;
do not add duplicate `fuzz/fuzz_targets/*.rs` wrappers unless cargo-fuzz is
reconfigured to consume that layout directly.

On Linux, cargo-fuzz 0.13 defaults `check` to `x86_64-unknown-linux-musl`,
which is incompatible with ASan static libc linking. Use:

```sh
cargo fuzz check --target x86_64-unknown-linux-gnu
```
