# Code Review: VB-FG1PI Verus Mirror Refinement

**Reviewed source SHA:** 8a7e2340f14f739c57937c9cd2d06d3d8d3010a8
**Base SHA:** bfb8bac3
**Reviewer:** local5090 (independent)
**Date:** 2026-10-21
**Verus version:** 0.2026.05.05 (Rust 1.95.0)

## Status: REJECTED

Binding fidelity gaps found in try_from_parts spec model. All files compile and pass Verus verification, but the spec's decision function is incomplete relative to production's validation pipeline.

## Summary

This review covers the Verus mirror refinement for bead VB-FG1PI, authored by DeepSeek. The changes update 13 files across extern surfaces and production_inner mirrors to reflect production API drift. The work correctly mirrors new `CompiledInputSlot`, `InputSlotKind`, `ExprBranch`/`SlotBranch` types, new `semantic_source_identity` field, new `validate_input_slots` function, and `WorkflowError` variant set expansion to 23 variants. The `CliExitCode::LifecycleError` variant and `RunInputBinding`/`RetryCheckpoint` journal event additions are also properly mirrored.

All 8 changed extern files pass Verus verification individually. Production inner drift and binding checks pass. However, the spec's validation decision function is incomplete.

## Verus Verification Results

All files pass with exit code 0, default lifetime checks enabled:

| File | Exit | Time |
|------|------|------|
| extern_try_from_parts.rs | 0 | 0.449s |
| extern_budget_monotonic.rs | 0 | 0.287s |
| extern_ipc_runtime_transitions.rs | 0 | 0.268s |
| extern_recovery_verification.rs | 0 | 0.267s |
| extern_storage_kind_family.rs | 0 | 0.262s |
| extern_vb_core_replay_step.rs | 0 | 0.253s |
| extern_vb_jnz9_journal_event_seq_valid.rs | 0 | 0.249s |
| extern_vb_jpq724_events_for_run_production.rs | 0 | 0.263s |

Drift/binding checks from workspace root:
- `bash scripts/check-production-inner-drift.sh`: exit 0, 3.258s
- `bash scripts/check-verus-production-binding.sh`: exit 0, 0.413s

## Production Fidelity Verification

Verified all critical bindings against current production source:

- `CompiledWorkflow::try_from_parts` — production calls `validate_parts`, `validate_budget`, `validate_input_slots`. Mirror calls all three. ✓
- `WorkflowParts` struct — 12 fields including new `semantic_source_identity` and `input_slots`. Mirror matches. ✓
- `CompiledWorkflow` struct — 12 fields match. ✓
- `WorkflowError` — production has 23 variants. Mirror has 23 variants with identical names/fields. ✓
- `InputSlotKind` — production 9 variants at workflow/mod.rs:301-320. Mirror matches. ✓
- `validate_input_slots` — production at workflow/mod.rs:876-905 is real validation (not stub). Mirror provides stub returning `Ok(())` which is acceptable for the proof model. ✓
- `JournalEvent::RetryCheckpoint` — production variant at events/event.rs:63. Mirror includes variant with `attempt()` method handling. ✓
- `RunInputBinding` — production variants `LegacyUnbound`/`Digest(WorkflowDigest)`. Mirror matches. ✓
- `CliExitCode` — production has 10 variants. Mirror matches. ✓
- `try_from_parts` production source lines updated: 37-62 (was 33-51). ✓

## Findings

### BLOCKER: Spec decision function missing validate_input_slots

The try_from_parts spec's `try_from_parts_pure` function (in `extern_try_from_parts.rs`) documents and models only the production validation order:
1. `validate_parts`
2. `validate_budget`

Production now calls `validate_input_slots` as the third validation step. The spec's decision function is missing this step entirely. The binding ledger mentions `validate_input_slots` (line 59-60) but the actual decision function does not model it. This means the proofs do not cover the input slot validation path that production executes.

The production validation function `validate_input_slots` at workflow/mod.rs:876-905 performs three distinct checks:
- InputSlotOutOfBounds: slot index >= slot_count
- DuplicateInputSlot: duplicate slot indices
- NonCanonicalInputSlotOrder: slots not in strictly increasing order

These validation failures are not represented in the spec's validation decision function.

### BLOCKER: Spec discriminant set incomplete

The `SpecWorkflowError` enum (in `extern_try_from_parts.rs`) has 18 variants. Production has 23. The three new variants are missing:
- `InputSlotOutOfBounds` (production variant 21)
- `DuplicateInputSlot` (production variant 22)
- `NonCanonicalInputSlotOrder` (production variant 23)

Without these in the spec's closed-set discriminants, the validation proofs cannot express or reason about input slot validation failures.

### MINOR: validate_input_slots binding not in validation order documentation

The doc comment for `try_from_parts_pure` describes the production validation order as:
```
1. validate_parts(&parts)?
2. validate_budget(&parts)?
```

It should describe:
```
1. validate_parts(&parts)?
2. validate_budget(&parts)?
3. validate_input_slots(&parts)?
```

### ACCEPTED APPROXIMATION: Digest vs semantic_source_identity

The vb_xi2f_compile_source_production.rs projection passes the same digest value for both `digest` and `semantic_source_identity`. Production distinguishes these:
- `semantic_source_identity = canonical_digest(source)` — normalized source semantics
- `digest = blake3(encoded_parts)` — compiled artifact integrity

The projection cannot compute opaque crypto hashes and encodings, so this conflation is a documented projection limitation. Acceptable for the proof model.

### ACCEPTED APPROXIMATION: validate_input_slots stub

The production_inner mirror provides a stub `validate_input_slots` returning `Ok(())`. The actual validation logic is modeled in the spec's `try_from_parts_pure` decision function. This matches the established pattern for validation functions in this repo (validate_parts and validate_budget are also stubs in the mirror).

## Recommendation

The code should be rejected on BLOCKER grounds until:
1. `try_from_parts_pure` is extended to model the `validate_input_slots` validation step (or the decision function's trust boundary documentation is updated to explicitly state this approximation).
2. `SpecWorkflowError` is extended with the three missing input slot error variants.

Once those are addressed, the mirror refinement is structurally sound and production-faithful.

## Review Limits

- Kani parent delta reviewed separately (not in scope).
- Full moon ci and Kani fleet not run (out of scope per bead assignment).
- Production code itself not modified; review covers Verus mirror artifacts only.