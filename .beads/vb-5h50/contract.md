bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-3-contract
updated_at: 2026-05-09T00:00:00Z

# Contract Specification — Safe Journal Trimming

## Context
- **Feature**: Extend `crates/vb_storage/src/trimming.rs` with safe journal trimming that respects snapshot durability and terminal-run retention.
- **Domain terms**:
  - *Durable snapshot*: A `RunSnapshot` that has been persisted with `SyncAll` (fsync) to the underlying Fjall database, guaranteeing it survives a crash.
  - *Safe replay point*: The sequence number of the latest durable snapshot for a run. Events with `seq < safe_point` are reconstructable from the snapshot alone.
  - *Terminal run*: A run that has reached a terminal state (`RunFinished`, `RunCancelled`, or `RunFailed`).
  - *Retention policy*: Rules governing how long terminal-run evidence must be kept after trimming becomes eligible.
- **Assumptions**:
  - Fjall `PersistMode::SyncAll` guarantees durability.
  - `events_for_run(run)` already replays from the latest snapshot, establishing the snapshot-as-replay-baseline invariant.
  - Run headers store a `status` byte that can be interpreted for terminal-state detection.
- **Open questions**:
  - Should retention policy be per-workflow or global? ( bead implies global default with per-workflow override capability )
  - Should the snapshot durability confirmation be tracked explicitly (e.g., a `durable_snapshots` keyspace) or inferred from `put_snapshot` always using strict durability? ( Answer: for now, `put_snapshot` writes without strict durability; trim must only consider snapshots that have been explicitly confirmed durable. )

## Preconditions
- [P1] The journal keyspace must be open and readable.
- [P2] The run whose events are being trimmed must have at least one snapshot.
- [P3] The snapshot used as the safe replay point must be confirmed durable (fsynced).
- [P4] If the run is terminal, the retention policy must permit trimming (e.g., retention count not exceeded).

## Postconditions
- [Po1] After a successful trim, no event with `seq < safe_replay_point` remains in the journal for that run.
- [Po2] After a successful trim, replaying the run from the snapshot yields the same state as replaying from the original full journal.
- [Po3] After a successful trim, the run header and all snapshots are preserved.
- [Po4] If the run is terminal and retention policy prohibits trimming, the journal is untouched.
- [Po5] If no durable snapshot exists for the run, the trim operation returns an error and makes no changes.

## Invariants
- [I1] **No acknowledged state is recoverable only from deleted events.** Any state that could be reconstructed from deleted events must also be reconstructable from the durable snapshot at the safe replay point.
- [I2] **Trim idempotency.** Trimming the same run twice with the same safe replay point is a no-op (or returns `TrimStatus::NoOp`).
- [I3] **Terminal retention minimum.** Terminal runs are never trimmed younger than the retention policy specifies.
- [I4] **Cutoff boundary safety.** Events at or after the safe replay point are never deleted.

## Error Taxonomy
- `TrimError::NoDurableSnapshot { run }` — No durable snapshot exists for this run. Fails closed; no events deleted.
- `TrimError::RetentionPolicyBlocks { run }` — The run is terminal but retention policy requires keeping it.
- `TrimError::Fjall(fjall::Error)` — Underlying storage operation failed.
- `TrimError::Journal(JournalError)` — Underlying journal operation failed.
- `TrimError::IncompleteTrim { deleted_count }` — Trim was interrupted mid-operation. Partial state may exist.

## Contract Signatures

```rust
/// Extended retention policy for terminal runs.
pub struct TrimPolicy {
    /// If true, skip runs that have no events to trim (no-op runs).
    pub skip_noop_runs: bool,
    /// Number of most-recent terminal runs per workflow to retain.
    /// A run is eligible for trimming only if it is NOT among the
    /// `retain_last_n_terminal` most recent terminal runs for its workflow.
    pub retain_last_n_terminal: u32,
}

impl Default for TrimPolicy {
    fn default() -> Self {
        Self {
            skip_noop_runs: true,
            retain_last_n_terminal: 10,
        }
    }
}

impl FjallJournal {
    /// Returns the latest *durable* snapshot sequence for a run.
    /// A snapshot is considered durable only if it has been explicitly
    /// confirmed as fsynced (e.g., written via `put_snapshot_strict` or
    /// confirmed by a subsequent `persist_strict` call).
    pub fn latest_durable_snapshot_seq(&self, run: RunId) -> TrimResult<Option<EventSeq>>;

    /// Trims journal events for a specific run, respecting durability and retention.
    ///
    /// - Finds the latest durable snapshot.
    /// - If none, returns `TrimError::NoDurableSnapshot`.
    /// - If the run is terminal and retention policy blocks, returns
    ///   `TrimError::RetentionPolicyBlocks`.
    /// - Deletes all events with `seq < snapshot_seq`.
    /// - Returns `TrimStatus::NoOp` if no events were eligible.
    pub fn trim_events_for_run(
        &self,
        run: RunId,
        policy: TrimPolicy,
    ) -> TrimResult<TrimmedRunResult>;

    /// Trims all runs with durable snapshots, skipping runs blocked by retention.
    pub fn trim_all_eligible_runs(&self, policy: TrimPolicy) -> TrimResult<Vec<TrimmedRunResult>>;
}
```

## Non-goals
- Changing the snapshot format or serialization.
- Implementing the `doctor` command (covered by bead `vb-zo9d`).
- Adding per-workflow retention policy overrides (future enhancement).
- Automatic/background trimming (this bead provides the safe primitive only).

## Verification Layers
- **Unit tests**: Idempotency, boundary conditions, retention blocking, no-snapshot error.
- **Property tests**: Replay equivalence before/after trim for arbitrary event sequences.
- **Integration tests**: Full trim + recovery round-trip.

## Traceability
| Clause | Test Obligation | Proof Obligation |
|---|---|---|
| I1 No lost state | `test_replay_equivalence_after_trim` | Property test |
| I2 Idempotency | `test_second_trim_is_idempotent` | Unit test |
| I3 Terminal retention | `test_terminal_retention_policy_blocks_trim` | Unit test |
| I4 Cutoff safety | `test_trim_cannot_delete_events_at_or_after_cutoff` | Unit test |
| Po5 No durable snapshot fails closed | `test_trim_without_durable_snapshot_fails_closed` | Unit test |
