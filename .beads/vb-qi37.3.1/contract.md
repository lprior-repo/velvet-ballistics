# Contract Specification: vb-qi37.3.1 runtime: Verify collect state isolation

## 1. Scope

This bead specifies verification-only work for collect pagination state isolation in `vb_runtime`.

In scope:

- `crates/vb_runtime/src/primitives/collect.rs`
  - `CollectPaginationState`
  - `CollectStates`
  - `collect_start`
  - `collect_next`
  - `collect_finish`
  - durable extra capture/hydration helpers
- Runtime plumbing that proves collect state ownership remains per run:
  - `RunState.collect_states` in `crates/vb_runtime/src/shard/types.rs`
  - `handle_submit`, `drive_run`, and `drive_state` in `crates/vb_runtime/src/shard/lifecycle.rs`
  - `drive_deterministic_full` and collect evidence capture in `crates/vb_runtime/src/engine/drive.rs`
  - `execute_node_full` collect-node dispatch in `crates/vb_runtime/src/engine/execute.rs`

The required outcome is a set of red/green verification obligations for State 5. This State 3 artifact does not implement production code or tests.

## 2. Domain Terms

- `RunId`: unique active or recovered runtime run identity.
- `SlotIdx`: workflow slot identity. For collect, the collector slot may equal the source slot or an explicit output slot.
- `ListId`: value-store list handle. Equal numeric `ListId` values may exist in separate `ValueStore` instances and must not imply shared collect state.
- `CollectPaginationState`: durable cursor state for one collect flow.
- `CollectStates`: side table of active collect pagination states keyed by `(RunId, SlotIdx)`.
- `current_page`: the exact list currently present in the collector slot. `collect_next` must match it before advancing.
- `durable extra`: postcard-encoded `CollectPaginationState` stored on a slot-written journal/evidence event.
- `RunState`: shard-owned live state for one run, including frame, workflow, value store, action attempts, admission, and collect states.

## 3. Existing Constraints From Source

- `CollectStates.entries` is keyed by `(RunId, SlotIdx)`.
- `CollectStates::find` additionally filters by `current_page`.
- `collect_start` upserts state using `run.run_id()` and the selected collector slot.
- `collect_next` looks up state using `run.run_id()`, collector slot, and collector-slot current page.
- `collect_finish`, empty-source `collect_start`, empty-page `collect_next`, and exhausted `collect_next` remove only `(run.run_id(), collector_slot)`.
- `hydrate_extra` rejects decoded state whose embedded `run_id` or `collector_slot` differs from the journal/event identity.
- `RunState` owns `collect_states`, and shard drive passes `&mut state.collect_states` into `drive_deterministic_full`.
- `drive_deterministic_full` captures collect evidence extra from the caller-provided `collect_states`; no global collect state is allowed.

## 4. Assumptions

- The bead title says `runtime`, so primitive-only table independence is insufficient by itself. Verification must cover at least one engine or shard-level path proving normal runtime execution cannot share collect state across runs.
- Tests may use adversarially equal `SlotIdx` values and equal numeric `ListId` values to prove `RunId` participates in isolation.
- Existing identity-mismatch hydration tests are relevant, but this bead should strengthen cross-run state isolation rather than merely restate corrupt-extra behavior.
- Any added tests must comply with repository source constraints: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked casts, or unchecked arithmetic.

## 5. Open Questions

- Should State 5 place the runtime-level verification in `collect_tests.rs`, `engine/tests.rs`, or `shard/tests.rs`? Contract preference: use the smallest level that proves runtime behavior; if engine-level proves caller-provided `CollectStates`, add shard-level only if budgeted resume behavior is not otherwise covered.
- Does recovered-journal isolation across multiple runs belong to this bead or a separate storage/recovery bead? Contract preference: include hydration identity mismatch only as an acceptance guard; do not expand into broad storage recovery unless needed to prove runtime isolation.

## 6. Preconditions

### P1: Collect state identity is explicit

Any active collect state used by lookup, capture, remove, or hydrate must carry an explicit `run_id` and `collector_slot`.

### P2: Caller supplies the state table

Collect primitive and engine execution functions must receive `&mut CollectStates` from the owning runtime state or test harness. They must not acquire collect state from a static, global, thread-local, or process-wide mutable table.

### P3: Current page is authoritative for advancement

`collect_next` may advance only when the collector slot contains a list whose `ListId` equals the stored `current_page` for the same `(RunId, SlotIdx)`.

### P4: Hydration identity must match durable event identity

`hydrate_extra(run_id, collector_slot, extra)` may accept decoded state only when the decoded `run_id` and `collector_slot` equal the passed durable event identity.

### P5: Runtime run state owns collect state

Before driving a runtime run, the shard must have a `RunState` for that `RunId`, and the drive path must use that `RunState.collect_states` instance.

## 7. Postconditions

### Q1: Upsert isolation

After inserting states for two different `RunId`s using the same `SlotIdx`, both entries remain independently findable by their own `(RunId, SlotIdx, current_page)`.

### Q2: Lookup non-interference

Looking up run A must never return run B's state, even when `SlotIdx` matches and `current_page` is equal or adversarially similar.

### Q3: Capture non-interference

`capture_state(run_a, slot)` and `capture_extra(run_a, slot)` must return only run A's state or `None`; they must never serialize or expose run B's state.

### Q4: Remove non-interference

Removing `(run_a, slot)` must not remove, mutate, or make unreachable `(run_b, slot)`.

### Q5: Collect flow non-interference

When two runs perform collect pagination with the same collector slot, each run's `collect_next` advances only its own cursor and page; run A cannot advance with run B's pagination state.

### Q6: Missing or mismatched state fails closed

If a run has a non-empty collector page but no matching state for `(run_id, collector_slot, current_page)`, `collect_next` must return `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }` and must not modify another run's state.

### Q7: Hydration mismatch fails closed

Hydrating a durable extra whose embedded identity differs from the event identity must return `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" }` and must not upsert the mismatched state.

### Q8: Runtime ownership persists across budget resumes

If runtime execution exhausts step budget after creating collect pagination state, the retained `RunState` for that run must retain its own `CollectStates`. Resuming run A must not consume or mutate run B's collect state.

### Q9: Evidence extras are run-local

Collect slot-written evidence for a run must include the active collect extra captured from that run's `CollectStates` only. It must not include another run's state for the same collector slot.

## 8. Invariants

### I1: Key invariant

Every active `CollectStates` entry is uniquely addressed by `(state.run_id, state.collector_slot)`.

### I2: Embedded identity invariant

For every entry stored under key `(run_id, slot)`, `state.run_id == run_id` and `state.collector_slot == slot`.

### I3: Page-match invariant

`CollectStates::find` returns `Some(state)` only when key and `current_page` all match.

### I4: No global state invariant

Collect pagination state is owned by explicit `CollectStates` values. Runtime execution must not introduce global mutable collect state.

### I5: Per-run shard ownership invariant

Each active `RunState` owns exactly one `CollectStates` table for that run's active collect pagination progress.

### I6: Durable identity invariant

Serialized collect extras must include identity, and hydration must validate it before insertion.

### I7: Bounded resource invariant

Verification must not require unbounded loops, unbounded active runs, network access, runtime JSON/YAML/HTTP, or nondeterministic external services.

### I8: Error-as-data invariant

All fallible collect operations continue to report failure through `Result<T, EngineError>` or `RuntimeEngineResult<T>`; no panic path is acceptable.

## 9. Typed Error Taxonomy

No new production error variant is required by this bead unless implementation discovers an unrepresentable failure. Required existing errors:

- `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }`
  - Returned when `collect_next` sees a non-empty current collector page but cannot find matching `(RunId, SlotIdx, current_page)` state.
- `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" }`
  - Returned when decoded durable state identity differs from journal/event identity.
- `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" }`
  - Returned when durable extra bytes are not decodable collect state.
- `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state encode failed" }`
  - Returned if state serialization for evidence/journal extra fails.
- `EngineError::InternalInvariantViolation { reason: "collect cursor beyond source items" }`
  - Returned when a stored cursor exceeds source item bounds during `collect_next`.
- `EngineError::CollectTimeLimitExceeded`
  - Returned when collect pagination exceeds its configured time limit.
- `RuntimeEngineError::Core(EngineError::...)`
  - Engine-level wrapper for collect primitive failures.
- `RuntimeError::RunNotFound`
  - Shard-level failure when driving or resuming a run whose `RunState` is absent; not an isolation success path.

All future fallible signatures in scope must remain railway-oriented, for example:

```rust
fn upsert(&mut self, state: CollectPaginationState) -> Result<(), EngineError>;
fn capture_extra(&self, run_id: RunId, collector_slot: SlotIdx) -> Result<Option<Vec<u8>>, EngineError>;
fn hydrate_extra(&mut self, run_id: RunId, collector_slot: SlotIdx, extra: &[u8]) -> Result<(), EngineError>;
fn collect_start(...) -> Result<vb_core::EngineSignal, EngineError>;
fn collect_next(...) -> Result<vb_core::EngineSignal, EngineError>;
fn collect_finish(...) -> Result<vb_core::EngineSignal, EngineError>;
fn drive_deterministic_full(...) -> RuntimeEngineResult<RuntimeSignal>;
```

## 10. Acceptance Criteria

- AC1: Verification proves `CollectStates` can hold same-slot entries for at least two different `RunId`s without collision.
- AC2: Verification proves `remove(run_a, slot)` leaves `run_b`'s same-slot state intact.
- AC3: Verification proves `capture_state(run_a, slot)` and/or `capture_extra(run_a, slot)` cannot capture `run_b`'s same-slot state.
- AC4: Verification proves `find(run_a, slot, page)` returns `None` rather than run B's state when only run B owns that key, including an adversarial same-page case where feasible.
- AC5: Verification proves `collect_next` fails with `collect pagination state missing` rather than advancing from another run's state.
- AC6: Verification proves durable hydrate rejects embedded `run_id` mismatch and collector-slot mismatch with `collect pagination state identity mismatch`.
- AC7: Verification proves runtime/engine execution passes caller-owned `CollectStates` through collect node execution and evidence capture; no global state is used.
- AC8: If shard-level runtime tests are added, two active runs with same collector slot retain independent `RunState.collect_states` across drive/resume.
- AC9: Added verification uses existing helpers where possible and does not alter production behavior except to fix an isolation defect found by the tests.
- AC10: Later implementation state must pass canonical `moon ci` before completion.

## 11. Martin Fowler Given/When/Then Scenarios

### Scenario 1: same collector slot under different runs is independent

Given two `CollectPaginationState` values with the same `collector_slot` and different `RunId`s
And both are inserted into the same `CollectStates`
When each state is found by its own `RunId`, `collector_slot`, and `current_page`
Then each lookup returns only the state for that run
And cursor/page metadata for one run is not replaced by the other.

### Scenario 2: removing one run does not remove another run

Given run A and run B both have collect state for the same collector slot
When `remove(run_a, slot)` is called
Then `find(run_a, slot, page_a)` returns `None`
And `find(run_b, slot, page_b)` still returns run B's state.

### Scenario 3: capture is scoped by run identity

Given only run B has collect state for collector slot S
When `capture_state(run_a, S)` or `capture_extra(run_a, S)` is called
Then the result is `None`
And no bytes representing run B are returned.

### Scenario 4: adversarial equal page IDs do not cross runs

Given run A and run B use the same collector slot
And the current page `ListId` value is numerically equal or deliberately similar across isolated stores
And only run B owns collect state for `(run_b, slot)`
When run A calls `collect_next`
Then lookup for `(run_a, slot, current_page)` fails
And `collect_next` returns `InvalidCompiledWorkflow` with reason `collect pagination state missing`
And run B's state is unchanged.

### Scenario 5: two collect flows advance independently

Given two runs each start collect pagination with the same collector slot and separate `CollectStates` or separate `RunState` owners
When run A advances to the next page
Then run A's cursor/page changes
And run B's cursor/page remains unchanged
When run B advances
Then run B advances from its own prior cursor/page, not from run A's state.

### Scenario 6: durable run identity mismatch is rejected

Given a durable collect extra captured from run A
When hydration is attempted under run B's journal/event identity
Then hydration returns `InvalidCompiledWorkflow` with reason `collect pagination state identity mismatch`
And no state is inserted for run B.

### Scenario 7: durable collector-slot identity mismatch is rejected

Given a durable collect extra captured for collector slot A
When hydration is attempted under the same run but collector slot B
Then hydration returns `InvalidCompiledWorkflow` with reason `collect pagination state identity mismatch`
And no state is inserted for slot B.

### Scenario 8: runtime drive owns collect state per run

Given a shard has two active runs submitted with workflows that enter collect pagination
And both workflows use the same collector slot
When the shard drives run A to a budget boundary and later drives run B
Then run A's retained `RunState.collect_states` contains only run A's collect state
And run B's retained `RunState.collect_states` contains only run B's collect state
And resuming either run uses its own collect state.

### Scenario 9: collect evidence extra is authoritative for the active run

Given collect start writes a collector slot for run A
And another run has same-slot collect state elsewhere
When engine evidence is emitted for run A's collect slot write
Then the extra attached to run A's slot write decodes to run A and that collector slot
And it never decodes to run B.

## 12. Contract Verification Test Plan

Recommended expressive test names for State 5:

- `collect_states_remove_is_scoped_by_run_id`
- `collect_states_capture_extra_is_scoped_by_run_id`
- `collect_states_find_rejects_other_run_even_with_same_slot_and_page`
- `collect_next_rejects_other_run_state_for_same_collector_slot`
- `collect_pagination_extra_rejects_collector_slot_identity_mismatch`
- `drive_deterministic_full_uses_caller_collect_states_for_collect_evidence`
- `shard_collect_states_remain_per_run_across_budgeted_resume` if shard-level coverage is selected

Each test must assert the observable behavior, not private implementation details alone. Private `entries` inspection is acceptable in `collect_tests.rs` only as supplemental proof because existing tests already use it.

## 13. Proof Obligations

- PO1: Prove `(RunId, SlotIdx)` is the isolation boundary for table operations.
- PO2: Prove `current_page` prevents stale or mismatched page advancement.
- PO3: Prove capture/hydration preserves and validates embedded identity.
- PO4: Prove runtime execution passes explicit per-run `CollectStates` through engine and collect primitives.
- PO5: Prove no added verification uses global state, sleeps, external services, JSON/YAML/HTTP in runtime core, or unbounded resource patterns.
- PO6: Prove all failures remain typed `Result` errors, not panics.

## 14. Out-of-Scope Boundaries

- Do not redesign collect pagination.
- Do not introduce a new runtime storage mechanism.
- Do not add JSON, YAML, HTTP, or network behavior to runtime core.
- Do not introduce global/static mutable collect state.
- Do not benchmark or claim performance improvements for this bead.
- Do not broaden into full Fjall recovery semantics except for collect extra identity checks needed by isolation.
- Do not implement production code or tests in this State 3 contract artifact.

## 15. Risk Notes

- A primitive-only test can create false confidence: the bead title requires runtime isolation, so at least one engine or shard path should be verified.
- Equal `ListId` values across independent stores are adversarially useful, but tests must document that equality is intentional and not evidence of shared storage.
- Existing tests include `unwrap_or_else(|| panic!(...))`; new tests for this bead must not add forbidden panic/unwrap/expect patterns.
- Evidence extra tests must avoid asserting event order by unchecked indexing; use iteration/filtering helpers instead.
- Shard tests may be more expensive and more coupled than primitive tests. Prefer engine-level proof unless per-run `RunState` resume behavior is the explicit gap.
- Moon CI is canonical; ad-hoc cargo test success alone is not sufficient for final delivery in later states.
