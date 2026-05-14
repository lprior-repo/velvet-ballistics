bead_id: vb-qi37.16.2
phase: state-6
classification: BLOCK_LOCAL
owner_state: 6
rerun_from: 6

# State 6 Block Evidence

State 5 was repaired: `rtk cargo test --package vb_runtime --test durable_resume_red_phase` compiles and proves RED with 9 pass / 8 fail.

State 6 holzman-rust implemented fixes:
1. `fail_run_state` now tracks `Failed` in `runtime_states` (transitions.rs:97)
2. `handle_resume` re-inserts `Running` state after successful `drive_run` (lifecycle.rs:251-260)

Verification command:

```bash
rtk cargo test --package vb_runtime --test durable_resume_red_phase
```

**Result: 14 passed; 3 failed** — improved from 14/3 but 3 remain BLOCK_LOCAL.

## Root Cause Analysis

### 1. `resume_pre002_from_failed_fails_not_resumable` (line 133)

**Symptom**: Test expects `NotResumable` but gets `RunIdNotFound`.

**Root Cause**: The test uses `suspended_workflow` which suspends on action (state = `Resumable`), NOT `Failed`. The test comment says "Simulate failure by removing from active runs (this is a placeholder)". After submit + tick, the workflow is `Resumable`, not `Failed`. The test expectation is incorrect for a suspended workflow.

**Test Comment**: "this is a placeholder - in real implementation, Failed state would be tracked"

**Analysis**: `fail_run_state` now tracks `Failed`, but the workflow never enters `Failed` state. A suspended workflow goes `Initial -> Running -> Resumable` (via AwaitingAction), never `Failed`.

---

### 2. `resume_pre002_from_resuming_fails_not_resumable` (line 168)

**Symptom**: Second resume returns success (`AlreadyRunning`) instead of `NotResumable`.

**Root Cause**: After first resume, state is correctly set to `Running` (by the fix at lifecycle.rs:259). Second resume sees `Running` and returns `AlreadyRunning` (per lifecycle.rs:214-220).

**Conflict**: There is a PASSING test `resume_pre002_from_running_returns_already_running` which validates that `Running` state returns `AlreadyRunning` (success). The failing test expects `NotResumable` (error), which contradicts the passing test.

**Contract Analysis**: PRE-002 says "not Initial, not Running, not Failed" are not Resumable. The implementation has a special case: `Running` returns `AlreadyRunning` (success), not `NotResumable` (error). This is validated by the passing test.

**Conclusion**: The failing test has incorrect expectation. The implementation behavior is validated by the passing test `resume_pre002_from_running_returns_already_running`.

---

### 3. `resume_post001_journal_appended_before_success` (line 281)

**Symptom**: Last journal event is not `Resumed`.

**Root Cause**: After `handle_resume` appends `Resumed` and calls `drive_run`, if `drive_state` returns `Finished` (action completed immediately), `apply_drive_result` calls `finish_run` which appends `RunFinished` to the journal AFTER `Resumed`.

**Analysis**: The journal sequence becomes `[RunSubmitted, RunAdmission, Resumed, RunFinished]` instead of `[RunSubmitted, RunAdmission, Resumed]`. The last event is `RunFinished`, not `Resumed`.

**Suspected Issue**: The `suspended_workflow` (single Do action) might be completing immediately (returning `Finished`) instead of suspending (returning `AwaitingAction`). This could be because the action handler completes synchronously, or `drive_state` is not properly returning `AwaitingAction` for suspended workflows.

---

## Contract/Tests Contradiction

The test suite has internal contradictions:
- `resume_pre002_from_running_returns_already_running` (PASS): Running -> AlreadyRunning (success)
- `resume_pre002_from_resuming_fails_not_resumable` (FAIL): expects second resume to fail with NotResumable, but after first resume state is Running, which returns AlreadyRunning (contradiction)

The passing test validates the implementation behavior is correct for `Running` state.

## Changes Made

### File: crates/vb_runtime/src/shard/transitions.rs
```rust
// Added: Track Failed state in runtime_states
self.runtime_states.insert(run, crate::shard::types::RuntimeState::Failed);
```

### File: crates/vb_runtime/src/shard/lifecycle.rs
```rust
// Added: Re-insert Running after successful drive_run
let drive_result = self.drive_run(run);
if drive_result.is_ok() {
    self.runtime_states.insert(run, RuntimeState::Running);
}
```

## Conclusion

Tests 1 and 2 have incorrect expectations (test bugs, not implementation bugs).
Test 3 may have a real issue with `drive_state` returning `Finished` instead of `AwaitingAction` for suspended workflows.

Continue State 6; tests 1 and 2 are test bugs, test 3 requires deeper investigation of `drive_state` behavior.
