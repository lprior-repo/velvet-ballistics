# vb-hjdi0 Handoff: Public replay_journal idempotency and real ABI

## Summary
Fixed two production bugs exposed by ko29 replay probe:
1. Recovery rejected valid `ActionScheduledTicket` events (only checked legacy `ActionScheduled`)
2. Action completion wrote all-zero `action_abi_digest` instead of actual contract ABI

## Product Changes

### Bug 1: Recovery accepts ActionScheduledTicket
- **File**: `crates/vb_storage/src/recovery/replay/core/full.rs`
- **Change**: Line 45 now checks for both `JournalEvent::ActionScheduled` and `JournalEvent::ActionScheduledTicket` as valid schedule events
- **Invariant**: When `expected_action_abi_digests` is nonempty, journal must contain at least one schedule event of either variant

### Bug 2: Real ABI digest in completion envelope
- **File**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
- **Change**: `apply_action_completion` now computes actual `action_abi_digest` using `schedule_action_abi_digest(&state, ticket.action)?` before appending `ActionCompletedEnvelope`
- **Invariant**: Digest in completion envelope matches digest computed at schedule time

## Test Changes

### Restored Public API Tests
Both tests now use the public `vb_storage::replay_journal` API with real ABI from producer schedule:

1. **given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable**
   - Old behavior: Checked journal events directly without calling `replay_journal`
   - New behavior: Calls `vb_storage::replay_journal` with expected ABI, verifies replayed event count matches, no duplicate scheduling, journal unchanged

2. **given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action**
   - Old behavior: Checked journal events directly without calling `replay_journal`
   - New behavior: Calls `vb_storage::replay_journal` for evicted run, verifies replay resolves the action from durable journal

### Added Negative Regression Test
**given_no_schedule_event_when_expected_abi_then_replay_journal_rejects_with_abi_mismatch**
- Verifies `replay_journal` rejects with `RecoveryError::ActionAbiMismatch` when expected ABI digests are provided but no schedule event exists
- Confirms exact error variant and action_id are reported

### Removed Workarounds
- Removed all manual tracker folds and private API shortcuts
- Removed unused `digest_hex` helper function
- Removed duplicate old test function body

## Evidence

| Test Suite | Result | Evidence |
|---|---|---|
| vb_ko29_5_public_idempotency (17 tests) | PASS | `.beads/vb-hjdi0/evidence/ko29-test-4.*` |
| vb_storage recovery tests | PASS | `.beads/vb-hjdi0/evidence/storage-recovery-2.*` |
| vb_runtime action completion tests | PASS | `.beads/vb-hjdi0/evidence/runtime-action-completion.*` |
| clippy vb_storage --lib | PASS | `.beads/vb-hjdi0/evidence/clippy-vb-storage.*` |
| clippy vb_runtime --lib | PASS | `.beads/vb-hjdi0/evidence/clippy-vb-runtime.*` |
| source-length check | FAIL (pre-existing) | 2 verus files over 800 lines, unrelated to changes |
| panic-surface check | PASS | No violations |

## Verification Notes
- All 17 tests in vb_ko29_5_public_idempotency pass (16 original + 1 new negative regression)
- Both restored tests use real producer fixture with actual ABI from `produce_completed_run_with_abi`
- No fabricated tickets or manual tracker folds
- Source length failures are in `verification/verus/` directory, pre-existing, unrelated to this fix
