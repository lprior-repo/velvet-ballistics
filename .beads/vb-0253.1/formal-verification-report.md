# Formal Verification Report - vb-0253.1

STATUS: APPROVED

## Executed Obligations
- `KANI-QUEUE-001`: `cargo kani -p vb_runtime --harness command_queue_bounds` -> PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 3 failed`, against the `#[cfg(kani)]` queue model/shared capacity predicate rather than production `ArrayQueue` mutation.
- `VERUS-INV-001`: WAIVED via `formal-waivers.jsonl`.
- `VERUS-INV-002`: WAIVED via `formal-waivers.jsonl`.

## Machine Gates
- `cargo test -p vb_runtime command_queue -- --nocapture` -> PASS, `11 passed, 1450 filtered out`.
- `cargo check -p vb_runtime` -> PASS.
- `cargo fmt --check` -> FAIL_REGRESSION/DEFERRED_GLOBAL, unrelated pre-existing formatting diffs in `crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs` and `xtask/src/forbidden_scan.rs`.
