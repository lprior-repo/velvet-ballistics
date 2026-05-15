# Red Phase Report: vb-qi37.4.1

## Files changed

- `Cargo.toml`
  - Removed a duplicate `proptest.workspace = true` dev-dependency entry that prevented Cargo metadata from loading.
- `crates/vb_storage/tests/accepted_artifact_red_phase.rs`
  - Added executable red-phase integration tests for the accepted-artifact v1 boundary using current public storage/admission APIs.
- `benches/aggregate_resource_budget.rs`
  - Added the missing Criterion bench target declared by the workspace manifest so Cargo can resolve the bench target.
- `.beads/vb-qi37.4.1/red-phase.md`
  - This report.

## Intended failing test command

```bash
cargo nextest run -p vb_storage --test accepted_artifact_red_phase
```

## Verification attempted

The command above was run from `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`.

Current result: Cargo reaches `vb_storage` compilation but is blocked by pre-existing production compile errors in `crates/vb_storage/src/batch.rs`:

- `HashSet<Vec<u8>>::contains(&[u8; 17])` trait mismatch.
- `HashSet<Vec<u8>>::insert([u8; 17])` type mismatch.

No production implementation for the accepted-artifact contract was added.

## Why failures are expected before implementation

The tests assert the approved v1 contract, while current public code still implements the legacy two-gate compiled-workflow artifact path:

- Warning gate upper bound is still 13, but v1 requires 15.
- `submit_artifact` stores raw `CompiledWorkflow` parts in `CompiledIrRecord.ir`, but v1 requires a nested accepted-artifact payload.
- Journaled/Strict artifact proof gate count is still 2, but v1 requires 15.
- Relaxed raw submission still succeeds, but v1 required-mode admission must reject raw submit with `AdmissionRequired`.
- Runtime admission error taxonomy (`ArtifactInvalid`, `InputTooLarge`, `CapabilityDenied`, `SecretUnavailable`, durability failures, clock failure) is not yet surfaced by this boundary.

These tests should turn green only after the accepted-artifact envelope, real compiled-IR load validation, and runtime admission boundary are implemented.
