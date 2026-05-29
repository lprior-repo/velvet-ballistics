# Proof-to-Implementation Input — vb-7m21 (Replan, Reduced Scope)

## Bridge Inputs

- Use `proof-obligations.planned.jsonl` as the machine-readable obligation source.
- Use `verifier-lane-decisions.jsonl` for required vs not-applicable lane rationale.
- Behavior implementation must map proof claims to `crates/vb_storage` and `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` source refs.

## Required Mapping Rules

- Every required obligation needs a downstream rust-refinement-obligation/v1 row in State 7.
- Kani harnesses must use `kani::Arbitrary`/`kani::any()` or safe exhaustive generators, never one fixed dummy shape.
- Behavior-affecting waivers are forbidden.
- TLA+ claims are excluded from this plan scope; no TLA+-to-Rust mapping needed.

## Scope Context

This is a **test-first bead** (reduced scope replan). The primary deliverable is a test file: `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`. No new production implementation functions are in scope. The existing `vb_storage` public API is the target of test assertions.

Verus, Flux, and TLA+ are excluded because they require production implementation targets that do not exist in this delivery scope. The proptest obligations are the primary verification mechanism; Kani and fuzz complement codec-boundary seeds.

## Source Targets Referenced by Contract

- crates/vb_storage/src/codec/header.rs
- crates/vb_storage/src/codec/payload.rs
- crates/vb_storage/src/codec/validation.rs
- crates/vb_storage/src/constants.rs
- crates/vb_storage/src/keys.rs
- crates/vb_storage/src/indexes.rs
- crates/vb_storage/src/journal/core.rs
- crates/vb_storage/src/journal/internal.rs
- crates/vb_storage/src/journal/replay.rs
- crates/vb_storage/src/snapshots.rs
- crates/vb_storage/src/recovery/types.rs
- crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs

## Planned Obligations (Reduced Scope — 14 Total)

### Kani (3)

- **PO-vb-7m21-kani-001**: kani → `crates/vb_storage/src/kani_vb_7m21_001.rs` → `cargo kani -p vb_storage --harness vb_7m21_001_harness` (codec panic-freedom, PS-001, REQ-5)
- **PO-vb-7m21-kani-002**: kani → `crates/vb_storage/src/kani_vb_7m21_002.rs` → `cargo kani -p vb_storage --harness vb_7m21_002_harness` (header validation, PS-002, REQ-3)
- **PO-vb-7m21-kani-003**: kani → `crates/vb_storage/src/kani_vb_7m21_003.rs` → `cargo kani -p vb_storage --harness vb_7m21_003_harness` (payload bounds, PS-003, REQ-6)

### Proptest (8)

- **PO-vb-7m21-prop-001**: proptest → `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` → `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` (fixture gen: oversized payload, PS-001, REQ-5)
- **PO-vb-7m21-prop-002**: proptest → `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` → `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` (fixture gen: unknown schema, PS-002, REQ-3)
- **PO-vb-7m21-prop-003**: proptest → `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` → `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` (fixture gen: truncated header, PS-003, REQ-6)
- **PO-vb-7m21-prop-004**: proptest → `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` → `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` (fixture gen: missing side-index, PS-004, REQ-4)
- **PO-vb-7m21-prop-005**: proptest → `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` → `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` (fixture gen: journal gap, PS-005, REQ-8)
- **PO-vb-7m21-prop-006**: proptest → `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` → `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` (fixture gen: duplicate event, PS-006, REQ-9)
- **PO-vb-7m21-prop-007**: proptest → `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` → `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` (fixture gen: stale snapshot, PS-007, REQ-10)
- **PO-vb-7m21-prop-008**: proptest → `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` → `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` (fixture gen: missing manifest, PS-008, REQ-11)

### Cargo-Fuzz (3)

- **PO-vb-7m21-fuzz-001**: cargo-fuzz → `fuzz/fuzz_targets/vb_7m21_storage_envelope.rs` → `cargo fuzz run vb_7m21_storage_envelope -- -max_total_time=60 -runs=10000` (envelope decode, PS-001, REQ-5)
- **PO-vb-7m21-fuzz-002**: cargo-fuzz → `fuzz/fuzz_targets/vb_7m21_storage_envelope.rs` → `cargo fuzz run vb_7m21_storage_envelope -- -max_total_time=60 -runs=10000` (header parse, PS-002, REQ-3)
- **PO-vb-7m21-fuzz-003**: cargo-fuzz → `fuzz/fuzz_targets/vb_7m21_storage_envelope.rs` → `cargo fuzz run vb_7m21_storage_envelope -- -max_total_time=60 -runs=10000` (payload decode, PS-003, REQ-6)

## Downstream Dependencies

- **REQ-4 (PS-004)**: `IndexParityMismatch` does not exist in located `JournalError` taxonomy. Downstream implementation must decide: add error variant to `vb_storage`, add fixture-runner-local error enum, or map to existing error. Bridge must confirm resolution.
- **REQ-9 (PS-006)**: `duplicate idempotency key` is not a located storage public concept. Downstream must decide whether fixture uses duplicate event sequence (`DuplicateEvent`) or runtime/admission idempotency surface.
- **REQ-11 (PS-008)**: `missing manifest` is ambiguous; downstream must inspect existing manifest tests (`restate_fjall_keyspace_manifest_tests.rs`) before implementation.
- **REQ-16 (PS-009)**: No-copy fence is a review obligation only. Bridge must ensure source/provenance review in downstream states.
