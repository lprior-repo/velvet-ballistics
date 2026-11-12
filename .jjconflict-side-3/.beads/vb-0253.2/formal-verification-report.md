# State 11 Formal Verification Report

STATUS: APPROVED

PASS:
- `cargo kani -p vb_ipc --harness kani_ipc_header_decode_valid --quiet` -> `exit_code=0`.
- `rtk cargo clippy -p vb_ipc --lib -- -D warnings` -> `No issues found`.
- `rtk cargo test -p vb_ipc` -> `626 passed`.

WAIVED:
- Raw Verus ingress command failed to resolve Cargo dependencies. Waived as invalid lane selection for this facade-only refactor.

DEFERRED_GLOBAL:
- `moon run velvet-ballistics:check` failed in unrelated `crates/vb_storage/tests/recovery_bdd_tests.rs` unused warnings.
