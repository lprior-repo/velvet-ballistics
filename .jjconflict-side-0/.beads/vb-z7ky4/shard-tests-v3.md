# Shard Policy Decision Sequence Tests — Evidence

**Bead:** vb-z7ky4
**Test file:** `crates/vb_runtime/src/shard/lifecycle_tests/strict_policy_sequence.rs`
**Module wiring:** `crates/vb_runtime/src/shard/lifecycle.rs` → child module `strict_policy_sequence`

## Scenario Mapping

| Scenario | Test Name | Implementation Detail |
|----------|-----------|----------------------|
| Successful strict port | `successful_strict_policy_port` | Real shard, actual pending ticket from `suspended_workflow()`, `RecordingJournal` captures seq, verifies shard seq advances and pending ticket unchanged |
| Typed strict port failure | `typed_strict_policy_port_failure` | Same setup; `RecordingJournal` returns `RuntimeError::UnsupportedOperation` from strict port, shard propagates error, seq and pending ticket unchanged |
| Overflow at u64::MAX | `overflow_at_u64_max` | Uses `shard.seed_journal_sequence(run, u64::MAX)` to force `checked_next_seq` overflow before journal call; verifies no journal call occurred and seq remains at MAX |
| Wrong ticket rejected | `wrong_ticket_rejected` | Builds ticket with `ActionId::new(999)` instead of pending ticket's `ActionId::new(0)`; `require_pending_action_ownership` rejects with `InvalidActionCompletion` before journal call |

## Key Design Decisions

- **Real shard construction:** All 4 tests use `Shard::new_with_journal` with actual `suspended_workflow()` submitted via `ShardCommand::SubmitWithInputsAndContracts`, then `tick()` to reach genuine pending-action state. No manual ticket insertion or fake run state.
- **RecordingJournal:** Test-local implementation wrapping `Arc<Mutex<>>` state. Records `(DecisionRecord, EventSeq)` tuples for strict port calls. Supports injecting typed errors for failure tests.
- **Overflow test:** Uses `seed_journal_sequence` to set run sequence to `u64::MAX`. The shard's `checked_next_seq` arithmetic overflows before any journal port call, proving the contract: "reject before append".
- **Wrong ticket test:** Changes only the `action` field of the pending ticket. `require_pending_action_ownership` detects the mismatch and returns `InvalidActionCompletion`.

## Contract Coverage

- ✅ Shard owns sequence (checked arithmetic before append)
- ✅ Checked sequence before append (u64::MAX overflow test)
- ✅ Exact pending-ticket ownership validation (wrong ticket test)
- ✅ Strict port called with current seq (successful port test)
- ✅ Next seq published only on success (verified in successful port test)
- ✅ Errors propagate from journal port (typed failure test)
- ✅ No map capacity mutation (uses existing map infrastructure)
- ✅ Full 9-field DecisionRecord preserved (test fixture passes complete record)

## Limitations

- `RecordingJournal` does not verify physical fsync or power-loss durability. The test proves port ordering and error propagation, not storage durability.
- Overflow test depends on `seed_journal_sequence` accepting u64::MAX directly; actual runs would need to exhaust the entire sequence space.
