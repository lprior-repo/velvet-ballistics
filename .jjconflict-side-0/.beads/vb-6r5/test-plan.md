bead_id: vb-6r5
phase: 7
updated_at: 2026-05-18T02:05:00Z

# Test Plan - State 7

## Test Strategy
Tests derive from contract requirements (R1-R7), traceability matrix, and approved proof obligations (P1-P5).

## Unit Tests

### T1-T5: CLI Command Parsing (R1)
- T1: `list-crates` command parses correctly
- T2: `proof list` command parses correctly
- T3: `proof run` with all profile variants parses correctly
- T4: `proof crate <name>` with lane list parses correctly
- T5: `proof affected --base <rev>` parses correctly

### T6-T7: Profile Lane Selection (R2, P5)
- T6: Each profile selects the correct lane set
- T7: Profile monotonicity: fast ⊆ standard ⊆ deep ⊆ proof ⊆ all

### T8-T11: DAG Scheduler (R3, P1-P3)
- T8: Empty crate list produces empty schedule
- T9: Single crate produces single-level schedule
- T10: Linear dependency chain produces sequential levels
- T11: Independent crates grouped in same parallel level

### T12-T13: Structured Logging (R4)
- T12: JSONL log entry serializes correctly
- T13: Summary aggregates results correctly

### T14-T15: Workspace Discovery (R5)
- T14: cargo metadata output parsed into CrateInfo
- T15: Excluded crates filtered correctly

### T16-T18: CLI Flags (R6, P4)
- T16: --jobs auto resolves to num_cpus
- T17: --jobs 0 rejected
- T18: --jobs -1 rejected

### T19-T20: Exit Code Behavior (R7)
- T19: All lanes pass → exit 0
- T20: Required lane fails → exit non-zero

## Property Tests

### P1: DAG Topological Order
- Generate random DAGs (up to 20 nodes)
- Verify scheduler output respects topological order
- 1000 proptest cases

### P2: Dependency Ordering
- Generate random DAGs
- Verify no crate appears before any of its dependencies in schedule
- 1000 proptest cases

## Integration Tests
- End-to-end: Run `cargo xtask proof run --profile fast --dry-run` and verify output
- End-to-end: Run `cargo xtask list-crates --json` and verify JSON output

## Test Files
- `xtask/src/discovery.rs` — Tests inline
- `xtask/src/scheduler.rs` — Tests inline + proptest module
- `xtask/src/lanes.rs` — Tests inline
- `xtask/src/logger.rs` — Tests inline
- `xtask/src/profiles.rs` — Tests inline
- `xtask/src/summary.rs` — Tests inline
- `xtask/src/cli.rs` — Tests inline
