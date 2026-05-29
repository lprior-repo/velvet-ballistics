# Fuzz Evidence — vb-lnubg

## Repair summary

- Restored the four `fuzz/Cargo.toml` manifest-declared libFuzzer targets:
  - `fuzz/fuzz_targets/diagnostic_from_error.rs`
  - `fuzz/fuzz_targets/diagnostic_code_from_str.rs`
  - `fuzz/fuzz_targets/span_bridge_fuzz.rs`
  - `fuzz/fuzz_targets/compile_source_ast_marks.rs`
- Each wrapper calls the corresponding shared fuzz body in `fuzz/src/lib.rs`.
- Updated stale shared fuzz bodies to match current diagnostic and source-map APIs.

## Commands run

```bash
moon run velvet-ballistics:fuzz-smoke
```

Result: PASS. Output summary: `Tasks: 1 completed`.

```bash
rustup run nightly-2026-04-28 rustfmt --edition 2024 --check fuzz/src/lib.rs fuzz/fuzz_targets/diagnostic_from_error.rs fuzz/fuzz_targets/diagnostic_code_from_str.rs fuzz/fuzz_targets/span_bridge_fuzz.rs fuzz/fuzz_targets/compile_source_ast_marks.rs
```

Result: PASS.

## Scope and limitations

- This is fuzz-smoke/build evidence only, not long-duration campaign evidence.
- No `unsafe` code was added.
- Full `cargo fmt --check --manifest-path fuzz/Cargo.toml` remains blocked by pre-existing formatting drift in `fuzz/fuzz_targets/vb_storage_codec.rs`, which was not part of this bead.
