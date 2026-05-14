bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 4
updated_at: 2026-05-09T20:40:00Z

# Test Plan: Report journal trim eligibility in doctor

## Summary
- Behaviors identified: 12
- Trophy allocation: 5 unit / 6 integration / 1 e2e
- Proptest invariants: 2
- Fuzz targets: 0 (no new parsing boundaries)
- Kani harnesses: 1

## 1. Behavior Inventory

1. `FjallJournal::trim_eligibility_diagnostic` returns per-run eligibility for all runs.
2. Eligible runs report their safe point and events_trimmable count.
3. Runs without durable snapshots are reported as blocked with `NoDurableSnapshot`.
4. Terminal runs protected by retention policy are reported as blocked with `RetentionPolicy`.
5. Non-terminal runs are never blocked by retention policy.
6. The diagnostic never mutates the journal (pure read-only).
7. Running the diagnostic twice produces identical results.
8. `cmd_doctor` includes a `trim_eligibility` check in JSON output.
9. `cmd_doctor` includes a `trim_eligibility` check in text output.
10. `cmd_doctor` reports aggregate counts (total, eligible, blocked, events_trimmable).
11. `cmd_doctor` returns SUCCESS when the journal is healthy, even if trimming is recommended.
12. `cmd_doctor` returns StorageError when the journal cannot be opened.

## 2. Trophy Allocation

| Layer | Count | Rationale |
|---|---|---|
| Unit | 5 | Pure logic: eligibility classification, blocker detection, aggregate counting |
| Integration | 6 | Doctor command JSON/text output, exit codes, end-to-end with real journal |
| E2E | 1 | Full CLI invocation on temp database |
| Static | - | Existing clippy gates cover code style |

Deviation: Slightly more integration than typical (55%) because the primary value is in the CLI output correctness.

## 3. BDD Scenarios

### Behavior 1: Diagnostic returns per-run eligibility
```
Given: A journal with 2 runs, one with a snapshot and one without
When: trim_eligibility_diagnostic is called
Then: Returns one Eligible run and one Blocked(NoDurableSnapshot) run
```
Test: `fn diagnostic_returns_eligible_and_blocked_runs`

### Behavior 2: Eligible runs report safe point
```
Given: A run with events seq 0..10 and a snapshot at seq 5
When: trim_eligibility_diagnostic is called
Then: The run is Eligible with safe_point=5 and events_trimmable=5
```
Test: `fn diagnostic_reports_correct_safe_point_and_trimmable_count`

### Behavior 3: No snapshot blocks trim
```
Given: A run with events but no snapshot
When: trim_eligibility_diagnostic is called
Then: The run is Blocked with blocker=NoDurableSnapshot
```
Test: `fn diagnostic_blocks_run_without_durable_snapshot`

### Behavior 4: Retention policy blocks recent terminal runs
```
Given: A terminal run that is among the 10 most recent terminal runs for its workflow
When: trim_eligibility_diagnostic is called with default policy
Then: The run is Blocked with blocker=RetentionPolicy
```
Test: `fn diagnostic_blocks_recent_terminal_run_under_retention`

### Behavior 5: Non-terminal runs ignore retention
```
Given: A non-terminal run with a snapshot
When: trim_eligibility_diagnostic is called
Then: The run is Eligible regardless of retention policy
```
Test: `fn diagnostic_allows_non_terminal_run_despite_retention`

### Behavior 6: Diagnostic is read-only
```
Given: A journal with events and a snapshot
When: trim_eligibility_diagnostic is called
Then: All events are still present; no mutations occurred
```
Test: `fn diagnostic_does_not_delete_events`

### Behavior 7: Diagnostic is idempotent
```
Given: Any journal state
When: trim_eligibility_diagnostic is called twice in a row
Then: Both calls return identical TrimDiagnostic results
```
Test: `fn diagnostic_is_idempotent`

### Behavior 8: Doctor JSON includes trim eligibility
```
Given: A healthy journal with trimmable events
When: doctor --db <path> --json is invoked
Then: JSON output contains a check with name="trim_eligibility" and status="pass"
```
Test: `fn doctor_json_includes_trim_eligibility_check`

### Behavior 9: Doctor text includes trim eligibility
```
Given: A healthy journal with trimmable events
When: doctor --db <path> is invoked
Then: Text output mentions trim eligibility and safe point
```
Test: `fn doctor_text_reports_trim_eligibility`

### Behavior 10: Doctor reports aggregate counts
```
Given: A journal with 3 runs (2 eligible, 1 blocked)
When: doctor --db <path> --json is invoked
Then: JSON includes total_runs=3, eligible_runs=2, blocked_runs=1
```
Test: `fn doctor_json_reports_aggregate_counts`

### Behavior 11: Doctor returns SUCCESS for healthy journal
```
Given: A healthy journal that needs trimming
When: doctor --db <path> is invoked
Then: Exit code is 0 (SUCCESS)
```
Test: `fn doctor_returns_success_for_healthy_journal_with_trim_recommended`

### Behavior 12: Doctor returns StorageError for unreadable path
```
Given: A non-existent or unreadable database path
When: doctor --db <path> is invoked
Then: Exit code indicates StorageError
```
Test: `fn doctor_returns_storage_error_for_unreadable_path`

## 4. Proptest Invariants

### Proptest 1: Diagnostic idempotency
```
Invariant: For any journal state, two consecutive diagnostic calls produce identical results.
Strategy: Generate journals with 0..20 runs, each with 0..50 events and optional snapshots.
Anti-invariant: N/A (should always hold).
```

### Proptest 2: Events never decrease after diagnostic
```
Invariant: The total event count across all runs never decreases after a diagnostic call.
Strategy: Generate journals with events and snapshots.
Anti-invariant: N/A (should always hold).
```

## 5. Fuzz Targets

None. No new parsing or deserialization boundaries are introduced by this feature.
The existing fjall codec boundaries are already covered by separate beads.

## 6. Kani Harnesses

### Kani Harness 1: No panic in eligibility classification
```
Property: trim_eligibility_diagnostic never panics for valid journal state.
Bound: Bounded to small run/event counts due to fjall I/O.
Rationale: Doctor must be safe to run during incident triage; panics are unacceptable.
Note: Full Kani verification may require mocking fjall. If fjall mocking is infeasible,
      this obligation is waived with compensating Miri + manual QA evidence.
```

## 7. Mutation Checkpoints

Critical mutations to survive:
- `events_trimmable` calculation: must be caught by `diagnostic_reports_correct_safe_point_and_trimmable_count`
- Retention policy check logic: must be caught by `diagnostic_blocks_recent_terminal_run_under_retention`
- `NoDurableSnapshot` vs `Eligible` branch: must be caught by `diagnostic_blocks_run_without_durable_snapshot`
- Aggregate counting logic: must be caught by `doctor_json_reports_aggregate_counts`

Threshold: 90% mutation kill rate minimum.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy path: eligible run | run with snapshot, not terminal | Eligible { safe_point, events_trimmable } | unit |
| happy path: no-op run | run with snapshot, nothing to trim | Eligible { safe_point, events_trimmable=0 } | unit |
| error variant: no snapshot | run without snapshot | Blocked(NoDurableSnapshot) | unit |
| error variant: retention blocked | terminal run, recent | Blocked(RetentionPolicy) | unit |
| boundary: empty journal | no runs | TrimDiagnostic with all zeros | unit |
| boundary: single run | one run, one snapshot | correct counts | unit |
| boundary: max retention | retain_last_n_terminal = u32::MAX | all terminal runs blocked | unit |
| invariant: idempotency | any journal state | identical results on second call | proptest |
| invariant: read-only | any journal state | event counts unchanged | proptest |
| integration: JSON output | healthy journal with trimmable events | JSON contains trim_eligibility check | integration |
| integration: text output | healthy journal with trimmable events | text mentions trim | integration |
| integration: exit code | healthy journal | ExitCode::SUCCESS | integration |
| e2e: CLI invocation | temp database path | exit 0, readable output | e2e |

## Open Questions

None. The contract and codebase provide sufficient context.
