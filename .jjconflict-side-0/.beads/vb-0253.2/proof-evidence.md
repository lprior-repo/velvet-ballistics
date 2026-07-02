# Proof Evidence

Kani:
- Command: `cargo kani -p vb_ipc --harness kani_ipc_header_decode_valid --quiet`
- Result: PASS, `exit_code=0`.

Verus:
- Command: `verus crates/vb_ipc/src/ingress.rs`
- Result: FAIL_ENV. Raw file invocation cannot resolve Cargo crates (`bytes`, `crossbeam_channel`, `vb_core`) or facade re-exports outside Cargo context.
- Classification: WAIVED for this bead by State 6 because implementation changed module ownership/re-exports, not queue algorithms.

Executable substitute evidence:
- `rtk cargo test -p vb_ipc` -> `628 passed` after rebase onto `main`.
- `rtk cargo clippy -p vb_ipc --lib -- -D warnings` -> `No issues found`.
