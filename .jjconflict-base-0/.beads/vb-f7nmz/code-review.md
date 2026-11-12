# Production Code Review: vb-f7nmz

**Under review:** vb-f7nmz "Independently review restored public admission retry replay and journal coverage"
**Author:** LOCAL3090
**Reviewer:** LOCAL5090 (independent)
**Status:** APPROVED

## Production Changes Verified

### 1. `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` — `apply_action_completion`

**Change:** Completion envelope now computes actual schedule ABI digest via `schedule_action_abi_digest(state, ticket.action)?` instead of writing zero.

**Contract identity verified:**
- Same helper `schedule_action_abi_digest` used by both schedule and completion paths (defined in `chunk_005_prepare.rs:3`)
- ABI digest computed from authoritative action contract: `state.action_contracts` → `action_contract_abi_digest(contract)`
- Zero/default digest eliminated; actual contract identity carried through durable completion envelope
- No API widening: `action_abi_digest` field exists on `ActionCompletedEnvelope` (record kind 35)
- Validation occurs before durable append: preflight + ABI computation at lines 569-574, then `append_journal_event` at 575
- No wrong-run/step acceptance: `ticket.action` and `ticket.step` are validated by `require_pending_action_ownership` (line 568) and `preflight_action_completion` (line 571)
- Callback ownership unchanged: drive_run still uses `preflight.outcome`

**Check:** Mutation "write zero instead of real ABI" caught by `given_real_producer_zero_expected_abi_when_replayed_then_rejected_with_abi_mismatch` — test asserts real producer completion envelope contains nonzero ABI that matches schedule event.

### 2. `crates/vb_storage/src/recovery/replay/core/full.rs` — `recover_full_journal`

**Change:** Preflight schedule presence accepts modern `ActionScheduledTicket` (record kind 34) in addition to legacy `JournalEvent::ActionScheduled`.

**Contract identity verified:**
- Missing both `ActionScheduled` AND `ActionScheduledTicket` → `ActionAbiMismatch` for first expected action (lines 42-56)
- Per-event ABI validation unchanged: `replay_events` still validates each completion against expected digests
- Per-event identity validation unchanged: `validate_ticket_attempt` still validates run/step/generation/attempt
- Public replay API `replay_journal` accepts `expected_action_abi_digests: &[(ActionId, WorkflowDigest)]` — actual run/step resolution happens internally via `ActionReplayTracker`
- Malformed completion still typed-rejected: `replay_events` validates ABI at each completion event

**Check:** Mutation "ignore ActionScheduledTicket" caught by `given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable` and `given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action` — both use real producer which emits `ActionScheduledTicket`. If ticket acceptance were removed, these real producer tests would fail.

### 3. `crates/vb_runtime/src/shard/lifecycle/chunk_005_drive_apply.rs` — `apply_action_failure`

**Note:** Old zero ABI write in `ActionFailedEnvelope` remains orphaned — not resurrected by vb-hjdi0. This is correct: completion ABI is the contract boundary; failure path does not require ABI for replay resolution (failure outcome is independent of ABI digest).

## Proof Registry/Binding Impact

- Verus production binding: 0 STRONG, 72 WEAK, 0 VACUUM — unchanged from baseline. No new vacuum models introduced.
- Production-inner drift: 0 findings — no source-to-model drift introduced.
- No false whole-production proof claims: existing binding/drift status preserved.

## Verification Evidence

| Check | Result | Evidence |
|-------|--------|----------|
| 19 named tests pass | OK | `review-v1-cargo-test.*` |
| vb_storage recovery::replay | OK | `review-v2-vb-storage-recovery.*` |
| vb_runtime shard::lifecycle | OK | `review-v3-vb-runtime-shard.*` |
| Clippy -D warnings (vb_runtime, vb_storage) | OK | `review-v4-clippy.*` |
| Verus production binding | OK | 0 VACUUM |
| Production-inner drift | OK | 0 findings |

## Mutation Resistance Assessment

| Hypothetical Regression | Caught By |
|-------------------------|-----------|
| Revert to zero ABI in completion | `given_real_producer_zero_expected_abi_when_replayed_then_rejected_with_abi_mismatch` |
| Revert to ignoring ActionScheduledTicket | `given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable` |
| Accept wrong expected ABI | `given_real_producer_wrong_expected_abi_when_replayed_then_rejected_with_abi_mismatch` |
| Accept missing schedule event | `given_no_schedule_event_when_expected_abi_then_replay_journal_rejects_with_abi_mismatch` |
| Journal mutated by rejected replay | Multiple tests assert `after == before` |

**STATUS: APPROVED** — Production changes implement exact contract: real ABI in completion envelope + modern schedule ticket acceptance in recovery preflight. No API widening, no weakening, no false proof claims.