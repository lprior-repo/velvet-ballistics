# Handoff: vb-pxby2 (LOCAL5090 → root)

**Status:** Ready for code/test review by root.

## What Changed

Single opaque type specification added to `verification/verus/idempotency_replay_tracker.rs`:

```verus
/// Spec-mode name for the production `WorkflowDigest` type
/// (used as a field in `ActionScheduleEvidence` and
/// `ActionCompletionEvidence`).
#[verifier::external_type_specification]
pub struct ExWorkflowDigest(pub production::prod_src::WorkflowDigest);
```

Location: between `ExActionCompletionEvidence` (line 143) and `ExActionTicket` (line 145). Follows the exact pattern of all other external type bridges in the file.

## Why

`ActionScheduleEvidence` and `ActionCompletionEvidence` both have `action_abi_digest: WorkflowDigest` fields. Under `#[verifier::external]`, production types referenced by name in spec mode require opaque type specification bridges. The fix is a one-line addition matching Verus' own suggestion.

## Verification Results

All via immutable runner, full evidence in `.beads/vb-pxby2/evidence/`.

| Gate | Result | Detail |
|------|--------|--------|
| Verus registry | PASS | 36/36 artifacts, 0 errors, 11.66s |
| Production drift | PASS | 64 mirrors, 78 externs, 0 drift |
| Binding audit | PASS | 0 vacuum, 72 weak (production_inner) |

The specific failing file `idempotency_replay_tracker.rs` now verifies: 12 verified, 0 errors.

## No Other Changes

- No axioms, no `assume_specification`, no shadow model
- No lifetime bypass
- No weakened math
- No formatting of untouched files
- No other files modified

## Review Checklist for Root

1. Confirm `ExWorkflowDigest` bridge type is correct opaque specification pattern
2. Confirm no vacuum proof introduced (binding audit shows 0 vacuum)
3. Confirm no drift (drift gate shows 0)
4. Close bead `vb-pxby2` when satisfied