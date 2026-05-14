bead_id: vb-qi37.16.4
phase: state-6
classification: BLOCK_LOCAL
owner_state: 6
rerun_from: 6

# Block Evidence

## Compile Errors Fixed

Previous compile errors (duplicate/missing `encoded_len` fields in `ActionOutputReady`, `ActionFailure`, and `AskAnswer` initializers) have been resolved.

## Verification Command

```bash
rtk cargo test -p vb_runtime --lib -- "shard::lifecycle::tests::red_"
```

## Current Result

```text
test result: FAILED. 10 passed; 2 failed; 0 ignored; 0 measured; 1337 filtered out
```

## Improved from Previous State

Previous state: 8 passed; 4 failed
Current state: 10 passed; 2 failed

The following now PASS (fixed via compile error resolution):
- `red_ask_answer_diagnostics_safe`: secret taint contract check now works correctly
- `red_test_taint_secret_rejected_without_permission`: secret taint rejection now works correctly  
- `red_test_payload_size_one_byte_over`: payload boundary check now works correctly

## Remaining Bead-Local Failures

### 1. `red_ask_answer_durable` - BLOCK_LOCAL

**Error:**
```
assertion `left == right` failed: After journal replay, run should complete (POST-004 durability)
  left: 0
 right: 1
```

**Root Cause Analysis:**
`VolatileRuntimeJournal` does not implement `events_for_run()` method. It only stores events via `append()` but has no replay mechanism. When a new shard is created with the same journal Arc and `tick()` is called, there are no commands in the queue - the journal events are stored but not replayed.

The `RuntimeJournal` trait only has `append()` and `drain_for_shutdown()` methods. `VolatileRuntimeJournal` does not implement journal replay capability.

**Test Expectation:** After creating a new shard with the same journal and calling `tick()`, the run should complete by replaying the stored `AskAnswered` journal event.

**Implementation Gap:** No replay mechanism exists. The journal would need either:
1. An `events_for_run()` method to retrieve events
2. A `replay()` method on the shard to process journal events

### 2. `red_ask_answer_secret_redaction` - BLOCK_LOCAL

**Error:**
```
assertion `left == right` failed
  left: Err(SecretResultNotAllowed)
 right: Ok(true)
```

**Root Cause Cause Analysis:**
The test uses `ask_workflow()` which sets `resource_contract: ResourceContract::DEFAULT`. `ResourceContract::DEFAULT` has `allows_secret_results: false` (line 231 in `workflow/mod.rs`).

The implementation correctly rejects secret-tainted answers when the contract doesn't allow secret results (lines 326-330 in `lifecycle.rs`):
```rust
if answer.taint == Taint::Secret
    && !state.workflow.resource_contract().allows_secret_results
{
    return Err(RuntimeError::SecretResultNotAllowed);
}
```

**Test Expectation:** The test expects `Ok(true)` when using `Taint::Secret`, suggesting it expects the answer to be accepted and trace to be checked for redaction.

**Test Comment vs Implementation:**
- Comment says: "FAILURE: TraceEvent::AskAnswered emits slot value without taint-gate check"
- But actual behavior: Implementation rejects at gate before creating trace event
- The test uses `ask_workflow()` which has `allows_secret_results = false`

**Incompatible Test Design:** `red_ask_answer_secret_redaction` expects success with `Taint::Secret`, but `red_test_taint_secret_rejected_without_permission` (which passes) expects error with the same `Taint::Secret` and same workflow. These tests have opposite expectations for identical inputs.

## Conclusion

State 6 implementation remains BLOCK_LOCAL. Two failures require design decisions:

1. **Durable replay** - requires adding journal replay capability to `VolatileRuntimeJournal` or `Shard`
2. **Secret redaction test** - test expects behavior incompatible with `ResourceContract::DEFAULT` settings

Cannot advance to State 7 until these are resolved.
