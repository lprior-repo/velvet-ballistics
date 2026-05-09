# Test Plan: vb-qi37.3.1 runtime: Verify collect state isolation

## Summary

This repaired plan explicitly addresses every finding in `test-plan-review.md`: dedicated BDD scenarios exist for `upsert`, `capture_extra`, `hydrate_extra`, `collect_start`, `collect_next`, `collect_finish`, and `drive_deterministic_full`; every contract error has an exact typed oracle; vague `Some(bytes)` and “exact variant” placeholder language has been removed; unit-test density is raised to 35 named unit tests; start/finish/runtime dispatch mutants are mapped to killing tests; deterministic resource, panic, fuzz, Kani, proptest, CLI/integration, and static gates are concrete.

- Behaviors identified: 24
- Planned unit tests: 35 minimum named tests for 7 public/fallible functions (5x rule satisfied)
- Planned integration tests: 14
- Planned e2e/acceptance tests: 1 shard-level resume scenario
- Proptest invariants: 10
- Fuzz targets: 2 with 4096-byte max input and allocation ceiling oracle
- Kani harnesses: 5
- Mutation threshold: >=90% killed mutants, and 100% of critical mutants listed in Section 7 killed
- Canonical final gate: `moon ci`

## 1. Behavior Inventory

1. `CollectStates::upsert` stores a state under `(run_id, collector_slot)` when embedded identity is valid.
2. `CollectStates::upsert` replaces only the same `(run_id, collector_slot)` entry when duplicate identity is inserted.
3. `CollectStates::upsert` preserves same-slot entries when `RunId` differs.
4. `CollectStates::find` returns the exact state when `RunId`, `SlotIdx`, and `current_page` match.
5. `CollectStates::find` returns `None` when only another run owns the same slot and page.
6. `CollectStates::find` returns `None` when page differs for the same run and slot.
7. `CollectStates::remove` removes only the addressed `(RunId, SlotIdx)` entry.
8. `CollectStates::remove` is idempotent when the addressed key is absent.
9. `CollectStates::capture_state` returns the exact addressed state when present.
10. `CollectStates::capture_state` returns `None` when only foreign same-slot state exists.
11. `CollectStates::capture_extra` returns bytes that decode to the addressed run/slot/page/cursor/source/limit when present.
12. `CollectStates::capture_extra` returns `Ok(None)` when the addressed key is absent.
13. `CollectStates::capture_extra` returns `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state encode failed" }` when serialization fails under an injected encode-failure seam.
14. `CollectStates::hydrate_extra` inserts a decoded state when embedded identity matches event identity.
15. `CollectStates::hydrate_extra` overwrites only the same key when duplicate matching durable state is hydrated.
16. `CollectStates::hydrate_extra` returns `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" }` when embedded `run_id` differs.
17. `CollectStates::hydrate_extra` returns `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" }` when embedded `collector_slot` differs.
18. `CollectStates::hydrate_extra` returns `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" }` when bytes are undecodable.
19. `collect_start` creates only the active run's pagination state when source has more items than the first page.
20. `collect_start` removes only the active run's state and emits completion when source is empty.
21. `collect_next` advances only the active run's state when matching state and current page exist.
22. `collect_next` returns `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }` and preserves foreign state when local state is absent.
23. `collect_next` returns `EngineError::InternalInvariantViolation { reason: "collect cursor beyond source items" }` when stored cursor is above source length.
24. `collect_next` returns `EngineError::CollectTimeLimitExceeded` when elapsed collect time is one tick over the configured limit.
25. `collect_finish` removes only the active run's state when finishing a same-slot collect flow.
26. `drive_deterministic_full` emits collect evidence extra decoded as the active run/slot and never as a foreign run.
27. `drive_deterministic_full` wraps primitive collect failures as `RuntimeEngineError::Core(EngineError::...)`.
28. `execute_node_full` dispatches collect nodes using the caller-provided `CollectStates` table.
29. `handle_submit`, `drive_run`, and `drive_state` retain `RunState.collect_states` per run across budgeted resume.
30. Shard drive returns exactly `RuntimeError::RunNotFound` when asked to drive or resume an absent run.

## 2. Trophy Allocation

| Behavior(s) | Layer | Required tests | Rationale |
|---|---:|---:|---|
| 1-12, 14-18, 25 | Unit | 35 | Table, hydration, boundaries, and finish cleanup are deterministic calc/primitive behavior. |
| 13 | Unit with fault injection | 1 of 35 | Encode-failed is an error contract; use a test-only serialization seam or smallest existing failure hook, not production behavior changes. |
| 19-24 | Integration | 8 | `collect_start`/`collect_next` touch frame, value store, source lists, time limits, and state table. |
| 26-28 | Integration | 4 | Engine dispatch/evidence proves runtime plumbing and wrapper errors through public runtime result. |
| 29-30 | E2E acceptance/integration | 2 | Shard submit/drive/resume is the outside runtime lifecycle seam. |
| Static policy | Static | 8 gates | Repository safety rules catch panic/resource/global-state regressions cheaply. |

Density rule: at least 7 public/fallible contract functions (`upsert`, `capture_extra`, `hydrate_extra`, `collect_start`, `collect_next`, `collect_finish`, `drive_deterministic_full`) x 5 = 35 named unit tests. The exact 35-test floor appears in Section 3.1 and may be exceeded.

## 3. BDD Scenarios

### 3.1 Required 35 Unit-Test Floor

These are named unit tests; integration tests below do not count toward the 35 floor.

| # | Test function | Public behavior | Exact oracle |
|---:|---|---|---|
| 1 | `fn upsert_stores_min_cursor_state_when_identity_valid()` | `upsert` min cursor/page limit | `find(run, slot, page)` returns state with `cursor == 0`, `page_limit == 1`, exact `run_id`, `collector_slot`, `current_page`. |
| 2 | `fn upsert_stores_max_bounded_state_when_identity_valid()` | `upsert` max bounded source/page/cursor | exact max generated fields retained. |
| 3 | `fn upsert_replaces_same_run_slot_when_duplicate_key_inserted()` | duplicate upsert | second state fields replace first only for same key. |
| 4 | `fn upsert_preserves_other_run_when_same_slot_inserted()` | same-slot cross-run | both states remain exact and independently findable. |
| 5 | `fn upsert_preserves_one_below_and_one_above_neighbor_slots()` | adversarial slot neighbors | slot S lookup never returns S-1 or S+1 state. |
| 6 | `fn find_returns_exact_state_when_run_slot_page_match()` | exact lookup | returned fields equal inserted fields. |
| 7 | `fn find_returns_none_when_run_differs_even_with_same_slot_and_page()` | wrong run | `None`. |
| 8 | `fn find_returns_none_when_slot_differs_with_same_run_and_page()` | wrong slot | `None`. |
| 9 | `fn find_returns_none_when_page_differs_with_same_run_and_slot()` | wrong page | `None`. |
| 10 | `fn find_rejects_equal_numeric_list_id_from_foreign_run()` | adversarial equal page | `None` for run A, exact run B state for run B. |
| 11 | `fn remove_deletes_only_requested_run_slot()` | remove exact | run A absent; run B exact unchanged. |
| 12 | `fn remove_absent_key_is_idempotent_and_preserves_all_entries()` | absent remove | all preexisting captured states equal post-remove captures. |
| 13 | `fn remove_neighbor_run_ids_do_not_remove_target_neighbors()` | one-below/above run IDs | neighbor run states exact unchanged. |
| 14 | `fn remove_neighbor_slots_do_not_remove_target_neighbors()` | one-below/above slots | neighbor slot states exact unchanged. |
| 15 | `fn capture_state_returns_exact_state_when_present()` | capture present | `Some(state)` with exact all public fields. |
| 16 | `fn capture_state_returns_none_when_only_foreign_run_owns_slot()` | capture wrong run | `None`. |
| 17 | `fn capture_state_returns_none_when_slot_absent()` | capture absent | `None`. |
| 18 | `fn capture_state_after_duplicate_upsert_returns_replacement_state()` | duplicate capture | captured fields equal replacement. |
| 19 | `fn capture_extra_decodes_to_exact_state_when_present()` | capture bytes | bytes decoded/hydrated produce exact `run_id`, `collector_slot`, `current_page`, `cursor`, `source`, `page_limit`, and time limit. |
| 20 | `fn capture_extra_returns_ok_none_when_key_absent()` | capture absent | `Ok(None)`. |
| 21 | `fn capture_extra_does_not_serialize_foreign_same_slot_state()` | capture cross-run | hydrating captured bytes as foreign run yields identity mismatch; no foreign key inserted. |
| 22 | `fn capture_extra_returns_encode_failed_when_serialization_fails()` | encode error | `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state encode failed" })`. |
| 23 | `fn hydrate_extra_inserts_exact_state_when_identity_matches()` | hydrate happy | `Ok(())`; exact state retrievable. |
| 24 | `fn hydrate_extra_duplicate_matching_key_replaces_only_same_key()` | duplicate hydrate | same key replaced; other run exact unchanged. |
| 25 | `fn hydrate_extra_returns_identity_mismatch_when_run_id_differs()` | wrong run | exact identity mismatch error; target run absent. |
| 26 | `fn hydrate_extra_returns_identity_mismatch_when_collector_slot_differs()` | wrong slot | exact identity mismatch error; target slot absent. |
| 27 | `fn hydrate_extra_returns_decode_failed_when_bytes_empty()` | empty bytes | exact decode failed error. |
| 28 | `fn hydrate_extra_returns_decode_failed_when_bytes_single_byte()` | single byte | exact decode failed error. |
| 29 | `fn hydrate_extra_returns_decode_failed_when_bytes_truncated()` | truncated valid bytes | exact decode failed error. |
| 30 | `fn hydrate_extra_returns_decode_failed_when_bytes_max_fuzz_sized_garbage()` | 4096 garbage bytes | exact decode failed error and bounded resource use. |
| 31 | `fn collect_finish_removes_only_active_run_same_slot_state()` | finish cleanup | active run absent; same-slot foreign state exact unchanged. |
| 32 | `fn collect_finish_is_idempotent_when_active_state_absent()` | finish absent | returns expected finish signal; foreign states unchanged. |
| 33 | `fn collect_finish_preserves_neighbor_slots_for_same_run()` | finish slot scope | neighbor slots exact unchanged. |
| 34 | `fn collect_finish_preserves_neighbor_runs_for_same_slot()` | finish run scope | neighbor run IDs exact unchanged. |
| 35 | `fn collect_finish_empty_page_cleanup_removes_only_active_key()` | empty-page finish path | active key removed; foreign same-slot key exact unchanged. |

### 3.2 Dedicated BDD by Contract Function

#### Behavior: `upsert` scopes state by `(RunId, SlotIdx)`

Given: run A and run B have valid `CollectPaginationState` values with the same collector slot S, adversarial equal numeric `ListId` P, different cursors, and valid page limits.
When: both states are inserted through `CollectStates::upsert`.
Then: `find(run_a, S, P)` returns a state whose `run_id == run_a`, `collector_slot == S`, `current_page == P`, and cursor equals run A's cursor.
And: `find(run_b, S, P)` returns a state whose `run_id == run_b`, `collector_slot == S`, `current_page == P`, and cursor equals run B's cursor.

#### Behavior: `capture_extra` returns concrete run-local durable bytes

Given: run A and run B own same-slot states, and run A's state has known `source`, `current_page`, `cursor`, `page_limit`, `started_at`, and `time_limit` fields.
When: `capture_extra(run_a, S)` is called.
Then: the return is `Ok(Some(run_a_bytes))` where hydrating `run_a_bytes` into a fresh table as `(run_a, S)` yields a state with exactly run A's known fields.
And: hydrating `run_a_bytes` as `(run_b, S)` returns `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" })`.
And: no lookup for `(run_b, S, run_b_page)` exists in the fresh table.

Error variant:
Given: the encode-failure seam causes serialization of run A's state to fail.
When: `capture_extra(run_a, S)` is called.
Then: the result is exactly `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state encode failed" })`.

#### Behavior: `hydrate_extra` validates durable identity before insertion

Given: durable bytes captured from run A, slot S, page P, cursor C, source SRC, page limit L, and time limit T.
When: `hydrate_extra(run_a, S, bytes)` is called on a fresh table.
Then: the result is exactly `Ok(())` and `find(run_a, S, P)` returns a state whose public fields equal `(run_a, S, P, C, SRC, L, T)`.

Error variants:
- When the same bytes are hydrated as `(run_b, S)`, then exactly `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" })` and no run B entry exists.
- When the same bytes are hydrated as `(run_a, S2)`, then exactly `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" })` and no slot S2 entry exists.
- When undecodable bytes `[]`, `[0]`, truncated valid bytes, or 4096 bytes of `0xff` are hydrated, then exactly `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" })` and no entry is inserted.

#### Behavior: `collect_start` creates active-run pagination state only

Test function: `fn collect_start_upserts_active_run_state_when_source_exceeds_first_page()`

Given: run A and run B use collector slot S; run A's source length is `page_limit + 1`; run B already owns same-slot state; page limit is 1 for the min boundary and `MAX_TEST_PAGE_LIMIT` for the max boundary variant.
When: `collect_start` executes for run A.
Then: run A's collector slot contains the exact first page list for run A.
And: `capture_state(run_a, S)` returns run A state with `cursor == page_limit`, `current_page` equal to run A's first page, source equal to run A's source, and the configured limit fields.
And: run B's captured state equals its pre-call snapshot.

Empty-source variant:
Given: run A source length is 0 and run B owns same-slot state.
When: `collect_start` executes for run A.
Then: run A produces the existing completion/empty-source signal for the collect primitive.
And: `capture_state(run_a, S)` returns `None`.
And: run B's same-slot captured state equals the pre-call snapshot.

Boundary variants:
- source length exactly `page_limit` produces no active pagination state after first-page completion/finish behavior.
- source length `page_limit + 1` creates active state with cursor exactly `page_limit`.
- page limit zero or below-min configuration must be rejected by the existing public validation path with its exact existing error; if no such runtime input is representable, the test records that the type system makes it unrepresentable.

#### Behavior: `collect_next` advances only matching active-run state

Test function: `fn collect_next_advances_active_run_only_when_state_and_current_page_match()`

Given: run A has collector slot S containing current page P and a matching state `(run_a, S, P)` with source length `page_limit + remaining`; run B owns same-slot state with equal numeric page P in another store.
When: `collect_next` executes for run A.
Then: run A's emitted signal/value equals the expected next page for run A.
And: run A's captured cursor advances by exactly the emitted page length or is removed when exhausted.
And: run B's captured state equals its pre-call snapshot.

Error variants:
- Missing state: when only run B owns `(run_b, S, P)`, `collect_next` for run A returns exactly `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" })` and run B is unchanged.
- Cursor equal source length: when cursor equals source item count, `collect_next` takes the exhausted cleanup path, returns the exact existing completion signal, and removes only run A's state.
- Cursor one above source length: when cursor is `source_len + 1`, `collect_next` returns exactly `Err(EngineError::InternalInvariantViolation { reason: "collect cursor beyond source items" })` and no foreign state changes.
- Time exactly at limit: when elapsed time equals configured limit, the operation follows the existing allowed path and returns the exact next-page/completion signal.
- Time one tick over limit: when elapsed time is `limit + 1 tick`, the result is exactly `Err(EngineError::CollectTimeLimitExceeded)` and all foreign states are unchanged.

#### Behavior: `collect_finish` cleanup is active-run scoped

Test function: `fn collect_finish_removes_only_active_run_same_slot_state()`

Given: run A and run B own states for collector slot S; run A is ready to finish; run B has a pre-call captured state.
When: `collect_finish` executes for run A.
Then: `capture_state(run_a, S)` returns exactly `None`.
And: `capture_state(run_b, S)` returns a state equal to run B's pre-call snapshot by all public fields.
And: the returned signal equals the exact existing collect-finish signal for run A.

#### Behavior: `drive_deterministic_full` uses caller-owned state and wraps errors

Test function: `fn drive_deterministic_full_uses_caller_collect_states_for_collect_evidence()`

Given: engine run A drives a collect node for collector slot S using caller-owned `CollectStates` A, and an unrelated table B contains run B's same-slot state.
When: `drive_deterministic_full` emits slot-written evidence for run A.
Then: the collect slot evidence has one `extra` byte vector `run_a_evidence_extra` whose hydration as `(run_a, S)` returns `Ok(())` and exact run A fields.
And: hydrating `run_a_evidence_extra` as `(run_b, S)` returns exactly `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" })`.

Error wrapper variant:
Given: engine run A reaches a collect node whose primitive path returns `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }`.
When: `drive_deterministic_full` drives that node.
Then: the result is exactly `Err(RuntimeEngineError::Core(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }))`.

#### Behavior: shard resume retains per-run `RunState.collect_states`

Test function: `fn shard_collect_states_remain_per_run_across_budgeted_resume()`

Given: a shard has run A and run B submitted with workflows that enter collect pagination on slot S.
And: the deterministic step budget is exactly the smallest existing budget that stops after collect state creation and before collect completion; the test must assert the retained run status is the existing suspended/budgeted status, not completed.
When: `drive_run(run_a, budget)` then `drive_run(run_b, budget)` execute.
Then: the observation seam used by existing shard tests shows run A retained state with embedded `run_id == run_a` and run B retained state with embedded `run_id == run_b`.
When: each run resumes with the same deterministic budget ceiling.
Then: each run advances or finishes from its own state, and neither retained state is replaced by a fresh empty table.

Error variant:
Given: no `RunState` exists for `missing_run`.
When: the shard attempts to drive or resume `missing_run`.
Then: the result is exactly `Err(RuntimeError::RunNotFound)`.

## 4. Proptest Invariants

Use deterministic proptest config: `cases = 256`, `max_shrink_iters = 1024`, fixed seed through repository proptest replay config or `PROPTEST_RNG_SEED=0x5137_0301`. Generated vectors are bounded to <=8 states and source lists <=32 items.

1. **`upsert` key isolation**: for distinct run IDs and shared slot, upserting two valid states makes each findable only by its own run; wrong run returns `None`.
2. **`upsert` duplicate replacement**: for the same `(run, slot)`, second upsert replaces exactly that key; all other generated keys are unchanged.
3. **`find` boundary page matching**: `find(run, slot, page)` returns `Some` only when `page == current_page`; one-below/one-above generated pages return `None` when representable.
4. **`remove` non-interference**: removing any generated key leaves all other generated keys byte-for-byte equal via `capture_extra` and field-equal via `capture_state`.
5. **`capture_extra` round trip**: every generated valid state captured as bytes hydrates under the same identity into an exact field-equal state.
6. **`hydrate_extra` mismatch rejection**: generated valid bytes hydrated under any distinct run or slot always return identity mismatch and insert nothing.
7. **`collect_start` pagination partitioning**: for source length `0..=32` and page limit `1..=8`, empty source creates no active state; `len <= limit` finishes without active state; `len > limit` creates active state with cursor `limit` and first page length `limit`.
8. **`collect_next` foreign-state preservation**: if run A lacks local matching state and run B owns same-slot state, run A missing-state error leaves run B field-equal to pre-call snapshot.
9. **`collect_finish` scoped cleanup**: finishing a generated active key removes only that key and leaves every other run/slot field-equal.
10. **runtime/engine state ownership**: generated two-run engine fixtures with shared collector slot must produce evidence extras whose decoded identity equals the driven run and never another generated run.

## 5. Fuzz Targets

### Fuzz Target: durable collect extra hydration bytes

- Input type: `&[u8]` with hard maximum length 4096 bytes; oversized fuzz inputs are truncated by the harness before calling runtime code.
- Entry point: `CollectStates::hydrate_extra(run_id, collector_slot, bytes)`.
- Resource oracle: no panic; no hang; no allocation above 1 MiB per case as measured by the repository allocation guard if available, otherwise by the fuzz harness allocation counter; no loop over more than input length plus fixed decode overhead.
- Corpus seeds: `[]`, `[0]`, truncated valid extra at every prefix length up to 16, full valid matching extra, valid extra with wrong run, valid extra with wrong slot, 1/8/64/256/4096 bytes of `0xff`.
- Expected oracle: matching valid bytes hydrate exact fields; wrong identity returns `InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" }`; malformed bytes return `InvalidCompiledWorkflow { reason: "collect pagination state decode failed" }`.

### Fuzz Target: recovered collect evidence extras

- Input type: bounded vector length <=16 of synthetic slot-written evidence records `(RunId, SlotIdx, Option<Vec<u8 <= 4096>>)`. 
- Entry point: the existing public/crate-visible recovered evidence hydration helper, if accessible; otherwise create a follow-up bead and keep this target blocked rather than silently downgrading verification.
- Resource oracle: no sleeps, network, external services, or unbounded event scans; each case processes <=16 events and <=4096 bytes per extra.
- Corpus seeds: one matching event; two runs same slot with matching extras; run A event carrying run B extra; same run wrong slot extra; `extra: None`; corrupt extra.
- Expected oracle: only matching identity extras hydrate; mismatches return exact identity-mismatch error; corrupt extras return exact decode-failed error; no event hydrates another run's state.

## 6. Kani Harnesses

1. **Key uniqueness**: two bounded distinct run IDs, one slot, two pages; lookup by run A cannot return run B. Bound: runs 0..=1, slots 0..=1, pages 0..=2.
2. **Page-match completeness**: one stored state; candidate page equal/one-below/one-above; `Some` implies key and page equality.
3. **Remove/finish non-interference**: two entries same slot distinct runs; remove/finish one leaves the other unchanged. Bound: 2 entries.
4. **Cursor boundary classification**: source length 0..=4, cursor 0..=6; cursor == len takes exhausted path, cursor > len returns `InternalInvariantViolation { reason: "collect cursor beyond source items" }`; no unchecked index occurs.
5. **Time-limit comparison**: elapsed values `limit - 1`, `limit`, `limit + 1` for bounded integer tick model; only `limit + 1` returns `CollectTimeLimitExceeded`, killing `>`/`>=` mutants.

Kani is a pass/fail gate for harnesses added by this bead. If Kani is not installed or not wired in this workspace, State 5 must create a follow-up bead and report that formal harness execution is blocked; it must not claim this verification passed.

## 7. Mutation Testing Checkpoints

Required command after tests exist: run the repository-approved mutation command for `vb_runtime` touched paths; if no Moon wrapper exists, use `cargo mutants -p vb_runtime --minimum-test-timeout 60 --timeout 300`. Overall threshold: >=90% killed mutants. Critical mutants below must be killed 100%.

| Mutant | Killing test(s) |
|---|---|
| Remove `RunId` from `CollectStates` key | `find_returns_none_when_run_differs_even_with_same_slot_and_page`, `collect_next_returns_missing_state_when_only_other_run_owns_same_slot_and_page` |
| Key table by `SlotIdx` only | `upsert_preserves_other_run_when_same_slot_inserted` |
| Remove `current_page` filter | `find_returns_none_when_page_differs_with_same_run_and_slot`, page-match proptest |
| `remove` deletes all entries for slot | `remove_deletes_only_requested_run_slot` |
| `capture_state` ignores run | `capture_state_returns_none_when_only_foreign_run_owns_slot` |
| `capture_extra` ignores run | `capture_extra_does_not_serialize_foreign_same_slot_state` |
| Omit hydrate run validation | `hydrate_extra_returns_identity_mismatch_when_run_id_differs` |
| Omit hydrate slot validation | `hydrate_extra_returns_identity_mismatch_when_collector_slot_differs` |
| Decode error mapped to success or identity mismatch | `hydrate_extra_returns_decode_failed_when_bytes_empty`, fuzz target |
| Encode failure swallowed as `Ok(None)` | `capture_extra_returns_encode_failed_when_serialization_fails` |
| `collect_start` upserts under wrong run key | `collect_start_upserts_active_run_state_when_source_exceeds_first_page` |
| `collect_start` leaves active state for empty source | empty-source `collect_start` scenario |
| `collect_start` uses `len >= limit` instead of `len > limit` | source length exactly-limit and one-over-limit boundary tests |
| `collect_next` uses foreign same-slot state | `collect_next_returns_missing_state_when_only_other_run_owns_same_slot_and_page` |
| `collect_next` mutates foreign state on error | `collect_next_preserves_other_run_state_when_local_state_is_missing` |
| Cursor check uses `>=` instead of `>` | cursor-equals-source and one-above-source tests, Kani cursor harness |
| Time limit uses `>=` instead of `>` | exact-at-limit and one-tick-over-limit tests, Kani time harness |
| `collect_finish` removes all same-slot states | `collect_finish_removes_only_active_run_same_slot_state` |
| `collect_finish` ignores absent active key and mutates neighbors | `collect_finish_is_idempotent_when_active_state_absent` |
| `execute_node_full` dispatches collect with global/foreign table | runtime ownership proptest, `drive_deterministic_full_uses_caller_collect_states_for_collect_evidence` |
| `drive_deterministic_full` returns raw/success instead of `RuntimeEngineError::Core(...)` | wrapper error scenario |
| Evidence capture uses global/static state | engine evidence test and hydrate-as-foreign mismatch assertion |
| `handle_submit` fails to initialize per-run `CollectStates` | shard budget resume test |
| `drive_run` drops retained `RunState.collect_states` | shard budget resume test |
| `drive_state` resumes with fresh table | shard budget resume test |
| Missing run maps to wrong error | exact `RuntimeError::RunNotFound` scenario |

## 8. Boundary Tables Per Public Function

| Function | Min valid | Max valid | Empty/zero/None | One-below-min | One-above-max | Overflow/underflow | Exact-at-limit | One-over-limit |
|---|---|---|---|---|---|---|---|---|
| `upsert` | cursor 0, page limit 1 | source/page/cursor at bounded test max | no preexisting entries | slot/run neighbor below when representable | slot/run neighbor above when representable | generated IDs use checked constructors only | n/a | n/a |
| `capture_extra` | one minimal state | max bounded state | absent key -> `Ok(None)` | neighbor run/slot absent | neighbor run/slot absent | encode-failure seam -> encode failed | n/a | n/a |
| `hydrate_extra` | minimal valid bytes | valid bytes <=4096 | empty bytes -> decode failed | wrong lower neighbor run/slot -> identity mismatch | wrong upper neighbor run/slot -> identity mismatch | corrupt 4096 bytes -> decode failed | n/a | n/a |
| `collect_start` | source len 1, limit 1 | source len 32, limit 8 | source len 0 -> no active state | page limit 0 rejected or unrepresentable | source len limit+1 creates state | checked length arithmetic only | source len == limit -> no active pagination | source len == limit+1 -> active state |
| `collect_next` | cursor 0 with non-empty current page | cursor/source at bounded max | missing state -> missing error | cursor/source underflow unrepresentable via constructors | cursor source_len+1 -> invariant violation | no unchecked index/slice | elapsed == limit allowed | elapsed == limit+1 -> time limit exceeded |
| `collect_finish` | one active state | bounded table of 8 states | absent active key idempotent | neighbor run/slot preserved | neighbor run/slot preserved | checked key construction only | n/a | n/a |
| `drive_deterministic_full` | one collect node, one run | two runs, <=16 evidence events | primitive missing state -> wrapped error | wrong neighbor run cannot decode evidence | wrong neighbor slot cannot decode evidence | bounded steps; no unchecked event indexing | budget exactly stops after collect state creation | budget resume uses retained state |

## 9. Static, Resource, CLI, and Integration Gates

State 5 must report these commands/results; targeted gates supplement but never replace `moon ci`.

1. Canonical CI: `moon ci` must pass.
2. Targeted runtime tests: run the exact new test names for `vb_runtime` using the repo's Moon test target if present; otherwise `cargo test -p vb_runtime <test_name>` for each new test group. Output must show test names and pass status.
3. Proptest: run with `PROPTEST_CASES=256 PROPTEST_RNG_SEED=0x51370301` and no ignored failures.
4. Fuzz: fuzz targets must compile; each target must run a deterministic smoke of the seed corpus with max input 4096 bytes. If fuzz infra is unavailable, create a follow-up bead and do not mark fuzz verification complete.
5. Kani: added harnesses must pass when Kani is available; if unavailable, create a follow-up bead and do not claim formal proof.
6. Mutation: report `cargo mutants` or repo wrapper summary showing >=90% kill rate and every critical mutant killed.
7. Panic/forbidden pattern audit over new diff: fail on `unsafe`, `.unwrap(`, `.expect(`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `thread::sleep`, unchecked `[` indexing in new assertions, unchecked slicing, `as` casts, discarded `Result` via `let _ =`, and `static mut`/global mutable state.
8. Resource cleanup: shard/engine tests must use temporary stores/work directories from existing test fixtures; cleanup is RAII/drop-based; no network, no sleeps, no wall-clock waits, no external services, no JSON/YAML/HTTP in runtime core, max two active runs, max 16 evidence events, max 32 source items, max 4096 extra bytes.

## 10. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| upsert same slot different runs | valid A/B states | each exact state retrievable by own run | unit |
| upsert duplicate | same key twice | replacement exact, others unchanged | unit |
| find wrong run | same slot/page, different run | `None` | unit |
| find wrong page | same run/slot, different page | `None` | unit/proptest |
| remove active key | A/B same slot | A absent, B exact unchanged | unit |
| remove absent key | no active key | all existing states unchanged | unit |
| capture_state present | addressed key present | exact `Some(state)` fields | unit |
| capture_state foreign only | only other run owns slot | `None` | unit |
| capture_extra present | addressed key present | `Ok(Some(bytes))`; bytes hydrate to exact known fields | unit/integration |
| capture_extra absent | addressed key absent | `Ok(None)` | unit |
| capture_extra encode failure | serialization seam fails | `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state encode failed" })` | unit |
| hydrate matching | valid matching bytes | `Ok(())`; exact state retrievable | unit/integration |
| hydrate wrong run | valid bytes, wrong run | `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" })` | unit |
| hydrate wrong slot | valid bytes, wrong slot | same exact identity mismatch | unit |
| hydrate corrupt | invalid bytes <=4096 | `Err(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state decode failed" })` | unit/fuzz |
| collect_start empty | source len 0 | no active state; foreign unchanged | integration |
| collect_start exact limit | source len == page_limit | no active pagination state | integration/proptest |
| collect_start one over | source len == page_limit + 1 | active state cursor == page_limit | integration/proptest |
| collect_next happy | matching local state/page | exact next page/cursor; foreign unchanged | integration |
| collect_next missing | only foreign same-slot state | exact missing-state error; foreign unchanged | integration |
| collect_next cursor equal len | exhausted boundary | exact completion path; active removed only | integration/Kani |
| collect_next cursor one above len | invalid cursor | exact internal invariant violation | integration/Kani |
| collect_next exact time limit | elapsed == limit | allowed exact normal output | integration/Kani |
| collect_next one over time | elapsed == limit + 1 | `Err(EngineError::CollectTimeLimitExceeded)` | integration/Kani |
| collect_finish active | A/B same slot | A removed, B exact unchanged | unit/integration |
| engine evidence | caller table A plus foreign table B | evidence extra hydrates as A, identity-mismatches as B | integration |
| engine wrapper | primitive collect failure | `Err(RuntimeEngineError::Core(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }))` | integration |
| shard resume | two retained runs budgeted | each retained/resumed state has own run identity | e2e/integration |
| missing shard run | absent run | `Err(RuntimeError::RunNotFound)` | integration |
| static audit | new diff | no forbidden patterns/resources/globals/discarded errors | static |
| mutation quality | touched paths | >=90% mutants killed; critical mutants all killed | mutation |

## Open Questions

1. The encode-failure scenario requires an existing or test-only serialization failure seam. If no such seam exists without production design change, State 5 must create a follow-up bead for an injectable serializer and still cover all other errors now.
2. If recovered evidence hydration is not publicly/crate-visibly accessible, the second fuzz target is blocked and must become a follow-up bead rather than being counted as completed.
