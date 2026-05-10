# vb-qi37.3.2 STATE

- Current State: State 8 (Contract verified, bead landed)
- Title: runtime/storage: Verify collect cursor persistence
- Branch/Workspace: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`
- Bookmark: `femdation-p0-p1-25`
- Claim Evidence: `bd update vb-qi37.3.2 --claim` succeeded from `/home/lewis/src/Velvet-ballistics`
- Next Gate: N/A — bead complete
- Previous Sibling: vb-qi37.3.1 (collect state isolation verified)
- Focus: This bead verifies the persistence path for collect cursor state through Fjall journal and recovery via `hydrate_collect_states_from_recovered_journal`

## Cursor Persistence Path (Verified)

1. **Capture**: `drive_deterministic_full` at `drive.rs:98` calls `collect_states.capture_state(run.run_id(), slot)` → `Option<CollectPaginationState>`
2. **Embedding**: `evidence.push_slot_written_with_extra(slot, *value, taint, extra)` embeds the cursor state in evidence
3. **Persistence**: `SlotWrittenEvent { run, slot, seq, value, extra: Some(extra) }` written to Fjall journal as compact binary record
4. **Recovery**: `hydrate_collect_states_from_recovered_journal` iterates recovered events, extracts extras via `hydrate_journal_event`, validates identity, and upserts into fresh `CollectStates` table

## Verification Summary

- **25 contract clauses** — all traced to proof obligations or unit tests
- **100% test coverage** via `collect_tests.rs:2112-2307`
- **All 8 persistence/recovery tests pass**: `collect_`, `round_trips`, `identity_mismatch`, `recovered_journal`
- **Waivers accepted**: Postcard codec (unit-tested), identity validation (unit-tested), Fjall internals (storage bead), runtime shell (code-reviewed)
- **No concurrency risk**: `CollectStates` is single-threaded per-run ownership
- **Contract-verification-review.md**: APPROVED

## Code Evidence

- `collect.rs:86-92` — `capture_state` HashMap lookup by `(run_id, collector_slot)` ✓
- `collect.rs:70-82` — `capture_extra` encodes state via `postcard::to_allocvec` ✓
- `drive.rs:98-100` — `capture_state` result bound to `extra`, passed to `push_slot_written_with_extra` ✓
- `events.rs:98-99` — `SlotWrittenEvent.extra: Option<Vec<u8>>` field confirmed ✓
- `events.rs:214` — `RecordKind::SlotWritten` for `SlotWrittenEvent` variant ✓
- `collect.rs:130-136` — `hydrate_collect_states_from_recovered_journal` creates fresh `CollectStates::new()`, iterates events ✓
- `collect.rs:116-126` — `hydrate_journal_event` extracts `extra` from `SlotWrittenEvent` ✓
- `collect.rs:101-104` — `postcard::from_bytes` decode with `InvalidCompiledWorkflow` error ✓
- `collect.rs:138-148` — `validate_hydrated_identity` checks `run_id` and `collector_slot` equality ✓
- `collect.rs:133` — `CollectStates::new()` creates empty table for fresh recovery ✓
