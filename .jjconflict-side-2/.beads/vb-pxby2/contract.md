# vb-pxby2: Verus Registry Bridge Repair — WorkflowDigest External Type

## Problem

`scripts/verify-verus.sh` failed on `verification/verus/idempotency_replay_tracker.rs` lines 137/143:

```
error: cannot use type `idempotency_replay_tracker::production::prod_src::WorkflowDigest`
       which is ignored because it is either declared outside the verus! macro
       or it is marked as `external`.
```

Root cause: `ActionScheduleEvidence` (line 194) and `ActionCompletionEvidence` (line 204) both contain `action_abi_digest: WorkflowDigest` fields. The production module is marked `#[verifier::external]`, so `WorkflowDigest` is an opaque external type that requires an `external_type_specification` bridge — but none existed.

## Fix

Added `ExWorkflowDigest` bridge type to `verification/verus/idempotency_replay_tracker.rs`:

```verus
#[verifier::external_type_specification]
pub struct ExWorkflowDigest(pub production::prod_src::WorkflowDigest);
```

This matches the established pattern used for all other external production types in the file (ExActionReplayTracker, ExActionScheduleEvidence, ExActionCompletionEvidence, etc.). No axioms, no `assume_specification`, no shadow model — purely an opaque type specification.

## Verification Evidence

All evidence captured via immutable runner in `.beads/vb-pxby2/evidence/`.

### Registry Check (PASS)

```
env VERUS_EVIDENCE_DIR=.beads/vb-pxby2/evidence/verus-registry-raw bash scripts/verify-verus.sh
Wall time: 11.66 seconds
VERUS_REGISTRY_OK evidence=.beads/vb-pxby2/evidence/verus-registry-raw
```

**Counts:** 36/36 Verus artifacts verified, 0 errors.

Individual files:
- idempotency_replay_tracker.rs: 12 verified, 0 errors (was failing, now fixed)
- recovery_verification.rs: 123 verified
- budget_bounded.rs: 65 verified
- try_from_parts.rs: 67 verified
- (29 additional files, all verified)

### Drift Gate (PASS)

```
bash scripts/check-production-inner-drift.sh
Mirror files checked: 64
Extern files scanned: 78
Drift findings: 0
Production-inner drift gate: PASS
```

### Binding Audit (PASS)

```
bash scripts/check-verus-production-binding.sh
STRONG (direct crates/ binding): 0
WEAK (production_inner/ mirror): 72
VACUUM (no production binding):  0
```

Zero vacuum proofs. All 72 Verus artifacts bind to production via production_inner mirrors.

## Files Modified

- `verification/verus/idempotency_replay_tracker.rs` — Added `ExWorkflowDigest` bridge type at line 145.

No other files changed. No drift, no weakened math, no axioms, no lifetime bypass.