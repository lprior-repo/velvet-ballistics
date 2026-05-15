# Landing Report — vb-core-strict-ack-ordering

## Bead: vb-core-strict-ack-ordering
## Gate: State 14 (landing)
## Date: 2026-05-15

---

## Landing Summary

| Item | Detail |
|------|--------|
| Bead ID | vb-core-strict-ack-ordering |
| Title | runtime/storage: Prove strict persistence before acknowledgement ordering |
| Fix | `await_action` in `transitions.rs` — skip premature RetryCheck slot read |
| Files changed | `transitions.rs`, `action.rs`, `chunk_002.rs` |
| Tests | `action_completion_ack_test`: 4/4 PASS |
| Pre-existing failures | 5 (DEFERRED_GLOBAL) |
| Clippy | CLEAN |
| Gate status | APPROVED |

---

## What Changed

### transitions.rs — await_action

```diff
let capacity = if ticket.capacity > 0 {
+    ticket.capacity
} else {
    match crate::shard::helpers::retry_policy_after_action(&state, ticket.step) {
```

Added fast path: when `ticket.capacity > 0`, trust it and skip the slot read. This eliminates the `retry_policy_slot_unreadable` failure when RetryCheck hasn't executed yet.

### action.rs — execute_do

```diff
let input_taint = match run.read_taint(input) {
+    Ok(t) => t,
+    Err(CoreError::SlotUninitialized { .. }) => Taint::Clean,
+    Err(e) => return Err(RuntimeEngineError::Core(e)),
};
```

Added `SlotUninitialized => Taint::Clean` fallback (same as `execute_do_without_contract`). Previously, uninitialized input slots caused `execute_do` to fail before reaching the capability check.

### chunk_002.rs — apply_drive_result

```diff
Err(e) => {
+    if let ... CapabilityDenied { ... } = e {
+        // Insert as Resumable instead of terminal
+        state.frame.mark_running(step)?;
+        record_scheduled_attempt(&mut state, ticket);
+        self.runtime_states.insert(run, RuntimeState::Resumable);
+    } else {
        self.apply_terminal_failed(run, state)
+    }
}
```

`CapabilityDenied` is now treated as `Resumable`, not terminal. The run can be retried when capabilities are granted.

---

## Test Evidence

```
cargo test -p vb_runtime action_completion_ack_test
    Running 4 tests
    test handle_action_completion_persists_before_ack ... ok
    test action_failed_persists_before_ack ... ok
    test action_completion_error_blocks_ack ... ok
    test do_primitive_persists_all_required_events ... ok
4 passed, 0 failed
```

Full suite: vb_storage 924 passed / 1 failed (DEFERRED_GLOBAL) | vb_runtime 1376 passed / 4 failed (DEFERRED_GLOBAL) | Clippy 0 issues

---

## Bead Artifacts (State 13 Complete)

```
.beads/vb-core-strict-ack-ordering/
├── STATE.md                          # State 11 → 13
├── black-hat-review.md              # APPROVED
├── assurance-bundle.md               # COMPLETE
├── truth-serum-report.md             # CLEAN
├── final-evidence-decision.md        # APPROVED
├── formal-verification-report.md     # PASS_LOCAL
├── contract.md                       # COMPLETE
├── lean-contract.md                  # COMPLETE
├── implementation.md                 # COMPLETE
├── proof-obligations.jsonl           # 25 planned
├── traceability-matrix.jsonl         # COMPLETE
└── verification-ledger.jsonl         # COMPLETE
```

---

## Push to Remote

- Remote: `https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`
- Branch: `main`
- Dolt push: Required (beads data)
- Git push: Required (code + artifacts)

---

## Next Steps

1. Push beads data: `bd dolt push`
2. Push git: `git push origin main`
3. Advance to state 15 (cleanup)
