# vb-hjdi0: Public replay_journal idempotency and real ABI

## Problem
ko29 replay probe exposed two production bugs:
1. `recover_full_journal` rejects valid journals containing `ActionScheduledTicket` (modern) because it only checks for legacy `ActionScheduled`.
2. `apply_action_completion` writes all-zero `action_abi_digest` in `ActionCompletedEnvelope` instead of the actual scheduled action's contract ABI digest.

## Fix

### Bug 1: Recovery rejects ActionScheduledTicket
- File: `crates/vb_storage/src/recovery/replay/core/full.rs`
- Change: Line 45 must accept both `JournalEvent::ActionScheduled` (legacy) and `JournalEvent::ActionScheduledTicket` (modern) as valid schedule events.
- Invariant: When `expected_action_abi_digests` is nonempty, the journal must contain at least one schedule event of either variant. Missing both → `ActionAbiMismatch`.
- Blast radius: `recover_full_journal` only; `replay_events_with_schedule_requirement` unchanged.

### Bug 2: Zero ABI digest in completion envelope
- File: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
- Change: Compute actual `action_abi_digest` using `schedule_action_abi_digest(&state, ticket.action)?` before appending `ActionCompletedEnvelope`.
- Invariant: The digest in the completion envelope must match the digest computed at schedule time from the action's contract.
- Blast radius: `apply_action_completion` only; schedule path unchanged.

## Tests
- Restore 2 public API tests in `vb_ko29_5_public_idempotency.rs` using `vb_storage::replay_journal` with real ABI from producer schedule.
- Remove all manual tracker folds and private API workarounds.
- Add focused negative regression tests: missing schedule, wrong ABI, zero ABI all rejected with exact `RecoveryError`.
- All 16 named tests in the suite must pass.
