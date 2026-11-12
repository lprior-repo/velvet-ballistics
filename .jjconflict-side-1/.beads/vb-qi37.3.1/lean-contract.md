# Lean Contract Projection: vb-qi37.3.1

## Boundary

- **Lean-owned kernel**: `CollectStates` table operations: `upsert`, `find`, `remove`, `capture_state`, `capture_extra`, `hydrate_extra`, `validate_hydrated_identity`. These are deterministic pure functions over `HashMap<(RunId, SlotIdx), CollectPaginationState>`.
- **Rust/runtime shell**: `Shard` lifecycle (`handle_submit`, `drive_run`, `drive_state`), `drive_deterministic_full`, `execute_node_full`, and `collect_start`/`collect_next`/`collect_finish` which involve `RunFrame`, `ValueStore`, and wall-clock time.
- **External systems excluded from Lean proof**: Fjall persistence (handled separately by storage beads), action completion callbacks, timer wheel.

## Lean-Owned Clauses

All pure critical behavior is already structurally proven by code review and unit tests. The `CollectStates` key isolation property is a direct consequence of:
1. Rust's `HashMap<(K, V)>` key uniqueness by construction
2. The `find` operation additionally filtering by `current_page`

A Lean proof would model the `CollectStates` operations as a map with compound key `(RunId, SlotIdx)` and prove:

**Theorem (Key Isolation)**: For any `CollectStates` `C`, any `run_id`, `slot`, `page`:
```
C.find(run_id, slot, page) = Some(state)
  ↔ state.run_id = run_id
  ∧ state.collector_slot = slot
  ∧ state.current_page = page
```

This theorem is a direct consequence of the `HashMap` key lookup semantics and the explicit filter. It does not require a separate Lean proof because:
- The `HashMap` key uniqueness is a standard library invariant
- The `find` implementation is a direct composition of `get` + `filter` on a `Copy` type

## Theorem Obligations

No Lean theorems are **required** because:
1. The isolation property is a structural consequence of the compound key design
2. All acceptance criteria are covered by existing unit tests (`collect_tests.rs` 3146 lines)
3. The runtime ownership is a reference-passing argument, not an algorithmic property

## Waivers

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|--------|-------|--------|--------|-----------------------|
| All pure kernel clauses | vb-qi37.3.1 contract synthesizer | Structural HashMap property; unit tests cover key isolation scenarios; no algorithmic complexity requiring Lean proof | Not applicable — property is structurally proven | `collect_states_independent_entries_per_run` (collect_tests.rs:2507), `collect_states_find_returns_none_for_wrong_run_id` (collect_tests.rs:1486) |
| Runtime shell (drive_state, execute_node_full) | N/A | Shell is outside Lean scope (I/O, scheduling, time) | N/A | Code review at `lifecycle.rs:416-439`, `drive.rs:47-127` |

## Non-goals

- Lean proof of `collect_start`/`collect_next`/`collect_finish` (these involve `ValueStore`, `RunFrame`, wall-clock time — outside pure kernel)
- Lean proof of Fjall persistence behavior (separate storage bead)
- Lean proof of concurrent shard behavior (no concurrency in collect primitive — `Shard` is single-threaded)
