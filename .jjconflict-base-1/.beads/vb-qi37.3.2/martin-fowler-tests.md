# Martin Fowler Test Plan: vb-qi37.3.2 — Collect Cursor Persistence

## Overview

This test plan covers the collect cursor persistence path: capture → embed → persist → recover. All tests verify behavior against the contract clauses established in `contract.md`.

## Test Naming Convention

- **Format**: `test_<scenario>_<expected_behavior>`
- **Location**: `crates/vb_runtime/src/collect_tests.rs`
- **Coverage**: All happy paths, error paths, and edge cases for cursor persistence and recovery

---

## Happy Path Tests

### test_collect_pagination_extra_round_trips_for_recovery

**Given**: A `CollectPaginationState` captured from an active collect operation with `cursor=1, page_size=1`

**When**: The extra bytes are captured via `capture_extra`, then hydrated via `hydrate_extra` into a fresh `CollectStates`

**Then**:
- `capture_extra` returns `Some(extra_bytes)` containing postcard-encoded state
- `hydrate_extra` returns `Ok(())`
- The hydrated state has `run_id`, `collector_slot`, `cursor=1`, `page_size=1` matching the original

**Contract clauses**: PP1, PP3, PQ1, PQ2, PQ3

---

### test_collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page

**Given**: A Fjall journal containing `SlotWrittenEvent { run, slot, extra }` where `extra` encodes cursor state at `cursor=1`

**When**: `hydrate_collect_states_from_recovered_journal` is called with the recovered events

**Then**:
- Returns `Ok(hydrated_states)` containing the correct cursor
- `hydrated_states.capture_state(run, slot)` returns `Some(state)` with `cursor=1`
- `collect_next` resumed with the hydrated state advances to `cursor=2`

**Contract clauses**: PQ4, PQ5, PQ6, RP1, RP5, RQ2, RQ6

---

## Error Path Tests

### test_collect_pagination_extra_rejects_corrupt_bytes

**Given**: An empty `CollectStates` and corrupt extra bytes `[255, 0, 7]`

**When**: `hydrate_extra(RunId::new(1), SlotIdx::new(1), corrupt_bytes)` is called

**Then**: Returns `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" })`

**Contract clauses**: RP4, RQ3

---

### test_collect_journal_extra_rejects_corrupt_bytes

**Given**: A `JournalEvent::SlotWrittenEvent { extra: Some(corrupt_bytes) }`

**When**: `hydrate_journal_events(&[event])` is called

**Then**: Returns `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" })`

**Contract clauses**: RP1, RP4, RQ3

---

### test_collect_pagination_extra_recovered_journal_rejects_corrupt_bytes

**Given**: A Fjall journal with a `SlotWrittenEvent` carrying corrupt extra bytes

**When**: The journal is recovered and passed to `hydrate_collect_states_from_recovered_journal`

**Then**: Returns `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" })`

**Contract clauses**: RP5, RQ3

---

### test_collect_pagination_extra_rejects_identity_mismatch

**Given**: A captured extra from `run_a` and `slot_a`

**When**: `hydrate_extra(run_b, slot_a, extra)` is called where `run_b != run_a`

**Then**: Returns `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" })`

**Contract clauses**: RP2, RP3, PI3

---

### test_collect_journal_extra_rejects_identity_mismatch

**Given**: A `JournalEvent::SlotWrittenEvent { run: run_b, slot: slot_a, extra }` where the extra was captured for `run_a`

**When**: `hydrate_journal_events(&[event])` is called

**Then**: Returns `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" })`

**Contract clauses**: RP1, RP2, RP3, PI3

---

### test_collect_pagination_extra_recovered_journal_rejects_identity_mismatch

**Given**: A Fjall journal with `SlotWrittenEvent { run: durable_run, slot, extra }` where extra was captured for a different `run_id`

**When**: The journal is recovered and `hydrate_collect_states_from_recovered_journal` is called

**Then**: Returns `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" })`

**Contract clauses**: RP1, RP2, RP3, RP5, PI3, RQ4

---

## Edge Case Tests

### test_hydrate_collect_states_from_recovered_journal_empty_events

**Given**: An empty journal event list

**When**: `hydrate_collect_states_from_recovered_journal(&[])` is called

**Then**: Returns `Ok(CollectStates::new())` — empty table

**Contract clauses**: RQ1

---

### test_hydrate_collect_states_from_recovered_journal_events_without_extra

**Given**: A journal with `SlotWrittenEvent { extra: None }` events

**When**: `hydrate_collect_states_from_recovered_journal` is called

**Then**: The events without extras are skipped; returns `Ok(CollectStates::new())`

**Contract clauses**: RP1 (no-op for non-extra events)

---

## Contract Verification Tests

### test_precondition_capture_state_preserves_run_id

**Given**: A `CollectPaginationState` with `run_id = RunId(42)` stored in `CollectStates`

**When**: `capture_state(RunId(42), slot)` is called

**Then**: The returned state's `run_id` equals `RunId(42)`

**Contract clauses**: PP1, PQ1

---

### test_precondition_capture_state_preserves_collector_slot

**Given**: A `CollectPaginationState` with `collector_slot = SlotIdx(7)` stored in `CollectStates`

**When**: `capture_state(run_id, SlotIdx(7))` is called

**Then**: The returned state's `collector_slot` equals `SlotIdx(7)`

**Contract clauses**: PP1, PQ2

---

### test_postcondition_hydrate_extra_upserts_correct_state

**Given**: A valid extra encoding `CollectPaginationState { cursor: 5, page_size: 10 }`

**When**: `hydrate_extra(run_id, slot, extra)` succeeds

**Then**: `capture_state(run_id, slot)` returns `Some(state)` with `cursor=5` and `page_size=10`

**Contract clauses**: RQ2

---

### test_invariant_identity_preserved_through_persistence_cycle

**Given**: A `CollectPaginationState` with identity `(run_id=RunId(1), collector_slot=SlotIdx(3))`

**When**: The state goes through capture → encode → decode → hydrate cycle

**Then**: The hydrated state's identity matches the original: `run_id=RunId(1)` and `collector_slot=SlotIdx(3)`

**Contract clauses**: PI1, PI2

---

### test_invariant_fresh_collect_states_on_recovery

**Given**: An existing `CollectStates` with entries for multiple runs

**When**: `hydrate_collect_states_from_recovered_journal` is called with events for only one run

**Then**: The returned `CollectStates` contains only the entries from the recovered events — no contamination from the pre-existing entries

**Contract clauses**: PI4

---

## Given-When-Then Scenario Summary

| Scenario | Given | When | Then |
|----------|-------|------|------|
| Extra round-trip | collect at cursor=1 | capture → hydrate | Hydrated cursor=1 |
| Fjall recovery | Journal with SlotWrittenEvent | Recover + hydrate | Cursor resumes at page boundary |
| Corrupt bytes | Invalid postcard bytes | hydrate_extra | InvalidCompiledWorkflow |
| Identity mismatch (direct) | Extra for RunId(1) | hydrate RunId(2) | InvalidCompiledWorkflow |
| Identity mismatch (journal) | Event for RunId(2) with RunId(1) extra | hydrate_journal_events | InvalidCompiledWorkflow |
| Empty recovery | No events | hydrate_collect_states | Empty CollectStates |
| No extra skip | SlotWrittenEvent without extra | hydrate_journal_events | No-op, continues |
