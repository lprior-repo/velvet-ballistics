bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-4-test-plan
updated_at: 2026-05-09T00:00:00Z

# Test Plan: Safe Journal Trimming

## Summary
- Behaviors identified: 12
- Trophy allocation: 7 unit / 4 integration / 1 e2e
- Proptest invariants: 3
- Fuzz targets: 1
- Kani harnesses: 2

## 1. Behavior Inventory

1. **Trim deletes only events older than the durable snapshot** — Events with seq < snapshot_seq are removed.
2. **Trim preserves events at or after the snapshot** — Events with seq >= snapshot_seq remain.
3. **Trim is idempotent** — Second trim with same safe point returns NoOp.
4. **Trim without durable snapshot fails closed** — No events deleted, error returned.
5. **Trim preserves run headers** — Header keyspace untouched.
6. **Trim preserves all snapshots** — Snapshot keyspace untouched.
7. **Terminal retention policy blocks recent terminal runs** — Runs among the N most recent terminal runs per workflow are skipped.
8. **Terminal retention allows older terminal runs** — Runs outside the retention window are trimmed.
9. **Non-terminal runs are not subject to retention** — Active runs trim normally.
10. **Trim_all skips runs without durable snapshots** — Iterates headers, only trims eligible.
11. **Trim_all reports per-run results** — Returns a `TrimmedRunResult` for each trimmed run.
12. **Replay equivalence after trim** — Hydrating from trimmed journal + snapshot yields same state as full journal.

## 2. Trophy Allocation

| Layer | Count | Rationale |
|---|---|---|
| Unit | 7 | Pure logic: idempotency, boundary, error variants, retention check |
| Integration | 4 | Real Fjall dependency: full round-trip, multi-run, replay equivalence |
| E2E | 1 | Storage doctor integration (trim eligibility reporting) |
| Static | — | Clippy + compile-time checks already enforced by CI |

Deviation: Higher unit ratio than typical because trimming logic is mostly pure key-comparison with minimal I/O surface. Integration tests cover the Fjall boundary.

## 3. BDD Scenarios

### Behavior 1: Trim deletes only events older than the durable snapshot
```
Given: A run with events seq 0..9 and a durable snapshot at seq 5
When: trim_events_for_run is called
Then: Events seq 0..4 are deleted, events seq 5..9 remain
And: TrimmedRunResult reports deleted_count=5, status=Trimmed
```
Test: `fn trim_deletes_events_older_than_durable_snapshot()`

### Behavior 2: Trim preserves events at or after the snapshot
```
Given: A run with events seq 0..3 and a durable snapshot at seq 2
When: trim_events_for_run is called
Then: Events seq 0..1 are deleted, events seq 2..3 remain
And: No event with seq >= 2 is missing
```
Test: `fn trim_preserves_events_at_or_after_snapshot()`

### Behavior 3: Trim is idempotent
```
Given: A run already trimmed at snapshot seq 5
When: trim_events_for_run is called again with same policy
Then: Returns TrimStatus::NoOp with deleted_count=0
```
Test: `fn trim_is_idempotent_on_already_trimmed_run()`

### Behavior 4: Trim without durable snapshot fails closed
```
Given: A run with events but no durable snapshot
When: trim_events_for_run is called
Then: Returns Err(TrimError::NoDurableSnapshot)
And: No events are deleted
```
Test: `fn trim_without_durable_snapshot_fails_closed()`

### Behavior 5: Trim preserves run headers
```
Given: A run with a header and events
When: trim_events_for_run succeeds
Then: The run header is still readable with identical content
```
Test: `fn trim_preserves_run_header()`

### Behavior 6: Trim preserves all snapshots
```
Given: A run with multiple snapshots at different sequences
When: trim_events_for_run succeeds
Then: All snapshots remain readable
```
Test: `fn trim_preserves_all_snapshots()`

### Behavior 7: Terminal retention policy blocks recent terminal runs
```
Given: A terminal run with a durable snapshot, and retention policy retain_last_n_terminal=3
When: The run is among the 3 most recent terminal runs for its workflow
Then: trim_events_for_run returns Err(TrimError::RetentionPolicyBlocks)
```
Test: `fn terminal_retention_blocks_recent_terminal_runs()`

### Behavior 8: Terminal retention allows older terminal runs
```
Given: 5 terminal runs for the same workflow, retention policy retain_last_n_terminal=3
When: trim_all_eligible_runs is called
Then: The 2 oldest terminal runs are trimmed, the 3 newest are retained
```
Test: `fn terminal_retention_allows_older_terminal_runs()`

### Behavior 9: Non-terminal runs are not subject to retention
```
Given: An active (non-terminal) run with a durable snapshot
When: trim_events_for_run is called with any retention policy
Then: The run is trimmed normally, not blocked by retention
```
Test: `fn non_terminal_runs_ignore_retention_policy()`

### Behavior 10: Trim_all skips runs without durable snapshots
```
Given: Run A has a durable snapshot, Run B does not
When: trim_all_eligible_runs is called
Then: Only Run A is trimmed; Run B is silently skipped
```
Test: `fn trim_all_skips_runs_without_durable_snapshot()`

### Behavior 11: Trim_all reports per-run results
```
Given: Multiple eligible runs
When: trim_all_eligible_runs is called
Then: Returns a Vec with one TrimmedRunResult per trimmed run
And: No result for skipped runs
```
Test: `fn trim_all_reports_per_run_results()`

### Behavior 12: Replay equivalence after trim
```
Given: A run with events and a durable snapshot
When: Events are trimmed, then the run is recovered via recover_snapshot_plus_tail
Then: The recovered state matches the state recovered from the full journal
```
Test: `fn replay_equivalence_after_trim()` (integration test)

## 4. Proptest Invariants

### Proptest 1: trim_preserves_replay_equivalence
```
Invariant: For any valid event sequence and snapshot position,
  replay after trim == replay before trim
Strategy: Vec<JournalEvent> with contiguous sequences, snapshot at random seq > 0
Anti-invariant: Snapshot seq > max event seq (should still hold — no trim)
```

### Proptest 2: trim_idempotence
```
Invariant: trim(trim(events, policy), policy) == trim(events, policy)
Strategy: Any valid journal state + any TrimPolicy
Anti-invariant: None (always holds for valid inputs)
```

### Proptest 3: retention_policy_never_trims_retained_runs
```
Invariant: For any set of terminal runs and retention count N,
  at most (total - N) runs are trimmed per workflow
Strategy: Vec of terminal runs with varying accepted_at_ms, random N >= 0
Anti-invariant: N > total terminal runs (should trim 0)
```

## 5. Fuzz Targets

### Fuzz Target 1: snapshot_prefix_key byte math
```
Input type: bytes (RunId + snapshot sequence)
Risk: Panic in byte slicing, incorrect key construction
Corpus seeds: RunId=0, RunId=u64::MAX, seq=0, seq=u64::MAX
```
Note: Low priority; key construction is simple concatenation. Covered by unit tests.

## 6. Kani Harnesses

### Kani Harness 1: verify_trim_boundary
```
Property: For any event sequence number s and cutoff c,
  if s >= c, the event is NOT in the deletion set
Bound: seq and cutoff as u64, limited to reasonable range for BMC
Rationale: Proves I4 (cutoff safety) for ALL inputs, not just sampled
```

### Kani Harness 2: verify_idempotence
```
Property: For any journal state J and policy P,
  trim(trim(J, P), P) == trim(J, P)
Bound: Small fixed-size event set (3 events) to keep state space tractable
Rationale: Proves I2 (idempotence) mathematically
```

## 7. Mutation Checkpoints

Critical mutations to survive:
- Changing `<` to `<=` in the seq comparison → caught by `trim_preserves_events_at_or_after_snapshot`
- Removing the `NoDurableSnapshot` check → caught by `trim_without_durable_snapshot_fails_closed`
- Removing retention policy check → caught by `terminal_retention_blocks_recent_terminal_runs`
- Changing `skip_noop_runs` default from true to false → caught by idempotency test

Threshold: 90% mutation kill rate minimum.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| happy path | events + snapshot | Ok(Trimmed) | unit |
| idempotency | already trimmed | Ok(NoOp) | unit |
| no snapshot | events, no snapshot | Err(NoDurableSnapshot) | unit |
| retention block | terminal + recent | Err(RetentionPolicyBlocks) | unit |
| retention allow | terminal + old | Ok(Trimmed) | unit |
| non-terminal | active run | Ok(Trimmed) | unit |
| empty journal | no events | Ok(NoOp) | unit |
| boundary: seq == cutoff | event at cutoff | preserved | unit |
| boundary: seq == cutoff-1 | event just before | deleted | unit |
| multi-run mixed | some eligible, some not | Vec of eligible only | integration |
| replay equivalence | full round-trip | state equality | integration |

## Open Questions
- How is terminal state detected for retention? Via `events_for_run` replay to find terminal event, or via run header status byte? (Answer: Use `recovery::replay::core::extract_terminal` for canonical detection.)
- Should `retain_last_n_terminal` be 0 to disable retention entirely? (Yes, 0 means no terminal runs are retained; all terminal runs are eligible for trimming.)
- What timestamp orders terminal runs for retention? `accepted_at_ms` from run header, or the sequence number of the terminal event? (Use `accepted_at_ms` from header for stable ordering.)
