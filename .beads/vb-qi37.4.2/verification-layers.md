# Verification Layers: vb-qi37.4.2

## Boundary

- **Verus-owned kernel**: None (waived — deterministic Rust control flow)
- **TLA+ temporal model**: None (waived — single atomic step function)
- **Theorem projection**: None (no algebraic kernels)
- **Runtime shell**: `handle_submit_with_inputs_contracts_and_header_mode` — single-shard, no concurrency
- **External systems**: None — fully in-process

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Waiver Rationale |
|----------------|--------------|----------------|------------------|
| INV-001 (run never inserted on rejection) | `integration_test` | `miri` | Deterministic Rust control flow; behavioral test is sufficient |
| INV-002 (sequencing order) | `integration_test` | `miri` | Linear step function; no branching/temporal behavior |
| ERR-Rejection (exhaustive errors) | `integration_test` | N/A | Each error variant tested by integration test |
| POST-002 (no state change on rejection) | `integration_test` | `miri` | Behavioral verification via `active_run_count` assertion |

## Integration Test Plan

### New Test: `admission_rejection_does_not_insert_run_state_strict`

**Location**: `crates/vb_runtime/src/shard/lifecycle_tests/chunk_003.rs`

**Setup**:
- `ShardConfig { policy: RuntimePolicy::Strict }` (NOT Relaxed)
- `NeverPresentArtifactStore` (implements `AcceptedArtifactStore`, always returns `ArtifactEnvelopeError::ArtifactNotFound`)

**Execution**:
1. `shard.enqueue(ShardCommand::Submit { run, workflow, caps })`
2. `shard.tick()`

**Assertions**:
- `shard.active_run_count() == 0` (run NOT inserted)
- `shard.counters().snapshot().runs_submitted == 0` (no submission counted)
- Return value is `Err(RuntimeError::AdmissionArtifactNotFound { digest })`

### New Test: `admission_rejection_does_not_insert_run_state_journaled`

Same as above but `RuntimePolicy::Journaled`.

### New Test: `admission_capability_mismatch_does_not_insert`

**Setup**:
- `RuntimePolicy::Strict`
- Artifact with non-empty `required_capabilities`
- `CapabilitySet::empty()` passed to submit

**Assertions**: Same as above with `RuntimeError::AdmissionCapabilityDenied`

### Existing Test Fix: `admission_rejection_does_not_insert_run_state`

The current test uses Relaxed policy and asserts run IS inserted. This test name is misleading — it should be split:
- Rename current test to `admission_relaxed_always_inserts` (existing behavior, keep as regression test)
- New test `admission_rejection_does_not_insert_run_state` uses Strict policy + NeverPresentArtifactStore

## Miri Scope

`MIRIENV='-Zmiri-strict-provenance=y' cargo miri test admission_rejection_does_not_insert_run_state_strict`

Miri verifies:
- No UB in the rejection path
- No use-after-free, no invalid values
- No aliasing violations through the `?` propagation

## Proptest Scope

None required — the rejection behavior is a deterministic branch on policy/store combination, not a property requiring random input exploration.

## Kani Scope

None required — no numeric bounds, indexing, or state transition properties beyond what integration tests cover.

## Fuzzing Scope

None required — the admission gate is a deterministic function of (store, policy, digest, caps), not a parser or data transformation.

## Loom/Shuttle Scope

None required — single-shard execution with no concurrency, no thread interleavings, no cancellation.

## Static Scan Scope

`cargo clippy --workspace --lib --bins -- -D warnings` on vb_runtime covers:
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`
- No unchecked indexing or arithmetic
- Error propagation is explicit via `?`

## Performance Scope

None — no performance-critical path in the rejection test.

## Assembly/IR Scope

None — no zero-cost abstraction or vectorization claims.

## API Compatibility Scope

None — no public API changes in this bead.

## Release Provenance Scope

None — test-only bead.

## Waivers

| Clause | Owner | Reason | Compensating Evidence |
|--------|-------|--------|----------------------|
| TLA+ temporal model | vb-qi37.4.2 | Single atomic step function; no temporal/state-over-time behavior | Integration test + Miri |
| Verus proof | vb-qi37.4.2 | Deterministic Rust control flow; integration test is sufficient | Integration test + Miri |
| Theorem kernel | vb-qi37.4.2 | No algebraic state transitions or arithmetic bounds beyond tests | N/A |
| Kani | vb-qi37.4.2 | No numeric/indexing/state-transition properties beyond integration test | N/A |
| Fuzzing | vb-qi37.4.2 | Deterministic function of store/policy; no parser or adversarial input | N/A |
| Loom/Shuttle | vb-qi37.4.2 | Single-shard, no concurrency | N/A |
