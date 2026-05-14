# Contract: vb-qi37.3.2 — Collect Cursor Persistence Verification

## Context

- **Feature**: Collect cursor persistence through Fjall journal and recovery via `hydrate_collect_states_from_recovered_journal`
- **Domain terms**:
  - `CollectPaginationState`: Durable cursor `(run_id, collector_slot, source, current_page, cursor, page_size, item_count, limit, time_limit_ms, start_millis)`
  - `SlotWrittenEvent { extra: Option<Vec<u8>> }`: Journal event carrying embedded cursor state
  - `hydrate_collect_states_from_recovered_journal`: Recovery function that rebuilds `CollectStates` from journal events
  - `validate_hydrated_identity`: Identity validator preventing cross-run cursor contamination
- **Assumptions**:
  - `drive_deterministic_full` already captures cursor state via `collect_states.capture_state(run.run_id(), slot)` at `drive.rs:98` (proven in vb-qi37.3.1 Q9)
  - `SlotWrittenEvent.extra` field carries postcard-encoded `CollectPaginationState` bytes
  - Fjall journal persistence uses the binary envelope defined in master doc Section 18
- **Open questions**: None

## Isolation Theorem (Inherited from vb-qi37.3.1)

**Cross-run collect cursor persistence is safe: `(RunId, SlotIdx)` compound key boundary is preserved through the Fjall persistence layer.**

## Persistence Preconditions

| ID | Description | Verified |
|----|-------------|----------|
| PP1 | `capture_state(run.run_id(), slot)` returns `Some(state)` where `state.run_id == run.run_id()` and `state.collector_slot == slot` | `collect.rs:86-92` |
| PP2 | `evidence.push_slot_written_with_extra` receives the extra from `capture_state` | `drive.rs:98-100` |
| PP3 | `SlotWrittenEvent.extra` carries postcard-encoded `CollectPaginationState` via Postcard | `collect.rs:76-78`, `events.rs:98-99` |
| PP4 | Journal record encodes `SlotWrittenEvent` with `record_kind = RecordKind::SlotWritten` | `events.rs:214` |

## Persistence Postconditions

| ID | Description | Evidence |
|----|-------------|----------|
| PQ1 | Cursor capture embeds correct `run_id`: `capture_state(run_a, slot_a).run_id == run_a` | Structural proof at `collect.rs:86-92` |
| PQ2 | Cursor capture embeds correct `collector_slot`: `capture_state(run, slot_a).collector_slot == slot_a` | Structural proof at `collect.rs:86-92` |
| PQ3 | Extra bytes round-trip through Postcard encode/decode | `collect_tests.rs:2112-2154` |
| PQ4 | Journal `SlotWrittenEvent { extra: Some(bytes) }` is written to Fjall | `collect_tests.rs:2193-2258` (uses `append_strict`) |
| PQ5 | Recovery via `hydrate_collect_states_from_recovered_journal` reconstructs exact cursor state | `collect_tests.rs:2238-2258` |
| PQ6 | Recovered cursor resumes `collect_next` at correct position | `collect_tests.rs:2247-2251` |

## Recovery Preconditions

| ID | Description | Verified |
|----|-------------|----------|
| RP1 | `hydrate_journal_event` extracts `extra` from `SlotWrittenEvent { extra: Some(extra), .. }` | `collect.rs:116-126` |
| RP2 | `hydrate_extra(run, slot, extra)` validates `extra.run_id == run` and `extra.collector_slot == slot` | `collect.rs:138-148` |
| RP3 | Identity mismatch in `hydrate_extra` returns `EngineError::InvalidCompiledWorkflow` | `collect.rs:143-147` |
| RP4 | Corrupt extra bytes in `hydrate_extra` return `EngineError::InvalidCompiledWorkflow` | `collect.rs:101-104` |
| RP5 | `hydrate_collect_states_from_recovered_journal` creates empty `CollectStates` then hydrates all extras | `collect.rs:130-136` |

## Recovery Postconditions

| ID | Description | Evidence |
|----|-------------|----------|
| RQ1 | Empty journal produces empty `CollectStates` | `collect_tests.rs:2188-2189` (implicit in error path) |
| RQ2 | Single `SlotWrittenEvent { extra }` with valid identity produces correct `CollectPaginationState` | `collect_tests.rs:2238-2258` |
| RQ3 | `SlotWrittenEvent { extra }` with corrupt bytes fails with `InvalidCompiledWorkflow` | `collect_tests.rs:2185-2190` |
| RQ4 | `SlotWrittenEvent { extra }` with wrong `run_id` fails with `InvalidCompiledWorkflow` | `collect_tests.rs:2301-2306` |
| RQ5 | `SlotWrittenEvent { extra }` with wrong `collector_slot` fails with `InvalidCompiledWorkflow` | `collect_tests.rs:2285-2306` |
| RQ6 | Recovered state resumes `collect_next` at correct page/cursor | `collect_tests.rs:2247-2251` |

## Invariants

| ID | Invariant | Static Proof |
|----|-----------|--------------|
| PI1 | Cursor identity preserved through capture → embed → persist → recover cycle | `drive.rs:98` → `collect.rs:76-78` → `events.rs:98-99` → `collect.rs:130-136` |
| PI2 | No cursor state is lost: every `capture_state` result that is `Some` is encoded in extra | `drive.rs:98` + evidence chain |
| PI3 | Cross-run contamination impossible through persistence/recovery path | `validate_hydrated_identity` at `collect.rs:138-148` |
| PI4 | Recovery creates fresh `CollectStates` (no cross-contamination from old entries) | `collect.rs:133` (`CollectStates::new()`) |

## Error Taxonomy

| Error | Condition |
|-------|-----------|
| `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" }` | Postcard decode of extra bytes fails |
| `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" }` | Decoded `run_id` or `collector_slot` differs from event identity |

All errors are typed `Result<T, EngineError>` — no panic path exists.

## Contract Signatures

```rust
// Persistence capture
pub fn capture_state(&self, run_id: RunId, collector_slot: SlotIdx) -> Option<CollectPaginationState>

// Evidence embedding
pub fn push_slot_written_with_extra(&mut self, slot: SlotIdx, value: SlotValue, taint: Taint, extra: Option<CollectPaginationState>)

// Recovery
pub fn hydrate_collect_states_from_recovered_journal(events: &[JournalEvent]) -> Result<CollectStates, EngineError>
pub fn hydrate_extra(&mut self, run_id: RunId, collector_slot: SlotIdx, extra: &[u8]) -> Result<(), EngineError>
```

## Out of Scope

- Fjall internals (separate storage bead)
- `collect_start`/`collect_next`/`collect_finish` algorithm (proven in vb-qi37.3.1)
- Snapshot-based recovery (separate recovery bead)
- Concurrent shard behavior (no concurrency in collect primitive)

## Relationship to vb-qi37.3.1

This bead extends vb-qi37.3.1's isolation proof to cover the persistence and recovery path:
- vb-qi37.3.1 proved: `capture_state` uses correct `(RunId, SlotIdx)` key
- vb-qi37.3.2 proves: cursor state survives Fjall persistence and recovery with identity preserved
