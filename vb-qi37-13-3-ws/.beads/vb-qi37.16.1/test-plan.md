bead_id: vb-qi37.16.1
bead_title: cli/runtime: Implement durable cancel transition
phase: 4
updated_at: 2026-05-09T00:00:00Z

# Test Plan: Durable Cancel Transition

## Summary
- Behaviors identified: 18
- Trophy allocation: 8 unit / 8 integration / 2 e2e
- Proptest invariants: 3
- Fuzz targets: 2
- Kani harnesses: 3

## 1. Behavior Inventory

1. CLI parses cancel command with run_id, db path, and optional reason.
2. CLI rejects invalid run_id with structured error.
3. CLI rejects reason longer than 256 bytes.
4. CLI outputs structured JSON/JSONL on success.
5. CLI outputs human-readable confirmation on success.
6. Runtime enqueues cancel command to the correct shard.
7. Shard cancels an active run, persists journal event with reason.
8. Shard cancels a suspended run, persists journal event.
9. Shard cancels a waiting run, cleans pending timer, persists journal event.
10. Shard increments failed counter exactly once for first cancel.
11. Shard is idempotent: second cancel of same run is silent no-op.
12. Shard is idempotent: cancel of finished run is silent no-op.
13. Shard is idempotent: cancel of failed run is silent no-op.
14. Journal encodes/decodes RunCancelled event with reason roundtrip.
15. Journal encodes/decodes RunCancelled event without reason roundtrip.
16. IPC payload encodes/decodes CancelRun with reason roundtrip.
17. Storage-level integration: cancel CLI -> runtime -> journal -> read-back verifies reason.
18. E2E: full CLI invocation with --json produces expected output shape.

## 2. Trophy Allocation

| Layer | Count | Behaviors | Rationale |
|-------|-------|-----------|-----------|
| Unit | 8 | 1-6, 14-16 | Pure logic: parsing, encoding, counter semantics |
| Integration | 8 | 7-13, 17 | Component interactions: shard, journal, storage |
| E2E | 2 | 18, structured output | Full CLI black-box |
| Static | - | All | Clippy, forbidden patterns enforced by CI |

Deviation: 44% unit, 44% integration, 11% e2e. Slightly more unit than target due to extensive encoding/parsing logic. Acceptable.

## 3. BDD Scenarios

### Behavior 1: CLI parses cancel command
```
Given: velvet_ballastics binary with cancel subcommand
When: user runs "cancel <run_id> --db <path> --reason 'user request' --json"
Then: args parse into Command::Cancel { run_id, db, reason: Some("user request"), output: Json }
```
Test: `fn cli_parses_cancel_with_reason_and_json_output()`

### Behavior 2: CLI rejects invalid run_id
```
Given: cancel command with malformed run_id
When: args are parsed
Then: ParseError::InvalidRunId is returned
```
Test: `fn cli_rejects_malformed_run_id_when_parsing_cancel()`

### Behavior 3: CLI rejects reason longer than 256 bytes
```
Given: cancel command with reason of 257 bytes
When: args are parsed
Then: ParseError::ReasonTooLong is returned
```
Test: `fn cli_rejects_reason_longer_than_256_bytes_when_parsing_cancel()`

### Behavior 4: CLI outputs structured JSON on success
```
Given: a run exists in the journal
When: cancel command succeeds with --json
Then: output contains {"success": true, "run_id": "...", "status": "cancelled", "reason": "..."}
```
Test: `fn cancel_json_output_contains_success_run_id_and_status()`

### Behavior 5: CLI outputs human-readable confirmation
```
Given: a run exists in the journal
When: cancel command succeeds without --json
Then: stdout contains "Run <run_id> cancelled"
```
Test: `fn cancel_text_output_confirms_cancellation()`

### Behavior 6: Runtime enqueues cancel command
```
Given: a runtime with one shard and an active run
When: runtime.cancel_run(run_id, Some(reason)) is called
Then: ShardCommand::Cancel { run, reason } is enqueued to the shard
```
Test: `fn runtime_enqueues_cancel_command_to_shard()`

### Behavior 7: Shard cancels active run with journal
```
Given: a shard with an active run and a journal
When: handle_cancel(run, Some(reason)) is called
Then: RunCancelled event with reason is appended to journal, run is removed, frame released, trace pushed, counter incremented
```
Test: `fn shard_cancel_active_run_persists_journal_with_reason()`

### Behavior 8: Shard cancels suspended run
```
Given: a shard with a suspended run (awaiting ask)
When: handle_cancel(run, None) is called
Then: run is removed, journal event written without reason
```
Test: `fn shard_cancel_suspended_run_persists_journal()`

### Behavior 9: Shard cancels waiting run and cleans timer
```
Given: a shard with a run waiting on a timer
When: handle_cancel(run, None) is called
Then: pending timer is removed, run is removed, journal event written
```
Test: `fn shard_cancel_waiting_run_cleans_timer_and_persists_journal()`

### Behavior 10: Counter increments exactly once
```
Given: a shard with an active run
When: handle_cancel is called once
Then: failed counter is 1
When: handle_cancel is called again on same run
Then: failed counter remains 1
```
Test: `fn shard_cancel_increments_failed_counter_exactly_once()`

### Behavior 11: Second cancel is silent no-op
```
Given: a shard where a run was already cancelled
When: handle_cancel is called again on same run_id
Then: Ok(()) is returned, no journal event, no trace event, counter unchanged
```
Test: `fn shard_double_cancel_is_idempotent_no_events()`

### Behavior 12: Cancel of finished run is no-op
```
Given: a shard where a run has finished
When: handle_cancel is called
Then: Ok(()) is returned, no journal event, no counter increment
```
Test: `fn shard_cancel_finished_run_is_silent_no_op()`

### Behavior 13: Cancel of failed run is no-op
```
Given: a shard where a run has failed
When: handle_cancel is called
Then: Ok(()) is returned, no journal event, no counter increment
```
Test: `fn shard_cancel_failed_run_is_silent_no_op()`

### Behavior 14: Journal roundtrip with reason
```
Given: a RunCancelled event with reason "user request"
When: encoded to bytes and decoded
Then: decoded event equals original including reason
```
Test: `fn journal_event_run_cancelled_roundtrip_with_reason()`

### Behavior 15: Journal roundtrip without reason
```
Given: a RunCancelled event with reason None
When: encoded to bytes and decoded
Then: decoded event equals original with reason None
```
Test: `fn journal_event_run_cancelled_roundtrip_without_reason()`

### Behavior 16: IPC payload roundtrip with reason
```
Given: CancelRun payload with run_id and reason
When: serialized to bytes and deserialized
Then: deserialized payload equals original
```
Test: `fn ipc_cancel_run_payload_roundtrip_with_reason()`

### Behavior 17: Storage integration end-to-end
```
Given: a Fjall journal with a submitted run
When: cancel CLI is invoked with reason
Then: reading journal events for run returns RunCancelled with same reason
```
Test: `fn integration_cancel_cli_persists_reason_to_journal()` (in durability_matrix_integration.rs)

### Behavior 18: E2E CLI JSON output shape
```
Given: a Fjall journal with a run
When: "velvet_ballastics cancel <run_id> --db <path> --json" is executed
Then: stdout is valid JSON with success=true, run_id, status="cancelled"
```
Test: `fn e2e_cancel_cli_json_output_shape()` (inline in main.rs tests or integration)

## 4. Proptest Invariants

### Proptest: run_id parsing
Invariant: Any string that parses to RunId is non-zero; any zero input is rejected.
Strategy: `prop_oneof!["0", "00", any<u64>().prop_map(|n| n.to_string())]`
Anti-invariant: "0" always fails.

### Proptest: reason length validation
Invariant: Strings of length 0-256 bytes are accepted; 257+ bytes are rejected.
Strategy: `any<Vec<u8>>().prop_map(|v| String::from_utf8_lossy(&v).to_string())` with length filter.
Anti-invariant: 257-byte string always fails.

### Proptest: cancel idempotency
Invariant: Any sequence of cancels on the same run ID produces at most one journal event.
Strategy: `vec(CancelCommand, 1..20)` all targeting same run.
Anti-invariant: Two journal events for same run never occurs.

## 5. Fuzz Targets

### Fuzz Target: run_id string parsing
Input type: str
Risk: Panic on malformed input, incorrect parsing of edge cases.
Corpus seeds: "0", "1", "18446744073709551615", "", "abc", "0x1"

### Fuzz Target: CancelRun IPC payload decoding
Input type: bytes
Risk: Panic on malformed payload, incorrect deserialization of optional reason field.
Corpus seeds: valid encoded payload, truncated payload, extra bytes.

## 6. Kani Harnesses

### Kani Harness: run_id non-zero invariant
Property: RunId::new(0) returns None; RunId::new(n) for n > 0 returns Some.
Bound: u64 input space (symbolic).
Rationale: Critical invariant for run identification.

### Kani Harness: reason length boundary
Property: validate_reason_length returns Ok for len <= 256, Err for len > 256.
Bound: String length up to 300 bytes.
Rationale: Prevents unbounded memory usage and ensures consistent rejection.

### Kani Harness: counter increment once
Property: After handle_cancel on existing run, failed_counter == old_counter + 1; after second cancel, failed_counter == old_counter + 1.
Bound: Small state machine with 2 runs, 2 cancels.
Rationale: Ensures idempotency at the counter level.

## 7. Mutation Checkpoints

Critical mutations to survive:
- `handle_cancel` removing the `if self.runs.contains_key(&run)` guard → must be caught by duplicate-journal test.
- `handle_cancel` removing `self.counters.inc_failed()` → must be caught by counter test.
- `cmd_cancel` removing reason validation → must be caught by reason-length test.
- `cmd_cancel` removing JSON output branch → must be caught by JSON output test.
- Journal encoding omitting reason field → must be caught by roundtrip test.

Threshold: 90% mutation kill rate minimum.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| cancel active run | valid run_id, reason=None | journal event, trace, counter+1 | integration |
| cancel active run with reason | valid run_id, reason=Some | journal event with reason | integration |
| cancel suspended run | run awaiting ask | journal event, run removed | integration |
| cancel waiting run | run with pending timer | timer removed, journal event | integration |
| cancel nonexistent run | run_id not in shard | Ok(()), no events | unit |
| cancel finished run | run already finished | Ok(()), no events | unit |
| cancel failed run | run already failed | Ok(()), no events | unit |
| double cancel | two cancels on same run | one journal event, counter+1 | integration |
| parse cancel no reason | missing --reason | Command::Cancel { reason: None } | unit |
| parse cancel with reason | --reason "x" | Command::Cancel { reason: Some("x") } | unit |
| parse cancel json | --json | output=Json | unit |
| parse cancel invalid run_id | run_id="abc" | Err(InvalidRunId) | unit |
| parse cancel long reason | 257 bytes | Err(ReasonTooLong) | unit |
| journal roundtrip with reason | RunCancelled with reason | decoded == original | unit |
| journal roundtrip without reason | RunCancelled without reason | decoded == original | unit |
| IPC roundtrip with reason | CancelRun with reason | decoded == original | unit |
| JSON output | --json, success | {success:true, run_id, status} | unit |
| text output | no --json, success | "Run X cancelled" | unit |

## Open Questions
- None.
