# vb-jpq7.24 Proof-to-Rust Bridge

STATUS: BRIDGE EVIDENCE, NOT DIRECT VERUS PRODUCTION PROOF

## Scope decision

`verification/verus/vb_jpq724_events_for_run_production.rs` is downgraded to a
Verus-checked mirror model. It is not a direct Verus binding to production Rust,
because the production crate is ordinary Rust and does not expose Verus
`requires`/`ensures` contracts on the actual functions.

No PASS claim in this bead treats mirror-model verification as proof of live
Fjall I/O, keyspace iteration, allocation behavior, or the actual Rust function
bodies. The mirror model is useful only as a checked specification of the replay
contract that the following source refs and tests must realize.

## Production source mapping

| Mirror clause | Production source ref | Production behavior |
|---|---|---|
| `spec_exec_next_seq_contract` | `crates/vb_storage/src/codec/mod.rs:46-51` | `seq.get().checked_add(1).map(EventSeq::new).ok_or(JournalError::SequenceOverflow)` rejects overflow as typed `SequenceOverflow`. |
| `spec_exec_event_validation_contract` | `crates/vb_storage/src/codec/mod.rs:53-71` | Wrong run returns `JournalError::WrongRun`; wrong seq returns `JournalError::SequenceGap`; exact run+seq returns `Ok(())`. |
| `spec_exec_snapshot_start_contract` | `crates/vb_storage/src/journal/replay.rs:58-71` | `latest_durable_snapshot_seq(run)?` propagates snapshot errors; no snapshot starts at `EventSeq::new(0)`; valid snapshot starts at `next_seq(seq)?`; replay delegates to `events_for_run_from`. |
| strict per-run replay ordering | `crates/vb_storage/src/journal/replay.rs:73-117` | iteration begins at `run_event_key(run, start_seq)?`, stops at nonmatching `run_prefix`, decodes each event, calls `validate_replay_sequence`, and advances expected seq with `next_seq`. |
| replay bound/allocation failures | `crates/vb_storage/src/journal/replay.rs:119-143` | limit overflow maps to `TooManyEvents`; reservation failure maps to `ReplayAllocationFailed`. |

## Independent executable evidence

The bridge relies on scoped Rust tests, not on the Verus mirror alone:

- `rtk cargo test -p vb_storage events_for_run -- --nocapture` — 24 tests pass.
- Covered production paths include missing first tail after snapshot,
  snapshot lookup error propagation source audit, no-snapshot initial sequence,
  wrong-run/wrong-sequence replay validation, event limit failure, and empty run
  behavior.

Raw command evidence is recorded in `.evidence/vb-jpq7.24/raw-logs.md` and
`.evidence/vb-jpq7.24/cargo-test-vb-storage-events-for-run.log`.

## Non-vacuity statement

The Verus model can fail on the intended hazards: changing mirror next-seq to
wrap, changing event validation to ignore wrong run/sequence, or changing
snapshot-start classification to erase typed errors breaks the model contracts.
The production bridge can fail independently: changing the actual Rust source to
erase `latest_durable_snapshot_seq(run)?`, skip `validate_replay_sequence`, or
remove checked `next_seq` is detected by the scoped source/tests cited above.

## Limitations

- This is not an inline Verus proof of actual Rust bodies.
- This is not independent proof-review approval.
- Fjall engine correctness and OS durability are outside this Verus bridge.
