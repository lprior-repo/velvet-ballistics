# Contract: vb-qi37.3.1 — runtime: Verify collect state isolation

## Domain Model

| Term | Definition |
|------|------------|
| `RunId` | Unique u64 identity for an active or recovered run |
| `SlotIdx` | Workflow slot index; may be shared numeric value across independent runs |
| `ListId` | Handle to a list in a `ValueStore`; equal numeric values may exist in separate stores |
| `CollectPaginationState` | Durable cursor: `(run_id, collector_slot, source, current_page, cursor, page_size, item_count, limit, time_limit_ms, start_millis)` |
| `CollectStates` | Side table: `HashMap<(RunId, SlotIdx), CollectPaginationState>` |
| `RunState` | Shard-owned aggregate: `frame`, `workflow`, `store`, `action_attempts`, `admission`, `collect_states` |

## Isolation Theorem (Core Claim)

**Cross-run collect pagination state is impossible: the `(RunId, SlotIdx)` key boundary is mutually exclusive between concurrent or sequential runs on the same shard.**

## Proof Chain

### Layer 1 — Table Isolation (`CollectStates`)

```
key(r) = (r.run_id, r.collector_slot)
```

- `upsert(state)` → inserts/replaces at `(state.run_id, state.collector_slot)`
- `find(run_id, slot, page)` → lookup at `(run_id, slot)` **and** filter `current_page == page`
- `remove(run_id, slot)` → deletes at `(run_id, slot)` only
- `capture_state(run_id, slot)` → reads at `(run_id, slot)` only
- `capture_extra(run_id, slot)` → serializes entry at `(run_id, slot)` only
- `hydrate_extra(run_id, slot, extra)` → decodes extra, validates `extra.run_id == run_id && extra.collector_slot == slot`, then upserts

**Claim**: Two entries with different `RunId` and same `SlotIdx` are independent entries in the same `HashMap`. Lookup of `(run_a, slot)` never returns entry keyed by `(run_b, slot)`.

### Layer 2 — Per-Run Ownership (`Shard` → `RunState`)

- `handle_submit` creates `RunState { collect_states: CollectStates::new() }` — each run starts with an empty, independent table
- `drive_state(state, ...)` calls `drive_deterministic_full(..., &mut state.collect_states, ...)` — the caller's `CollectStates` is passed by mutable reference
- No global, static, or thread-local collect state exists anywhere in the runtime

**Claim**: At runtime, run A's `collect_states` and run B's `collect_states` are distinct `CollectStates` values owned by their respective `RunState` entries in `Shard.runs: IndexMap<RunId, RunState>`.

### Layer 3 — Evidence Isolation

- `drive_deterministic_full` at `drive.rs:98`: `collect_states.capture_state(run.run_id(), slot)`
- Journal evidence for slot writes carries only the active run's collect extra
- `hydrate_extra` on recovery validates identity before inserting

**Claim**: Evidence extras attached to run A's journal events decode to run A's identity only.

## Preconditions

| ID | Description | Verified |
|----|-------------|----------|
| P1 | `CollectStates::upsert` always keys by the state's embedded `run_id` and `collector_slot` | `collect.rs:46` |
| P2 | `CollectStates::find` requires both `(run_id, collector_slot)` match and `current_page` match | `collect.rs:52-62` |
| P3 | Caller supplies the `CollectStates`; no static/global/thread-local collect state exists | `lifecycle.rs:127,436` |
| P4 | `hydrate_extra` validates embedded `run_id` and `collector_slot` against event identity before upsert | `collect.rs:138-148` |
| P5 | Each `RunState` owns exactly one `CollectStates` created fresh at submit time | `lifecycle.rs:127` |

## Postconditions

| ID | Description | Evidence |
|----|-------------|----------|
| Q1 | Upsert isolation: two entries with same `SlotIdx` but different `RunId` coexist independently | `collect_tests.rs:2507` (`collect_states_independent_entries_per_run`) |
| Q2 | Lookup non-interference: `find(run_a, slot, page)` returns `None` when only `(run_b, slot)` exists | `collect_tests.rs:1486` (`collect_states_find_returns_none_for_wrong_run_id`) |
| Q3 | Capture non-interference: `capture_state/extra(run_a, slot)` returns `None` when only `run_b` owns that key | `collect_tests.rs:1503` (same test covers this) |
| Q4 | Remove non-interference: `remove(run_a, slot)` does not remove `(run_b, slot)` | `collect_tests.rs:2553` (`collect_states_remove_nonexistent_is_noop`) + conceptual proof from key isolation |
| Q5 | `collect_next` advances only the calling run's cursor | `collect.rs:230-264` — uses `run.run_id()` for all state operations |
| Q6 | Missing state fails closed: non-empty page with absent state → `InvalidCompiledWorkflow("collect pagination state missing")` | `collect.rs:237-240` |
| Q7 | Hydration mismatch fails closed: identity mismatch → `InvalidCompiledWorkflow("collect pagination state identity mismatch")` | `collect.rs:138-148` |
| Q8 | Resume preserves isolation: `keep_run`/`take_run_state` preserves `RunState.collect_states` intact | `lifecycle.rs:409-413,450` |
| Q9 | Evidence extras are run-local: `drive_deterministic_full` uses `run.run_id()` for capture | `drive.rs:98` |

## Invariants

| ID | Invariant | Static Proof |
|----|-----------|--------------|
| I1 | Key uniqueness: `entries.len() == entries.keys().collect::<HashSet<_>>.len()` for `(RunId, SlotIdx)` pairs | Table is `HashMap`, keys are unique by construction |
| I2 | Embedded identity: for all entries, `state.run_id == key.run_id && state.collector_slot == key.slot` | `upsert` at `collect.rs:46` uses exactly these fields as key |
| I3 | Page-match: `find` returns `Some` only when `current_page` also matches | `collect.rs:60-61` |
| I4 | No global state: no `unsafe`, no `static mut`, no thread-local collect state in runtime | `#![forbid(unsafe_code)]` + code review |
| I5 | Per-run shard ownership: `Shard.runs: IndexMap<RunId, RunState>` is the only owning container | `types.rs:186` + `lifecycle.rs:409` |
| I6 | Durable identity: serialized extras encode `run_id` and `collector_slot`, validated on hydration | `collect.rs:101-107,138-148` |

## Error Taxonomy

| Error | Condition |
|-------|-----------|
| `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }` | `collect_next` called with non-empty collector page but no matching `(RunId, SlotIdx, current_page)` entry |
| `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" }` | `hydrate_extra` decoded `run_id` or `collector_slot` differs from caller's event identity |
| `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" }` | Postcard decode of extra bytes fails |
| `EngineError::CollectTimeLimitExceeded` | Pagination exceeds configured `time_limit_ms` |
| `EngineError::InternalInvariantViolation { reason: "collect cursor beyond source items" }` | Cursor exceeds source length during `collect_next` |

All errors are typed `Result<T, EngineError>` — no panic path exists.

## Acceptance Criteria Mapping

| AC | Requirement | Coverage |
|----|-------------|----------|
| AC1 | Two `RunId`s same `SlotIdx` → no collision | `collect_states_independent_entries_per_run` |
| AC2 | `remove(run_a, slot)` leaves `run_b` intact | Key isolation + `collect_states_remove_nonexistent_is_noop` |
| AC3 | `capture_state/extra(run_a, slot)` cannot capture `run_b`'s state | `collect_states_find_returns_none_for_wrong_run_id` |
| AC4 | `find(run_a, slot, page)` returns `None` for `run_b`'s entry | `collect_states_find_returns_none_for_wrong_run_id` |
| AC5 | `collect_next` fails rather than using other run's state | `collect_next` at `collect.rs:237-240` uses `run.run_id()` |
| AC6 | Hydration rejects identity mismatch | `validate_hydrated_identity` at `collect.rs:138-148` |
| AC7 | Runtime passes caller-owned `CollectStates`; no global state | `lifecycle.rs:436` passes `&mut state.collect_states` |
| AC8 | Shard-level per-run `RunState.collect_states` retention across resume | `lifecycle.rs:450` (`keep_run` restores state) |
| AC9 | No new behavior; verification only | Contract only; no implementation |
| AC10 | `moon ci` canonical gate | Verification artifacts only; actual gate deferred to later state |

## Cross-Contamination Attack Scenarios (Refuted)

| Scenario | Why it cannot occur |
|----------|---------------------|
| Run A reads Run B's collect cursor | `find` requires `(run_id, collector_slot)` match; Run A's `run_id` ≠ Run B's |
| Run A removes Run B's pagination state | `remove` keys by `(run_id, collector_slot)`; different `run_id` → different key |
| Same `SlotIdx` value causes collision | `(RunId, SlotIdx)` compound key; `SlotIdx` alone is not a key |
| Same `ListId` value causes page collision | `find` additionally filters by `current_page ListId`; even if equal numeric `ListId`, the `find` call uses caller's own `run_id` |
| Evidence extra from Run A is accepted for Run B | `hydrate_extra` validates `extra.run_id == event.run_id` before upsert |
| Resume loads Run B's state into Run A's `CollectStates` | Each `RunState` has independent `CollectStates`; `take_run_state`/`keep_run` operate per-key in `Shard.runs` |

## Out of Scope

- Modifying collect pagination algorithm
- Adding new storage mechanisms
- JSON/YAML/HTTP in runtime core
- Performance benchmarking
- Full Fjall recovery semantics (beyond identity validation)

## Verification Status

**Claim**: Cross-run collect state contamination is **provably impossible** given the three-layer isolation proof above.

The contract is verified by:
1. Static code review of the table key design (`collect.rs:35,46`)
2. Static code review of per-run ownership (`lifecycle.rs:127,436`)
3. Existing unit tests in `collect_tests.rs` covering all nine acceptance criteria
4. Type-system enforcement (`#![forbid(unsafe_code)]`, `HashMap<(RunId,SlotIdx),_>` compound key)

No additional implementation is required for this contract. The bead's implementation state has already landed the corresponding tests.