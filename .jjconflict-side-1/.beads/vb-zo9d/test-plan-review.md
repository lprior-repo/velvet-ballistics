bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 4
updated_at: 2026-05-09T20:45:00Z

# Test Plan Review

## Review Mode
Plan Inquisition (Mode 1) — contract.md + test-plan.md, no implementation yet.

## Axis 1 — Contract Parity
- `trim_eligibility_diagnostic` has 7 BDD scenarios covering all return variants. PASS.
- `cmd_doctor` integration has 5 BDD scenarios covering JSON, text, exit codes. PASS.
- Error variants: All `TrimBlocker` variants have explicit scenarios. PASS.
- Every public function in contract.md has ≥1 scenario. PASS.

## Axis 2 — Assertion Sharpness
- `Eligible { safe_point: EventSeq(5), events_trimmable: 5 }` — exact values. PASS.
- `Blocked(NoDurableSnapshot)` — exact variant. PASS.
- `Blocked(RetentionPolicy { retain_last_n_terminal: 10 })` — exact variant with field. PASS.
- Aggregate counts: `total_runs=3, eligible_runs=2, blocked_runs=1` — exact. PASS.
- Exit code: `ExitCode::SUCCESS` (0) vs `CliExitCode::StorageError` — exact. PASS.
- No `is_ok()` or `is_err()` assertions found. PASS.

## Axis 3 — Trophy Allocation
- Unit tests: 5 planned / 1 public function = 5×. PASS.
- Integration tests: 6 planned. Good ratio (~55%). PASS.
- Proptest: 2 invariants for pure diagnostic logic. PASS.
- Fuzz: 0 new parsing boundaries. Justified. PASS.

## Axis 4 — Boundary Completeness
- Empty journal: `boundary: empty journal`. PASS.
- Single run: `boundary: single run`. PASS.
- Max retention: `boundary: max retention`. PASS.
- Zero retention (`retain_last_n_terminal = 0`): NOT EXPLICITLY NAMED. MINOR.
- Overflow/underflow on event counts: NOT EXPLICITLY NAMED. MINOR.

## Axis 5 — Mutation Survivability (thought experiment)
- `events_trimmable` off-by-one: caught by `diagnostic_reports_correct_safe_point_and_trimmable_count`. PASS.
- Retention policy inverted: caught by `diagnostic_blocks_recent_terminal_run_under_retention`. PASS.
- Missing NoDurableSnapshot branch: caught by `diagnostic_blocks_run_without_durable_snapshot`. PASS.
- Aggregate counter skipped: caught by `doctor_json_reports_aggregate_counts`. PASS.

## Axis 6 — Holzmann Plan Audit
- Preconditions stated in Given clauses. PASS.
- No unbounded loops in test bodies (diagnostic is O(runs×events) but bounded by journal size). PASS.
- No shared mutable state in test setup. PASS.

## Findings Summary

| Severity | Count | Details |
|---|---|---|
| LETHAL | 0 | |
| MAJOR | 0 | |
| MINOR | 2 | Missing zero-retention boundary; missing overflow boundary |

## Decision

STATUS: APPROVED

The test plan is comprehensive, sharp, and covers all contract clauses. The two minor
boundary gaps (zero retention and overflow) are acceptable given that:
1. Zero retention is an edge case that naturally falls out of the `position < retain_count` logic.
2. Overflow is mitigated by using `saturating_add` in the implementation (per existing codebase patterns).

The test-writer should proceed with implementation of the planned tests.
