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

On this Linux workspace, cargo-fuzz 0.13.1 defaults `build`/`run` to
`x86_64-unknown-linux-musl`, which is incompatible with ASan static libc
linking (`cannot specify -static with -fsanitize=address`). Use the GNU target
for canonical sanitizer builds and smoke runs:

```sh
cargo fuzz check --target x86_64-unknown-linux-gnu
cargo fuzz build --target x86_64-unknown-linux-gnu
cargo fuzz run --target x86_64-unknown-linux-gnu <target> -- -max_total_time=60 -print_final_stats=1
```

The target name still comes from `cargo fuzz list`; for example,
`foreach_digest_canonical` is the bounded canonical-digest target used by the
global verifier blocker repair evidence.
