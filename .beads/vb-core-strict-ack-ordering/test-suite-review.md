# test-suite-review.md — vb-core-strict-ack-ordering

## VERDICT: REJECTED

---

### Tier 0 — Static
[PASS] Banned pattern scan — no `assert!(result.is_ok())` / `assert!(result.is_err())` as hollow assertions in ack-specific tests
[PASS] Determinism/evidence scan — no static mutable, lazy_static with Mutex, or shared global state
[PASS] Mock interrogation — no mockall found in ack test files
[PASS] Integration test purity — no `use crate::` in integration test files
[PASS] Error variant completeness — RuntimeError, JournalError, RecoveryError variants exist
[PASS] Density audit — submit_direct_durability_test.rs has 23 tests covering matrix validation

**Tier 0 PASSED** — static analysis reveals no banned patterns in the ack-specific test files.

---

### Tier 1 — Execution
[FAIL] action_completion_ack_test: 2 passed, 2 failed
  - handle_action_completion_persists_before_ack: `RunNotFound` at line 120
  - action_failed_persists_before_ack: `RunNotFound` at line 197

[FAIL] ask_completion_ack_test: 2 passed, 2 failed
  - handle_ask_completion_persists_before_ack: `ask_workflow()` returns `None` at line 119
  - ask_completion_error_preserves_fail_closed_contract: `ask_workflow()` returns `None` at line 234

[FAIL] recovery_digest_match_test (vb_storage): 11 passed, 1 failed
  - replay_events_blocks_non_idempotent_action: returns `Ok(...)` instead of `Err(RecoveryError::NonIdempotentActionBlocked)` at line 188

[PASS] submit_direct_durability_test (vb_runtime): 23 passed, 0 failed
[PASS] vb_storage --lib: 924 passed

[INFO] vb_runtime --lib: 1266 passed, 85 failed — pre-existing failures in chunk_024.rs/chunk_026.rs (bead vb1u88), not related to vb-core-strict-ack-ordering

**Tier 1 FAILED** — REJECTED at Tier 1.

---

### Tier 2 — Coverage
[SKIPPED] Line/branch coverage not run — Tier 1 failures block progression.

---

### Tier 3 — Mutation
[SKIPPED] Mutation testing not run — Tier 1 failures block progression.

---

## LETHAL FINDINGS

### 1. action_completion_ack_test.rs:120 — RunNotFound on tick()
**File:** `crates/vb_runtime/tests/action_completion_ack_test.rs:120`
**Problem:** `shard.tick().unwrap()` returns `Err(RunNotFound)` after enqueueing `ActionCompleted`.
**Root cause:** The test enqueues `ActionCompleted` without first transitioning the run to a state where it expects action completion. The workflow is submitted and ticked once, but the run is not in an active action-waiting state when `ActionCompleted` is enqueued.
**Required fix:** The test must first tick the shard until the action is actually scheduled (StepStarted + ActionScheduled journal events must appear), THEN enqueue ActionCompleted.

### 2. action_completion_ack_test.rs:197 — RunNotFound on tick()
**File:** `crates/vb_runtime/tests/action_completion_ack_test.rs:197`
**Problem:** Same as finding #1 — `shard.tick().unwrap()` returns `Err(RunNotFound)`.
**Required fix:** Same as finding #1.

### 3. ask_completion_ack_test.rs:119 — ask_workflow() returns None
**File:** `crates/vb_runtime/tests/ask_completion_ack_test.rs:119`
**Problem:** `ask_workflow()` returns `None` because `CompiledWorkflow::try_from_parts(parts)` fails. The workflow fixture is structurally invalid — `AskResume` step 3 references answer slot 2, but the workflow has only 3 slots (0, 1, 2).
**Required fix:** Fix the `ask_workflow()` fixture to have `slot_count: 3` but the `AskResume` should reference a valid slot that was written by the ask step.

### 4. ask_completion_ack_test.rs:234 — ask_workflow() returns None
**File:** `crates/vb_runtime/tests/ask_completion_ack_test.rs:234`
**Problem:** Same as finding #3 — `ask_workflow()` returns `None`.
**Required fix:** Same as finding #3.

### 5. recovery_digest_match_test.rs:188 — NonIdempotentActionBlocked not returned
**File:** `crates/vb_storage/tests/recovery_digest_match_test.rs:188`
**Problem:** `replay_events` returns `Ok(...)` when given duplicate ActionCompleted events, but the test expects `Err(RecoveryError::NonIdempotentActionBlocked { .. })`.
**Root cause:** Either (a) `ActionReplayTracker` does not detect duplicate completions, or (b) `replay_events` does not call `tracker.is_resolved()` before appending, or (c) the contract RECOVERY-003 does not actually require this behavior.
**Required fix:** Either implement the missing detection logic in `replay_events`, OR acknowledge this is a planned feature not yet implemented and mark the test as `#[ignore]` with a comment explaining the gap.

---

## MAJOR FINDINGS (0)

No additional major findings beyond the 5 lethal ones above.

---

## MINOR FINDINGS (0)

No minor findings.

---

## MANDATE

The following must be resolved before resubmission:

1. **FIX: action_completion_ack_test.rs** — The test `handle_action_completion_persists_before_ack` must properly transition the run to an action-waiting state before enqueueing `ActionCompleted`. This requires multiple `tick()` calls or a different setup that causes the shard to emit `ActionScheduled` before the completion is enqueued.

2. **FIX: ask_completion_ack_test.rs** — The `ask_workflow()` fixture must be corrected so `CompiledWorkflow::try_from_parts` succeeds. The slot_count and slot references must be consistent with the node definitions.

3. **FIX: recovery_digest_match_test.rs:188** — Either implement non-idempotent action detection in `replay_events` and `ActionReplayTracker`, or document the gap with `#[ignore]` and a comment referencing the unimplemented RECOVERY-003 obligation.

4. **RE-RUN** all Tier 0–3 gates after fixes are applied.

---

## EVIDENCE CITATIONS

- Action completion test failure: `~/.local/share/rtk/tee/1778832372_cargo_test.log`
- Ask completion test failure: `~/.local/share/rtk/tee/1778832394_cargo_test.log`
- Recovery test failure: `~/.local/share/rtk/tee/1778832414_cargo_test.log`
- vb_storage test pass: `cargo test -p vb_storage --lib` → 924 passed
- submit_direct_durability_test pass: 23 passed, 0 failed
