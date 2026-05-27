# Codebase Map — vb-om21

## Bead

- Bead: `vb-om21`
- State: 2 / explore
- Scope: storage tests for journal tail scan fallback.
- Requested test file: `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` — **MISSING** in the isolated workspace; implementation state will likely need to create/register this integration test under `crates/workspace_tests`.
- External Restate reference: `/tmp/opencode/restate/crates/log-server/src/rocksdb_logstore/store.rs` — **MISSING** on this host during exploration. No-copy fence still applies; do not copy Restate code, APIs, layouts, async architecture, HTTP/JSON paths, or storage model.

## Master Contract Anchors

- `velvet-ballistics-MASTER.md:21-25`: runtime is single-server, no-unsafe/no-panic, numeric, deterministic, Fjall-backed, Postcard-backed, no runtime YAML/JSON/HTTP.
- `velvet-ballistics-MASTER.md:53-60`: Fjall stores run headers, journal events, snapshots, blobs, indexes; failures are typed; evidence must be executable.
- `velvet-ballistics-MASTER.md:753-804`: Fjall persistence contract, `run_event` key layout, snapshot-plus-tail/full replay, corrupt records fail typed, Fjall keys big-endian.
- `velvet-ballistics-MASTER.md:1218-1224`: public storage/recovery surface names include `FjallJournal::open`, append methods, key constructors, and recovery functions.
- `velvet-ballistics-MASTER.md:2812-2837`: storage contract repeats big-endian key ordering and full/snapshot-tail recovery requirements.

## Relevant Crates and Manifests

- `crates/vb_storage/Cargo.toml`
  - Package: `vb_storage`.
  - Dependencies relevant to this bead: `fjall`, `postcard`, `arrayvec`, `thiserror`, `vb_core`.
  - Dev dependencies: `tempfile`, `proptest`.
- `crates/workspace_tests/Cargo.toml`
  - Package: `velvet-ballistics-workspace-tests`.
  - Existing tests list includes Restate-derived storage tests (`restate_journal_side_index_contracts`, `restate_postcard_envelope_wire_tests`, `restate_runtime_version_barrier_tests`) but does **not** include the requested `restate_journal_tail_scan_fallback_tests` target.
  - Bead command text says `cargo nextest run -p velvet-ballastics-workspace-tests ...`; manifest package spelling is `velvet-ballistics-workspace-tests`.

## Core Storage Surface

### Key Encoding

- `crates/vb_storage/src/keys.rs`
  - `run_event_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>` at lines 40-43.
  - `journal_key(run, seq)` delegates to `sequenced_run_key(PREFIX_RUN_EVENT, run, seq)` at lines 133-135.
  - `sequenced_run_key` writes `[prefix][run_id_u64_be][seq_u64_be]` at lines 137-150.
  - `run_prefix_key(run)` returns `[0x11][run_id_u64_be]` at lines 177-180; this is the keyspace boundary seam needed to prove tail scans do not cross into another run.

### Journal Open and Keyspaces

- `crates/vb_storage/src/journal/core.rs`
  - `FjallJournal` owns `events: fjall::Keyspace` and other keyspaces at lines 50-67.
  - `FjallJournal::open` creates the `run_event` keyspace at lines 87-89 and returns the journal at lines 106-121.
  - `declared_keyspaces()` returns all 9 storage keyspaces at lines 124-138.

### Append / Raw Event Access / Replay

- `crates/vb_storage/src/journal/append.rs`
  - Public append methods: `append_journaled`, `append_strict`, `append_strict_batch`, `persist_strict`.
- `crates/vb_storage/src/journal/internal.rs`
  - `append_unpersisted` encodes `JournalEvent` under `run_event_key(event.run_id(), event.seq())`, rejects duplicate keys, and inserts into `self.events` at lines 27-48.
- `crates/vb_storage/src/journal/replay.rs`
  - `events_for_run` delegates to `events_for_run_bounded` at lines 52-55.
  - `get_event_bytes(run, seq)` exposes raw journal bytes for integration tests at lines 57-70.
  - `events_for_run_bounded` starts after latest snapshot when present; otherwise starts at seq 0 at lines 72-86.
  - `events_for_run_from` uses `snap.range(&self.events, start_key..)` and stops on `key.starts_with(&run_prefix)` failure at lines 88-120. This is the existing prefix-bound scan seam most relevant to a tail fallback implementation.
  - Replay validates contiguous event sequence via `validate_replay_sequence` at lines 123-132 and enforces collection bound via `push_replay_event` at lines 134-158.
- `crates/vb_storage/src/journal/injection.rs`
  - `inject_raw_event` and `inject_seq_gap` can write raw keys/records for disaster recovery/test setup. These are public but dangerous; use only if append APIs cannot create the needed corruption/suspect metadata test shape.

### Recovery Surface

- `crates/vb_storage/src/recovery/mod.rs`
  - Re-exports `recover_full_journal`, `recover_snapshot_plus_tail`, `recover_runtime_summary`, `recover_runtime_frame_seed`, `replay_events`, `summarize_recovery_events`, and types.
- `crates/vb_storage/src/recovery/recover.rs`
  - `recover_runtime_summary`, `recover_runtime_summary_with_expected`, and `recover_runtime_frame_seed` all call `journal.events_for_run(run)` and return `RecoveryError::NoRecoveryData { run }` when no events exist.
  - `recover_all_incomplete_runs` scans `journal.run_headers()` then `events_for_run(header.run)`; missing events currently map to `NoRecoveryData`.
- `crates/vb_storage/src/recovery/types.rs`
  - `RecoveryError` variants currently include `Journal(#[from] JournalError)`, digest mismatches, `NoRecoveryData`, `CorruptSnapshot`, `TerminalStateMismatch`, and `FrameDimensionOverflow`.
  - Requested bead names `TailMismatch` and `MissingJournal` are **not present** in the inspected `RecoveryError` variants.
- `crates/vb_storage/src/trimming/logic.rs`
  - `latest_durable_snapshot_seq` scans snapshot keys for a run prefix and validates key/payload run+seq agreement at lines 12-56. This is a nearby pattern for max-seq scan + typed mismatch checks, but it scans snapshots, not journal tail.

## Existing Tests to Reuse or Avoid Colliding With

- `crates/workspace_tests/tests/restate_fjall_keyspace_manifest_tests.rs`
  - Existing big-endian and prefix tests.
  - `run_event_ordering` at lines 121-142 proves lexicographic order follows `(run_id, seq)` ordering.
  - `max_sequence_ordering` at lines 145-163 proves `u64::MAX - 1` sorts before `u64::MAX` for one run.
  - `declared_keyspaces_*` at lines 169-214 cover all 9 keyspaces.
  - `cross_keyspace_non_collision` at lines 272-304 covers prefix isolation across sampled keyspaces.
- `crates/workspace_tests/tests/restate_journal_side_index_contracts.rs`
  - Contains public integration examples that use `run_event` key lookup and journal event visibility through `get_event_bytes`; relevant as a style/reference for workspace tests.
- `crates/vb_storage/src/journal/tests.rs`
  - Existing unit coverage for sequence ordering, gaps, duplicate events, snapshot-plus-tail replay, and bounded replay. Useful for implementation-level behavior, but bead explicitly names workspace integration tests.
- `crates/vb_storage/src/trimming/tests.rs`
  - Existing latest snapshot scan tests, including payload run/seq mismatch typed error cases. Useful as a pattern for suspected tail metadata mismatch semantics.
- `crates/vb_storage/src/recovery/tests.rs`, `recovery/recovery_unit_tests.rs`, `recovery/vb_h6ix_tests.rs`
  - Existing recovery behavior and `NoRecoveryData` tests. Collision risk if new typed variants replace broad `NoRecoveryData` behavior.

## Verification / Proof Artifacts Already Nearby

- `verification/verus/vb_jpq724_events_for_run_production.rs`
  - Production-bound Verus contracts for `FjallJournal::events_for_run` and `events_for_run_from`; relevant if implementation changes replay scan semantics.
- `verification/verus/vb_jnz9_journal_event_seq_valid.rs`
  - Event sequence validity proof model.
- `verification/verus/vb_rpch_hydrate_snapshot_tail.rs` and `verification/verus/vb_rpch_hydrate_preconditions.rs`
  - Snapshot-tail recovery precondition models; relevant if tail fallback semantics feed snapshot-tail hydration.
- `crates/vb_storage/src/kani_storage_invariants.rs`
  - Kani invariants around `WrongRun` and `SequenceGap` replay validation.
- `crates/vb_storage/src/kani_recovery_hydrate.rs`
  - Kani coverage for recovery hydration and `NoRecoveryData` behavior.

## Missing or Unclear Surfaces

- No inspected file exposes a tail metadata record or a public tail metadata API. `RunHeaderRecord` has no tail field (`crates/vb_storage/src/records.rs:241-258`).
- No inspected error type contains requested `TailMismatch` or `MissingJournal` names.
- No target test file exists yet, and `crates/workspace_tests/Cargo.toml` has no `[[test]]` entry for it.
- External Restate reference file is unavailable, so downstream agents must rely on bead text and local VB contracts, not external code.
- The exact public API for "reconstruct journal tail by scanning final big-endian journal key" is not yet present by name. Likely candidates are a new `FjallJournal` query method or a recovery helper that uses `run_prefix_key` + reverse/range iteration over `self.events`.

## Risk Tags

- `persistence`: Fjall keyspace behavior and durable journal tail semantics.
- `recovery`: recovery entry points depend on `events_for_run` and typed no-data/mismatch handling.
- `public-api`: integration tests probably require a public helper or observable recovery path.
- `bounded-resource`: scan must be prefix-bounded and should not collect/scan across unrelated keyspace ranges.
- `typed-error`: requested `TailMismatch` and `MissingJournal` are absent and must be modeled without weakening existing typed errors.
- `contract-parity`: bead asks tests first and specific typed errors; implementation must not silently map suspect metadata to successful recovery.
- `no-copy-fence`: Restate source missing and forbidden as implementation material.

## Likely Commands

- Scoped test once target is registered: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_journal_tail_scan_fallback_tests`
- Existing key ordering baseline: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_fjall_keyspace_manifest_tests`
- Storage unit blast-radius if replay/tail helper changes: `cargo nextest run -p vb_storage journal::tests trimming::tests recovery::tests`
- Canonical final gate per repository contract: `moon ci`

## Collision Notes

- Bead command text has package typo `velvet-ballastics-workspace-tests`; manifest says `velvet-ballistics-workspace-tests`.
- Creating the requested integration test file also requires adding a `[[test]]` entry to `crates/workspace_tests/Cargo.toml` unless Cargo auto-discovers it for this package layout; current manifest explicitly lists many tests and omits this one.
- Adding `TailMismatch`/`MissingJournal` to `RecoveryError` may affect tests asserting `NoRecoveryData` for empty journals. Keep semantics explicit: empty run-event keyspace may return zero tail for a tail scan helper while recovery without data may still be `NoRecoveryData` unless bead contract requires otherwise.
- Any tail scan over Fjall must stop at `run_prefix_key(run)` and never treat another run's lexicographically adjacent event as the requested run's tail.
