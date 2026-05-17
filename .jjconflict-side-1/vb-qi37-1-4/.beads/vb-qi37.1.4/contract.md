# Contract Specification — vb-qi37.1.4

## Context

- **Feature**: Runtime Recovery — Fail-Closed on Incomplete Recovery
- **Bead**: vb-qi37.1.4
- **State**: 3 (contract)
- **Source checkout**: /home/lewis/src/Velvet-ballistics
- **Risk**: critical (recovery safety)

### GAPS (from previous findings)

1. **GAP-1**: `slot_taint` alone does NOT trigger fail-closed (only when combined with `slot_values`)
2. **GAP-2**: `pending_actions.is_empty()` check means empty `pending_actions` bypasses the `unsupported.pending_actions` guard
3. **GAP-3**: Action ABI digest verification explicitly deferred in `verify_digests`

### Relevant Source Locations

- `crates/vb_runtime/src/recovery.rs::reject_unsupported_live_frame_state` (lines 73–82)
- `crates/vb_storage/src/recovery/recover.rs::verify_digests` (lines 54–74)
- `crates/vb_storage/src/recovery/types.rs::UnsupportedRecoveryState` (lines 222–231)

### Assumptions

- A run frame hydrated from storage must be fully verifiable or rejected.
- Any unsupported state flag set to `true` in `UnsupportedRecoveryState` must prevent runtime resumption.
- `verify_digests` at `DigestCheck::Full` must eventually verify action ABI digests and policy digests; the current implementation explicitly defers this with a comment.

### Open Questions

- O1: Does `unsupported.slot_taint` ever get set to `true` by storage replay? If not, GAP-1 may be untestable at integration level.
- O2: Should `pending_actions` guard trigger fail-closed when `unsupported.pending_actions` is `true` regardless of whether `pending_actions` is empty?

---

## Preconditions

- **PRE-001**: `RecoveryFrameSeed::unsupported` must accurately reflect which recovery state components are present in the durable journal.
- **PRE-002**: `verify_digests(DigestCheck::Full, ...)` must have access to the full set of scheduled action digests and policy digests from the journal.
- **PRE-003**: `reject_unsupported_live_frame_state` must receive a `RecoveryFrameSeed` whose `unsupported` flags truthfully represent durable record gaps.

---

## Postconditions

- **POST-001**: `reject_unsupported_live_frame_state` returns `Err(RuntimeError::InvalidRecoveryHydration)` when `unsupported.slot_taint` is `true`, regardless of `slot_values`.
- **POST-002**: `reject_unsupported_live_frame_state` returns `Err(RuntimeError::InvalidRecoveryHydration)` when `unsupported.pending_actions` is `true`, regardless of whether `pending_actions` is empty.
- **POST-003**: `verify_digests(DigestCheck::Full, ...)` returns `Ok(())` only when all of: workflow source digest, compiled IR digest, action ABI digests, and policy digests match their stored records.
- **POST-004**: When GAP-3 is formally waived, the waiver must state the owner, reason, expiry, limitation, and compensating evidence.

---

## Invariants

- **INV-GAP1-001**: `reject_unsupported_live_frame_state` MUST return `Err` when `unsupported.slot_taint` is `true`, independent of `slot_values`.
- **INV-GAP2-001**: `reject_unsupported_live_frame_state` MUST return `Err` when `unsupported.pending_actions` is `true`, independent of `pending_actions.is_empty()`.
- **INV-GAP3-001**: `verify_digests(DigestCheck::Full)` MUST verify action ABI digests and policy digests, or the contract must contain a formal waiver.

---

## Error Taxonomy

| Error Variant | Trigger | Fail-closed? |
|---|---|---|
| `RuntimeError::InvalidRecoveryHydration` | `slot_values`, `slot_taint`, or `pending_actions` unsupported flags set | Yes |
| `RecoveryError::ActionAbiMismatch` | Action ABI digest mismatch at `DigestCheck::Full` | Yes |
| `RecoveryError::PolicyDigestMismatch` | Policy digest mismatch at `DigestCheck::Full` | Yes |

---

## Contract Signatures

```rust
// crates/vb_runtime/src/recovery.rs

fn reject_unsupported_live_frame_state(seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    // GAP-1 fix: slot_taint alone triggers fail-closed
    // GAP-2 fix: pending_actions guard fires regardless of is_empty()
    if seed.unsupported.slot_values
        || seed.unsupported.slot_taint        // GAP-1: independent, not combined
        || seed.unsupported.pending_actions    // GAP-2: removed is_empty() guard
    {
        Err(RuntimeError::InvalidRecoveryHydration)
    } else {
        Ok(())
    }
}
```

```rust
// crates/vb_storage/src/recovery/recover.rs

pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
    // GAP-3: add action_abi_digests and policy_digests parameters
    action_abi_digests: &[(ActionId, WorkflowDigest)],
    policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<()> {
    // ... existing workflow/IR checks ...
    if matches!(level, DigestCheck::Full) {
        // GAP-3: verify action ABI digests
        for (action_id, expected_digest) in action_abi_digests {
            let stored_digest = lookup_action_abi_digest(journal, run, *action_id)?;
            if stored_digest != *expected_digest {
                return Err(RecoveryError::ActionAbiMismatch { action_id: *action_id });
            }
        }
        // GAP-3: verify policy digests
        for (step, expected_digest) in policy_digests {
            let stored_digest = lookup_policy_digest(journal, run, *step)?;
            if stored_digest != *expected_digest {
                return Err(RecoveryError::PolicyDigestMismatch { step: *step });
            }
        }
    }
    Ok(())
}
```

---

## Verus-Owned Clauses

- INV-GAP1-001: `reject_unsupported_live_frame_state` returns `Err` when `slot_taint` is `true` — Verus pure boolean spec
- INV-GAP2-001: `reject_unsupported_live_frame_state` returns `Err` when `pending_actions` unsupported, regardless of `is_empty()` — Verus pure boolean spec
- INV-GAP3-001: `verify_digests(DigestCheck::Full)` verifies both action ABI and policy digests — Verus pure spec (or waiver)

## TLA+-Owned Clauses

- None: The three GAPS are Rust-local pure boolean conditions in `reject_unsupported_live_frame_state` and deterministic digest comparison in `verify_digests`. No temporal/state-over-time behavior. TLA+ not applicable.

## Theorem-Owned Clauses

- None. All critical clauses are expressible in Verus.

## Non-goals

- Proving Fjall journal durability properties
- Proving snapshot encoding/decoding correctness (covered by Kani codec harness)
- Proving action retry backoff policy
