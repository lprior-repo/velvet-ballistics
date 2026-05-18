bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 10
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Implementation Summary

### Changes Made

1. **`crates/vb_storage/src/recovery/recover.rs`**:
   - Added `check_action_abi_digests(entries: &[(ActionId, WorkflowDigest, WorkflowDigest)])` — pure comparison function that returns `ActionAbiMismatch { action_id }` on first mismatch
   - Added `check_policy_digests(entries: &[(StepIdx, WorkflowDigest, WorkflowDigest)])` — pure comparison function that returns `PolicyDigestMismatch { step }` on first mismatch
   - Updated `verify_digests` doc comment to direct callers to the new functions for ABI/policy checks
   - Added imports for `ActionId` and `StepIdx` from `vb_core`

2. **`crates/vb_storage/src/recovery/mod.rs`**:
   - Added re-exports for `check_action_abi_digests` and `check_policy_digests`

3. **`crates/vb_storage/tests/recovery_bdd_tests.rs`**:
   - Unignored and rewrote `action_abi_mismatch_returns_typed_error` — now asserts exact error variant and action_id
   - Unignored and rewrote `policy_digest_mismatch_returns_typed_error` — now asserts exact error variant and step
   - Added `action_abi_match_returns_ok` — verifies no false positives
   - Added `policy_digest_match_returns_ok` — verifies no false positives
   - Added `check_action_abi_digests_empty_input_returns_ok` — verifies EARS-3 (no guessing)
   - Added `check_policy_digests_empty_input_returns_ok` — verifies EARS-3 (no guessing)
   - Updated imports to include new functions

### Design Decisions

- **Explicit verifier inputs over journal lookup**: Since `JournalEvent::ActionScheduled` and `RunAdmission` don't carry ABI/policy digests, the functions accept `(id, expected, found)` tuples from the caller. This follows EARS-3: "expose an explicit verifier input rather than silently returning Ok."
- **Standalone functions over extended verify_digests**: Keeping `verify_digests` at its existing parameter count (6 params, already over the 5-param guideline) and adding separate functions keeps each function focused and composable.
- **Pure comparison logic**: Both new functions are pure — no I/O, no journal access. The caller provides both sides of the comparison.

### Contract Mapping

| Contract Clause | Implementation |
|---|---|
| EARS-1: ActionAbiMismatch on mismatch | `check_action_abi_digests` returns `RecoveryError::ActionAbiMismatch { action_id }` |
| EARS-2: PolicyDigestMismatch on mismatch | `check_policy_digests` returns `RecoveryError::PolicyDigestMismatch { step }` |
| EARS-3: Explicit verifier input | Both functions take explicit `entries` tuples |
| INV-1/2: No false positives on missing data | Empty input returns `Ok(())` |
| INV-3: Matching digests succeed | Equality check returns `Ok(())` |
| INV-4: Exact identifiers in errors | Error variants carry exact `action_id` / `step` |

### Holzman Rust Compliance

- `#![forbid(unsafe_code)]` — no unsafe
- No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`
- Functions under 25 lines (both are 10 lines)
- Max 5 parameters (both have 1 parameter)
- Pure logic separated from I/O
- Typed errors, checked access, functional patterns
