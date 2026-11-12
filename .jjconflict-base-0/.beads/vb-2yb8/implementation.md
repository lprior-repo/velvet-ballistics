# Implementation Notes — vb-2yb8

## Changes Made

### 1. New Module: `crates/vb_runtime/src/durability_matrix.rs`
- `DurabilityRow` struct: maps primitive → IR node kind → journal events → storage partition → ack point → replay assertion → test evidence
- `DURABILITY_MATRIX` const: 11 rows covering all YAML primitives
- `REQUIRED_PRIMITIVES` const: canonical primitive list from MASTER.md §10
- Verifier functions:
  - `verify_matrix_completeness()`: every primitive has a row
  - `verify_matrix_replay_proofs()`: every row has test evidence
  - `verify_ack_after_persist()`: no row claims ack-before-persist
  - `verify_matrix()`: runs all verifications
- `DurabilityError` enum: typed errors for each failure mode

### 2. Module Registration
- Added `pub mod durability_matrix;` to `crates/vb_runtime/src/lib.rs`

### 3. Integration Tests: `crates/vb_runtime/tests/durability_matrix_integration.rs`
- 6 persistence-before-ack tests using `VolatileRuntimeJournal`:
  - `submit_handler_persists_before_ack`
  - `action_completed_persists_before_ack`
  - `action_failed_persists_before_ack`
  - `ask_answered_persists_before_ack`
  - `cancel_persists_before_ack`
  - `timer_fired_persists_before_ack`
- 3 gate tests:
  - `gate_fails_when_primitive_row_is_missing`
  - `gate_fails_when_row_omits_replay_evidence`
  - `gate_fails_when_row_claims_ack_before_persist`

### 4. Unit Tests
- 10 unit tests in `durability_matrix.rs` covering:
  - Matrix completeness
  - Replay proof existence
  - Ack ordering
  - Per-row correctness for set, do, wait, ask, finish

## Ack Point Audit Results

All handlers in `lifecycle.rs` append to journal before returning Ok:

| Handler | Line | Pattern |
|---------|------|---------|
| handle_submit | 109-117 | journal.append before runs.insert |
| handle_action_completion | 192-207 | journal.append before drive_run |
| handle_legacy_action_completion | 225-229 | journal.append before drive_run |
| handle_action_failure | 248-252 | journal.append before match outcome |
| handle_ask_answer | 333-351 | journal.append before drive_run |
| handle_timer | 363-373 | journal.append before drive_state |
| handle_cancel | 379-380 | journal.append before runs.swap_remove |

## Pre-existing Issues Noted

- `crates/vb_runtime/src/engine/property_tests.rs` has compilation errors (unrelated to this bead)
- `crates/vb_runtime/src/shard/tests.rs` lib tests fail to compile due to pre-existing issues
- Integration tests compile and pass successfully

## Test Results

```
cargo test -p vb_runtime --test durability_matrix_integration
  9 passed
```

## Follow-up Beads Identified

1. **Meta-primitive rows**: ErrorHandler and Retry are not YAML primitives but are runtime constructs that emit journal events. Consider adding meta-rows.
2. **Storage partition mapping**: Currently uses logical partitions (RuntimeJournal, ActionJournal, TimerJournal). Need to map to actual Fjall keyspace names.
3. **Replay equivalence tests**: The matrix claims replay assertions but lacks automated replay-from-journal tests.
4. **CI gate wiring**: The matrix verifier needs to be wired into `moon run :ci`.
