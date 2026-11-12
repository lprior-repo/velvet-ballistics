# Proof Strategy: vb-b8i8f Cancel/Kill Lattice Recovery

## Scope

- Bead: vb-b8i8f
- State: 4 / proof-planner
- Base: State 3 contract, 5 proof seeds, 6 traceability requirements
- No TLA+ per global removal mandate.

## Risk Profile Summary

| Seed | Requirement | Primary Risks | Required Lanes |
|------|-------------|---------------|----------------|
| vb-b8i8f-seed-001 | REQ-cancel-kill-live-only | public-api, typed-error, rust-local | Verus, Kani, Flux-rs, proptest |
| vb-b8i8f-seed-002 | REQ-single-terminal-winner | single-terminal, idempotency, rust-local | Verus, Kani, Flux-rs, proptest |
| vb-b8i8f-seed-003 | REQ-stale-authority-cleanup | stale-authority, rust-local | Verus, Kani, Flux-rs, proptest |
| vb-b8i8f-seed-004 | REQ-runkilled-kind28-admission | storage, codec, data-loss | Verus, Kani, Flux-rs, proptest, cargo-fuzz |
| vb-b8i8f-seed-005 | REQ-replay-ordinal-killed | replay, storage, ordinal | Verus, Kani, Flux-rs, proptest, cargo-fuzz |

## Non-Applicable Lanes

| Verifier | Reason | Evidence |
|----------|--------|----------|
| Loom | No concurrency, atomics, channels, locks, async shutdown, or task ownership in scope. Shard processing is single-threaded via command queue. | `#![forbid(unsafe_code)]` in runtime.rs; ShardCommand queue is serialized via single-threaded tick. |
| Miri | No `unsafe` blocks, FFI, raw pointers, aliasing, or provenance risk in any touched source file. All files use `#![forbid(unsafe_code)]`. | `crates/vb_runtime/src/runtime.rs` line 1; `crates/vb_storage/src/records.rs` line 1; `crates/vb_storage/src/events.rs` line 1; `crates/vb_storage/src/codec/validation.rs` uses safe Rust only. |

## Lane Strategy by Verifier

### Verus
- Model the pure lifecycle state machine: `Live | Terminal(kind) | Missing` with transition rules for Cancel/Kill.
- Model the codec kind-family invariant: `MAGIC_JOURNAL_EVENT` accepts `10..=28`.
- Models must name production source refs and bind to Rust `exec fn` behavior through bridge obligations.
- Workdir: `crates/vb_proof_kernels/` or inline in target crate under `verification/verus/`.

### Kani
- Bounded proofs on `is_known_record_kind(28)`, `validate_kind_family(MAGIC_JOURNAL_EVENT, 28)`, cancel/kill handler live-only checks, and single-terminal-winner state machine.
- Harnesses must use `kani::any()` for RunId, kind values; no hardcoded structural inputs.
- Workdir: `crates/vb_storage/` for codec; `crates/vb_runtime/` for lifecycle.

### Flux-rs
- Refinements on `validate_kind_family` postcondition: `magic == MAGIC_JOURNAL_EVENT && kind ∈ 10..=28 → Ok(())`.
- Refinements on cancel/kill: `terminalized(run) → handle_cancel/handle_kill returns Err`.
- Workdir: `crates/vb_storage/` and `crates/vb_runtime/`.

### proptest
- Property: for any RunId, cancel then cancel again returns error (no double terminal).
- Property: encode/decode round-trip for `JournalEvent::RunKilled` succeeds.
- Property: `is_known_record_kind(28) == true` holds.
- Workdir: `crates/vb_storage/` and workspace tests.

### cargo-fuzz
- Fuzz `validate_kind_family` with arbitrary (magic, kind) pairs to ensure only valid families pass.
- Fuzz `decode_record::<JournalEvent>` with arbitrary bytes to ensure kind 28 decode does not panic.
- Workdir: `crates/vb_storage/fuzz/` or `fuzz/` root.

## Trusted Base Plan
- `RecordKind::RunKilled = 28` is normative; wire compatibility with existing data relies on Fjall's append-only nature and postcard's stable encoding.
- `MAGIC_JOURNAL_EVENT` constant is trusted as the authoritative journal family discriminator.
- Existing Kani harnesses in `crates/vb_runtime/src/verification/kani/` may provide reusable scaffolding but must be reviewed against contract requirements.
- `postcard` crate is trusted for correct encode/decode within valid payload bounds.

## Waiver Candidates
- None. All behavior-affecting proof seeds require full proof coverage. No non-behavior exceptions needed.

## Implementation Bridge Preparation
- Every obligation names exact production source files and symbols.
- Bridge will map Verus/Kani/Flux/proptest claims to behavior tests in `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs`.
- Refinement harnesses will target `validate_kind_family`, `is_known_record_kind`, `handle_cancel`, `handle_kill`, and `run_storage_event`.
