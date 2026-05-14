# Contract: vb-h8h0 - Codegen Equivalence Must Verify Execution

## Requirement

`compare_generated_to_ir` at `crates/vb_codegen/src/lib.rs:2133` performs source-pattern counting only. It must verify actual execution equivalence: terminal results, taint states, journal events, and errors.

## Non-Goals

- Not replacing codegen (yet)
- Not adding full property tests for all workflow shapes

## Constraints

1. Generated Rust must compile under pinned nightly
2. No unsafe, unwrap, expect, panic
3. Existing test infrastructure should be reused

## Verification Criteria

| ID | Criterion | File | Command |
|----|-----------|------|---------|
| CODEGEN-001 | Execution equivalence for terminal results | `crates/vb_codegen/src/proptests.rs` | `cargo test fixed_six_step_emitted_rust_and_ir_match_finished_signal_and_slots` |
| CODEGEN-002 | Taint parity verified | `crates/vb_codegen/src/lib.rs` | `cargo test` |
| CODEGEN-003 | Journal event parity verified | `crates/vb_codegen/src/lib.rs` | `cargo test` |
| CODEGEN-004 | Error parity verified | `crates/vb_codegen/src/lib.rs` | `cargo test` |