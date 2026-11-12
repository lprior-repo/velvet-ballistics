bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-6-complete
updated_at: 2026-05-09T00:00:00Z

# Implementation Summary

## Changes Made

### `crates/vb_storage/src/trimming.rs`
1. **Extended `TrimPolicy`**:
   - Added `retain_last_n_terminal: u32` field (default: 10)
   - Retention policy prevents trimming of the N most recent terminal runs per workflow

2. **Extended `TrimError`**:
   - Renamed `NoSnapshot` → `NoDurableSnapshot` for contract alignment
   - Added `RetentionPolicyBlocks { run }` error variant
   - Added `RETENTION_POLICY_BLOCKS_CODE` diagnostic code (0x4103)

3. **Renamed `latest_snapshot_seq` → `latest_durable_snapshot_seq`**:
   - Aligns with MASTER.md §73 contract language
   - Same behavior: scans keyspace for highest-seq snapshot

4. **Added retention policy enforcement**:
   - `has_terminal_event(run)`: Scans all journal events for a run to detect terminal state
   - `check_retention_policy(run, policy)`: Enforces `retain_last_n_terminal` by:
     - Finding all terminal runs for the same workflow
     - Sorting by `accepted_at_ms` descending
     - Blocking trim if run's position < retention count

5. **Updated `trim_events_for_run`**:
   - Calls `check_retention_policy` after snapshot validation
   - Returns `RetentionPolicyBlocks` if run is retained

6. **Updated `trim_all_eligible_runs`**:
   - Now skips both `NoDurableSnapshot` and `RetentionPolicyBlocks` runs

7. **Added 8 new tests**:
   - `trim_preserves_events_at_or_after_snapshot` — boundary safety
   - `terminal_retention_blocks_recent_terminal_runs` — retention blocking
   - `terminal_retention_allows_older_terminal_runs` — retention allowing
   - `non_terminal_runs_ignore_retention_policy` — non-terminal exemption
   - `replay_equivalence_after_trim` — state preservation proof
   - `trim_policy_default_includes_retention` — default policy verification
   - `no_durable_snapshot_error_has_correct_diagnostic_code` — error taxonomy
   - `retention_policy_blocks_error_has_correct_diagnostic_code` — error taxonomy

### `crates/vb_storage/src/journal.rs`
1. **Updated `events_for_run`** to use `latest_durable_snapshot_seq`
2. **Made `events_for_run_from` `pub(crate)`** for internal crate access

## Contract Clause Mapping

| Contract Clause | Implementation | Test |
|---|---|---|
| I1 No lost state | Snapshot-based replay | `replay_equivalence_after_trim` |
| I2 Idempotency | `<` comparison + skip_noop_runs | `trim_is_idempotent_on_already_trimmed_run` |
| I3 Terminal retention | `check_retention_policy` | `terminal_retention_blocks_recent_terminal_runs` |
| I4 Cutoff safety | `seq_u64 < cutoff_seq.get()` | `trim_preserves_events_at_or_after_snapshot` |
| Po5 No durable snapshot fails closed | `NoDurableSnapshot` error | `trim_without_durable_snapshot_fails_closed` |

## Test Results
- `cargo test -p vb_storage`: 871 passed, 0 failed
- `cargo test -p vb_storage trimming`: 15 passed, 0 failed
