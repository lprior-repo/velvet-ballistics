# vb-qi37.12.2 Test Plan

## Bead
- **ID**: vb-qi37.12.2
- **Title**: [BUG] runtime/storage: Propagate journal and storage failures
- **Phase**: 8 (Write failing-first tests)

## Bug Description

### Bug 1: `observe_resume_drive_result` silently drops errors
**Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:178-182`

```rust
fn observe_resume_drive_result(result: RuntimeResult<()>) {
    match result {
        Ok(()) | Err(_) => {}
    }
}
```

The function is called from `handle_resume` at line 138:
```rust
let drive_result = self.drive_run(run);
Self::observe_resume_drive_result(drive_result);
```

**Problem**: Both `Ok(())` and `Err(_)` match to empty arms `{}`. Any error from `drive_run` is silently discarded. `handle_resume` then returns `Ok(ResumeResult { status: ResumeStatus::Resumed, ... })` even when `drive_run` failed.

**Impact**: Caller of `handle_resume` cannot distinguish a successful resume from a failed one.

**Fix Required**: Either:
1. `observe_resume_drive_result` should log/record/return the error, OR
2. The call site in `handle_resume` should check `drive_result` and propagate errors

### Bug 2: `handle_submit` journal ordering — events before state insert
**Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:30-77`

**Sequence in `handle_submit_with_inputs_and_header_mode`**:
1. `trace_ring.push(TraceEvent::RunSubmitted)` — trace only
2. `journal.append(RuntimeJournalEvent::RunSubmitted)` — **journal write**
3. `journal.append(RuntimeJournalEvent::RunAdmission)` — **journal write**
4. `self.runs.insert(run, state)` — **state insert**
5. `self.drive_run(run)`

**Problem**: If process crashes between steps 2-3 and step 4, journal records the run as submitted but no `RunState` exists. The durability contract requires: journal record must not exist without corresponding state.

**Fix Required**: Ensure atomicity of journal append + state insert (same durability domain).

### Bug 3: Error propagation paths
Multiple `?` operators in `handle_submit` and `handle_resume` that correctly convert `RuntimeError` to appropriate error types. The issue is the silent drop in `observe_resume_drive_result`.

## Test Coverage

### Test File
`crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs`

### Tests Written

| Test | Bug | Description | Pass Condition |
|------|-----|-------------|---------------|
| `handle_resume_returns_error_when_drive_run_fails` | Bug 1 | `handle_resume` must return error when `drive_run` fails | `result.is_err()` |
| `observe_resume_drive_result_does_not_drop_drive_run_error` | Bug 1 | Error from `drive_run` must propagate via `handle_resume` | `result.is_err()` |
| `handle_submit_journal_before_state_insert_noorphan_journal_record` | Bug 2 | After submit, both journal events AND run state exist | `active_run_count() == 1` |
| `handle_submit_propagates_journal_failure_before_drive_run` | Bug 3 | Journal failure propagates through `tick()` | `tick_result.is_err()` |
| `handle_submit_journal_event_ordering_run_submitted_before_admission` | Bug 2 | `RunSubmitted` appears before `RunAdmission` in journal | `submitted_pos < admission_pos` |
| `handle_resume_propagates_flush_evidence_failure` | Bug 1 | `drive_run` failure via journal flush propagates | `!run_exists` (error causes submit to fail) |

### FailingJournal Test Infrastructure
A `FailingRuntimeJournal` wrapper that injects `StorageJournalAppend` errors after N appends:
- `fail_after=0`: First append fails
- `fail_after=2`: Third append fails (RunAdmission + flush_evidence during submit)
- `fail_after=4`: Fifth append fails (resume's second flush_evidence)

## Known Limitations

### Bug 1 (`observe_resume_drive_result`) Testing Difficulty
In single-threaded tests, `drive_run` always returns `Ok(())` because:
- `apply_drive_result` handles errors internally via `apply_terminal_failed`
- `apply_terminal_failed` returns `Ok(())` unless `finish_run` fails
- `finish_run` appends `RunFailed` to the journal — if the journal is failing, `finish_run` fails and the error propagates

The tests for Bug 1 verify that journal failures propagate (via `finish_run`), but cannot directly verify that `observe_resume_drive_result` drops errors because the journal failure path is the same error path.

Internal unit tests (within `#[cfg(test)]` module in `lifecycle.rs`) would be needed to directly test `observe_resume_drive_result` behavior by calling `handle_resume` with controlled internal state.

## Test Execution

```bash
cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation
```

All 6 tests should pass after the bug fix is applied.