# Black-Hat Review — vb-av1y0 (P0-5b2)

## Review Status: **APPROVED**

### Reviewer
- Black-hat reviewer (manual, no delegation)
- Date: 2026-06-13

## Scope Verification

**Bead requirement:** Add `pub fn pending_actions_from_events(events: &[JournalEvent]) -> Vec<RecoveredPendingAction>` in `vb_storage::recovery::replay::summary` that delegates to the existing private `recovered_pending_actions`.

**Actual changes (working tree):**
1. `summary.rs`: Added `pub fn pending_actions_from_events()` (line 862) + private inner `fn recover_pending_actions_from_events_inner()` (line 817)
2. `types.rs`: Added `Hash` derive to `RecoveredPendingAction` (line 291)
3. `summary/tests.rs`: Added 10 unit tests

**Scope creep check:** 
- ✅ No behavioral changes to `seed_unsupported_state()` — original "pending_actions are always supported" preserved
- ✅ No `envelope_event_seen` field — removed
- ✅ No `action_payloads_unsupported()` function — removed
- ✅ No changes to `recovery/tests.rs` — clean
- ✅ Only the bead-requested function was added

## Contract Parity

| Requirement | Spec | Implementation | Status |
|---|---|---|---|
| Function name | `pending_actions_from_events` | `pub fn pending_actions_from_events` | ✅ Exact match |
| Signature | `&[JournalEvent]) -> Vec<RecoveredPendingAction>` | `pub fn pending_actions_from_events(events: &[JournalEvent]) -> Vec<RecoveredPendingAction>` | ✅ Exact match |
| Delegation | Delegates to `recovered_pending_actions` | Calls `recover_pending_actions_from_events_inner()` → `recovered_pending_actions()` | ✅ |
| No new trait | No trait methods | Free function only | ✅ |
| Preserve private fn | `recovered_pending_actions` unchanged | Unchanged | ✅ |
| No `ActionTicket` return | `Vec<RecoveredPendingAction>` | `Vec<RecoveredPendingAction>` | ✅ |
| No feature gate | None | None | ✅ |

## Anti-Hallucination Verification

1. **Function exists:** `rg "pub fn pending_actions_from_events" crates/vb_storage/src/recovery/replay/summary.rs` → 1 match at line 862 ✅
2. **Private fn still exists:** `rg "fn recovered_pending_actions" crates/vb_storage/src/recovery/replay/summary.rs` → 1 match ✅
3. **No unwrap/expect/panic/unsafe in new code:** Manual inspection of lines 817-884 → zero forbidden constructs ✅
4. **Inner function signature:** `fn recover_pending_actions_from_events_inner(events: &[JournalEvent]) -> HashSet<(ActionId, StepIdx)>` ✅
5. **Delegation chain:** `pending_actions_from_events → recover_pending_actions_from_events_inner → recovered_pending_actions → Vec<RecoveredPendingAction>` ✅

## Test Coverage

| Test | Scenario | Result |
|---|---|---|
| `pending_actions_from_events_returns_collected_actions` | 5 scheduled, 3 completed → 2 pending | ✅ PASS |
| `pending_actions_from_events_empty_input` | Empty slice → empty vec | ✅ PASS |
| `pending_actions_from_events_only_terminal_events` | RunFinished/Failed/Cancelled → empty | ✅ PASS |
| `pending_actions_from_events_orphan_completed_event` | Completed without scheduled → empty | ✅ PASS |
| `pending_actions_from_events_all_scheduled_no_completed` | 3 scheduled, 0 completed → 3 pending | ✅ PASS |
| `pending_actions_from_events_all_completed_no_pending` | 1 scheduled, 1 completed → empty | ✅ PASS |
| `pending_actions_from_events_empty_slice_precondition` | Empty slice → empty (duplicate of B2) | ✅ PASS |
| `pending_actions_from_events_length_equals_scheduled_minus_completed` | 2 scheduled, 1 completed → 1 pending | ✅ PASS |
| `pending_actions_from_events_is_pure_deterministic` | Same input → same output | ✅ PASS |
| `pending_actions_from_events_handles_ticket_variants` | Ticket envelopes work correctly | ✅ PASS |

## Regression Check

- All 1327 existing tests pass (no regressions)
- `unresolved_action_recovers_as_pending_action_supported` test restored to original name and assertion
- `seed_unsupported_state()` behavior unchanged from pre-bead state

## Defects Found

**None.** The implementation is minimal, correct, and in scope.

## Minor Notes

1. Test B7 (`pending_actions_from_events_empty_slice_precondition`) duplicates B2 (`pending_actions_from_events_empty_input`). Not a defect — both assert the same precondition from slightly different angles.
2. `Hash` derive on `RecoveredPendingAction` is needed for test assertions using `HashSet`. Both fields (`StepIdx`, `ActionId`) already implement `Hash`, so this is a zero-cost addition.

## Verdict

**STATUS: APPROVED** — The implementation satisfies the bead spec exactly. No scope creep, no behavioral changes, no forbidden constructs, all tests pass, no regressions.
