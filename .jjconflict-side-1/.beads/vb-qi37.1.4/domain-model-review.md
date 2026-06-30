# Domain Model Review — vb-qi37.1.4

## Boundary

- **Recovery module** (`vb_storage::recovery`): Storage-side replay, summary, frame seed construction, unsupported state flag computation, digest verification orchestration.
- **Runtime boundary** (`vb_runtime::recovery`): `RuntimeRecoveryBoundary` trait; `DurableFrameRecoveryBoundary`; `SummaryRecoveryBoundary`. The runtime boundary is the **fail-closed gate** for all incomplete/invalid recovery state.

## Key Domain Entities

### UnsupportedRecoveryState
Flags which parts of a `RecoveryFrameSeed` could NOT be reconstructed from durable journal events. Four boolean flags:

| Flag | Meaning | Currently Checked in Runtime Boundary? |
|---|---|---|
| `slot_values` | Slot value bodies missing/corrupt | ✅ Yes (INV-RC-001) |
| `slot_taint` | Slot taint markers missing | ✅ Yes (INV-RC-002) |
| `action_payloads` | Action result bodies missing | ❌ **No — gap** |
| `pending_actions` | Pending action resumability unknown | ✅ Yes (INV-RC-004) |

**Gap**: `action_payloads` is defined, included in `SUPPORTED` constant, covered by `union()`, but never tested in `reject_unsupported_live_frame_state()`. This means a seed with `action_payloads: true` passes the runtime boundary, but the runtime cannot safely consume action results — violating INV-RC-005.

### DigestCheck
Three-level enum controlling which digests are verified during recovery:

```rust
pub enum DigestCheck {
    WorkflowSourceOnly,  // Only RunAccepted workflow digest
    WorkflowAndIr,      // + compiled IR digest
    Full,               // + action ABI + policy (DEFERRED — no-op currently)
}
```

**Gap**: `DigestCheck::Full` is documented as verifying action ABI and policy digests, but the implementation in `verify_digests` (recover.rs line 71-72) contains only a comment: "Action ABI digest verification is deferred to a future phase." The function returns `Ok(())` without checking anything beyond IR digest.

### JournalEvent Lifecycle Variants
Three variants that are **silently absorbed** in `replay_events`:

- `RunResumed` — run resumed after suspend
- `RunRetried` — run retry scheduled
- `RunAnswered` — run answer delivered (selected a result slot)

These are matched exhaustively in `replay_events` (core.rs lines 61-63) but do nothing. They are included in `replayed.push(event.clone())` so they appear in the output, but no state machine transition occurs.

**Analysis**: This is intentional design — these events carry no state that affects the `RunFrame` reconstruction. However:
1. `RunAnswered` is a terminal-adjacent event (selects result slot) but `is_terminal_event` does NOT include it.
2. If a future `JournalEvent` variant is added and NOT added to the `replay_events` match, compilation will fail (exhaustive match). But if a new variant IS added to `replay_events` with a no-op branch, the runtime boundary trust in `UnsupportedRecoveryState` may not extend to cover it.

## Fail-Closed Boundary Semantics

The runtime boundary is the **last line of defense** before a potentially inconsistent `RunFrame` is used to resume a run. The boundary contract:

```
hydrate_run_frame() -> Ok(RunFrame)
  iff
  unsupported == { slot_values: false, slot_taint: false, action_payloads: false,
                   pending_actions: false }
  OR pending_actions.is_empty() when pending_actions flag is true
```

**Current implementation is missing `action_payloads`** — it only checks `slot_values`, `slot_taint`, and the conditional `pending_actions`.

## Invariant Violation Scenarios

### Scenario 1: Corrupt Action Payload During Recovery
1. Storage detects that some `ActionCompletedEvent` records have missing payload bodies.
2. `UnsupportedRecoveryState::action_payloads` is set to `true`.
3. `reject_unsupported_live_frame_state` does NOT check this flag.
4. `hydrate_run_frame` returns `Ok(frame)` with a frame that cannot safely execute action resolution.
5. Runtime resumes execution believing actions are complete when their results are unknown.

**Severity**: Critical. Violates INV-RC-003, INV-RC-005.

### Scenario 2: DigestCheck::Full Silently Passes
1. Caller requests `DigestCheck::Full` to verify action ABI digests.
2. `verify_digests` checks workflow source ✅ and compiled IR ✅.
3. Action ABI check is deferred; policy digest check is deferred.
4. `Ok(())` returned even though action digests are mismatched.
5. Runtime resumes with actions whose ABI may not match the current artifact.

**Severity**: Critical. Violates INV-RC-006, INV-RC-008.

## Review Findings

| Finding | Severity | Type |
|---|---|---|
| `action_payloads` not checked in runtime boundary | Critical | Missing invariant enforcement |
| `DigestCheck::Full` action/policy digests deferred | Critical | Missing implementation |
| `RunResumed/RunRetried/RunAnswered` silently absorbed | Medium | Intentional but undocumented; `RunAnswered` not in `is_terminal_event` |
| `action_payloads` flag may be dead code | Unknown | Needs confirmation from storage replay team |
| No Verus proof of fail-closed boundary | High | Required verifier mode |
