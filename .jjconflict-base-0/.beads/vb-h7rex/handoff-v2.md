# vb-h7rex: Policy Pin Constructor Compatibility and Checkpoint Schema Repair

**Bead:** vb-h7rex
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-h7rex-local5090-admission-pin
**Commit:** WIP c29e7b8a (preserved, unaccepted)

## Changes Made

### 1. RunAdmission Constructor Backward Compatibility

**Source files modified:**
- `crates/vb_runtime/src/admission/parts/chunk_002_records.rs`

**Changes:**
- Restored original 4-arg `RunAdmission::new(digest, run_id, caps, policy)` signature
  - Explicitly defaults to `RunPolicyPin::Unverified`
- Restored original signatures for `with_idempotency_evidence` and `with_budget`
- Added new builder method `with_policy_pin(RunPolicyPin)` for verified artifact paths
- Added `#[serde(default = "run_policy_pin_default")]` to `policy_pin` field for old checkpoint compatibility

### 2. RunPolicyPin Import Fix

**Source files modified:**
- `crates/vb_runtime/src/admission.rs` (added `use vb_storage::RunPolicyPin;` - already present)
- `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs` - removed explicit policy_pin args from constructor calls, use `with_policy_pin` builder instead
- `crates/vb_runtime/src/admission/tests.rs` - removed RunPolicyPin arg from legacy tests
- `crates/vb_runtime/src/run_checkpoint_tests.rs` - removed RunPolicyPin arg
- `crates/vb_runtime/src/ipc_refinement.rs` - added import for test that constructs RunAdmission
- `crates/vb_runtime/src/recovery.rs` - use `with_policy_pin` builder for hydration
- `crates/vb_runtime/tests/recovery_bdd_tests.rs` - added missing `policy_pin` field to JournalEvent::RunAdmission test helper
- `crates/vb_runtime/tests/recovery_hydration_tests.rs` - added missing `policy_pin` field to test helper
- `crates/vb_runtime/src/journal/tests/chunk_001.rs` - added missing `policy_pin` field to JournalEvent test

### 3. RunAdmission Constructor Callers Updated

**Source files modified:**
- `crates/vb_runtime/src/admission.rs` - `admit_run_with_budget_policy` removed policy_pin arg
- `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs` - `admit_run` and `admit_artifact_run` updated
  - `admit_artifact_run` uses `with_policy_pin` builder to set actual Pinned policy pin
  - Relaxed policy path remains Unverified
- `crates/vb_runtime/src/recovery.rs` - `hydrate_run_admission_from_events` uses `with_policy_pin`

### 4. JournalEvent::RunAdmission Schema

**No changes required** - `JournalEvent::RunAdmission` already includes `policy_pin` field.
Recovery tests updated to include the field.

## Gate Results

### Test Gate
- `python3 /tmp/vb-run-gate.py 300 .beads/vb-h7rex/evidence/REPAIR_V2_TESTS cargo test -p vb_runtime`
- Exit code: 0
- Duration: 2.68s
- All 1973 tests passed

### Clippy Gate
- `python3 /tmp/vb-run-gate.py 300 .beads/vb-h7rex/evidence/REPAIR_V2_CLIPPY cargo clippy -p vb_runtime -p vb_storage --lib -- -D warnings`
- Exit code: 0
- Duration: 0.10s
- No warnings or errors

## Checkpoint Compatibility Behavior

### New Checkpoints
- Serialize with `policy_pin` field set to actual policy identity (`Pinned(digest)`) for Strict/Journaled admission, or `Unverified` for Relaxed
- `RunPolicyPin` enum is serialized inline within the RunAdmission struct

### Old Checkpoints (Backward Compatibility)
- Historical checkpoints serialized before `policy_pin` field existed are handled via `#[serde(default = "run_policy_pin_default")]` attribute
- Old checkpoints deserialize with `RunPolicyPin::Unverified` - explicit compatibility state, not a new authority
- New checkpoint format preserves `policy_pin` through roundtrip (verified by test)

## Tests Added

1. `admission_legacy_constructor_produces_explicit_unverified_pin` - compatibility constructor sets Unverified
2. `admission_with_policy_pin_builder_sets_actual_pin` - builder sets actual policy identity
3. `admission_strict_journaled_admit_artifact_run_sets_pinned_policy_pin` - Strict/Journaled admission sets Pinned pin with artifact's policy_digest
4. `admission_relaxed_admit_run_keeps_unverified_pin` - Relaxed admission remains Unverified
5. `checkpoint_preserves_pinned_policy_pin_through_roundtrip` - new checkpoints preserve pin through serialize/deserialize
6. `checkpoint_deserializes_legacy_admission_with_unverified_pin` - old checkpoints decode as Unverified

## Remaining Gap (Out of Scope for This Bounded Slice)

- Public Runtime/Fjall end-to-end integration: not implemented. Requires dispatch/effect system integration (tag 38 work, separate worker). The core admission and checkpoint schema compatibility is complete; the public Fjall binding layer is a follow-up.

## Verification

- All tests compile and pass
- Raw strict clippy check passes with no warnings
- Constructor compatibility verified: all existing callers compile without changes
- Serialization compatibility verified: old checkpoint format decodes correctly with default
- Policy pin behavior verified: legacy constructors produce Unverified, artifact admission produces Pinned
