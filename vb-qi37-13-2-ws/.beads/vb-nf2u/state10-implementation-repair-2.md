STATUS: PASS

# State 10 Second Implementation Repair: vb-nf2u

## Files changed
- `.config/nextest.toml`
- `Cargo.lock`
- `crates/vb_ui_snapshot/src/checks.rs`
- `crates/vb_ui_snapshot/src/error.rs`
- `crates/vb_ui_snapshot/src/layout_kernel.rs`
- `crates/vb_ui_snapshot/src/lib.rs`
- `crates/vb_ui_snapshot/src/redaction.rs`
- `crates/vb_ui_snapshot/src/report.rs`
- `crates/vb_ui_snapshot/src/snapshot.rs`
- `fuzz/Cargo.toml`
- `xtask/src/evidence.rs`
- `xtask/src/lib.rs`

## Repair summary
- Added the public `vb_ui_snapshot::snapshot`, `redaction`, `layout_kernel`, and report validator seams required by the RED behavioral tests.
- Added fixture-backed public operations that return typed `UiSnapshotError` values through their owning snapshot/check/report/token/image/io paths.
- Replaced fieldless `UiReleaseGateError` behavior with diagnostic-bearing typed variants plus contract helper APIs used by `xtask/tests/ui_release_errors.rs`.
- Made `cargo xtask ai-release --bead vb-nf2u` fail closed when negative fixtures report `actual_status=passed`, while still writing `negative-fixtures.txt` evidence.
- Made negative fixture evidence consume command-boundary fixture fields for changed control IDs, rectangle bounds, nonces, status, and redacted secret samples instead of relying on canned audit text.
- Registered `fuzz/fuzz_targets/ui_redaction_artifact.rs` in `fuzz/Cargo.toml` and wired it to the real redaction scanner.
- Serialized nextest execution with `.config/nextest.toml` because the acceptance harness uses a shared negative-fixture directory across test processes.

## Commands run
- `bd prime` — PASS for workflow context; Dolt auto-push warning reported non-fast-forward remote.
- `cargo nextest run -p vb_ui_snapshot -p vb_ui_makepad -p xtask` — initially FAIL, then PASS: 128 run, 128 passed, 0 skipped.
- `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance` — initially FAIL from shared fixture races before serialization, then PASS: 8 run, 8 passed, 0 skipped.
- `rtk cargo fmt --all` — PASS.
- `rtk cargo fmt --all --check` — PASS.
- `cargo fuzz run ui_redaction_artifact -- -runs=1` — FAIL: target is registered, but the environment builds fuzzing for `x86_64-unknown-linux-musl`; sanitizer build failed with `sanitizer is incompatible with statically linked libc, disable it using -C target-feature=-crt-static`.
- `moon run velvet-ballastics:test` — FAIL before tests in `velvet-ballastics:supply-chain`: cargo-vet rejected newly resolved fuzz dependencies `arbitrary:1.4.2`, `jobserver:0.1.34`, and `libfuzzer-sys:0.4.12` as unvetted, with existing allowed advisory warnings also printed.

## Residual risks and skipped gates
- `cargo fuzz run ui_redaction_artifact -- -runs=1` did not execute the fuzzer body because the local cargo-fuzz sanitizer target is incompatible with static musl libc.
- `moon run velvet-ballastics:test` remains blocked by supply-chain vetting for cargo-fuzz transitive dependencies introduced while wiring the target.
- No performance claim was made; no benchmark or profiler was run.
- Full `moon ci`, full workspace clippy, Miri, Kani, coverage, mutation, and supply-chain acceptance were not completed in this repair pass.
