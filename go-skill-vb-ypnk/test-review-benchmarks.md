# Test Plan Review: test-plan-benchmarks.md

**Bead**: vb-qi37-4-bench
**Mode**: 1 — Plan Inquisition
**Document**: `test-plan-benchmarks.md` (721 lines)
**Verdict**: **REJECTED**

---

## Summary

12 missing benchmark group specifications are proposed. Coverage of the 12 surface areas is complete. However, **multiple LETHAL and MAJOR findings** block approval.

---

## Axis 1 — Contract Parity

**LETHAL FINDING — Arithmetic Inconsistency (blocks verification)**

The document header states:
- "Section 39 mandates 22 benchmark groups. 12 are MISSING."

But `criterion_group!` at `benches/velvet_ballastics.rs:2695` registers **17 existing** benchmark groups:
```
parse_yaml_benches, compile_and_validate_benches, expression_benches,
slot_and_transition_benches, storage_and_ipc_benches, generated_benches,
ir_vs_generated_benches, taint_scalar_expr_bench, taint_slot_loading_bench,
taint_build_object_bench, taint_build_list_bench, taint_full_workflow_bench,
submit_artifact_benches, budget_compute_benches, evidence_chain_benches,
admission_gate_benches, capability_check_benches
```

The plan then proposes adding **12 new groups** (ir_traversal through rtrb), which would yield **29 total** — not 22. The claim "12 are MISSING" implies 10 currently exist, which contradicts the 17 in `criterion_group!`. The source of the "22" mandate is Section 39 of the master plan, not verified here. The mismatch means the scope of "missing" is unresolvable without clarification from the authoritative Section 39 text.

---

## Axis 2 — Assertion Sharpness

Every "Then:" clause was inspected. The following benchmarks use **non-quantitative pass criteria**:

**LETHAL per-mode-definition (no measurable pass threshold)**:

| Benchmark | Assertion | Problem |
|-----------|-----------|---------|
| `IR_traversal` (depth-first) | "Returns total node count with no panic" | No panic is not a measurement; no threshold |
| `IR_traversal` (BFS) | "Returns nodes in correct topological order" | "correct" is undefined; no value asserted |
| `IR_traversal` (expr) | "All ops are visited with correct ordering" | "correct" is undefined |
| `collect_page` (first/second) | "current_page is populated, no error" | "populated" is not a value; no assertion on page contents |
| `collect_page` (exhausted) | "Returns materialization complete signal, state is removed" | No value for "complete signal" |
| `collect_page` (exceeded) | "Returns only remaining items up to limit" | No exact count asserted |
| `pagination_cost` (insert/upsert/find) | "State is stored and retrievable" / "Previous page recorded" / "O(1) amortized" | "retrievable" and "recorded" are booleans without values; O(1) is a growth rate claim, not a benchmark assertion |
| `pagination_cost` (find missing) | "Returns None without error" | `is_none()` = LETHAL per Mode 1 rules |
| `timer_wheel_tick` (fire empty) | "Returns empty Vec, no allocation" | No concrete Vec length or allocation byte threshold |
| `timer_wheel_tick` (fire 1/10/90) | "Returns the 1 timer entry" / "Returns 10 entries in deadline order" / "Returns exactly 90 expired entries" | Entry values not specified |
| `action_queuing` (enqueue) | "Returns Ok(()), queue len increases by 1" | Concrete `len` value not asserted |
| `action_queuing` (dequeue) | "Returns Some(command), queue len decreases by 1, FIFO order preserved" | No concrete value for len; "FIFO order" is qualitative |
| `snapshot_save` (small/50-step) | "Returns a FrameStateSnapshot with all slots and PC" | PC value not specified (e.g., PC=1, PC=50) |
| `snapshot_save` (large slots) | "Serialized snapshot includes all 10KB of data" | No exact byte count asserted |
| `snapshot_save` (correlation) | "Snapshot preserves correlation in output" | No concrete correlation value asserted |
| `snapshot_restore` (all variants) | "PC=X, executed=X, slot values restored" | PC and executed values named but not cross-checked against expected |
| `memory_footprint` (small) | "Peak heap bytes allocated is within documented bound" | Bound is "documented" but no value in the plan; if no such bound exists, this is not a benchmark |
| `memory_footprint` (save chain) | "Peak heap bytes scales linearly with slot count × 1000" | A growth-rate claim is not a pass/fail benchmark assertion |
| `cold_start` (1000-step) | "Frame is created with 1001 nodes initialized" | Node count = input + 1 is a specific claim, but no timing threshold |
| `cold_start` (full pipeline) | "Complete cold-start latency is measured" | No budget or threshold |
| `cold_start` (concurrent) | "All frames are created without contention errors" | Not a quantitative throughput assertion |

**Concrete pass criteria found (well-specified)**:

| Benchmark | Assertion | Assessment |
|-----------|-----------|------------|
| `ArrayQueue` push-full | "Returns Err(second_item) — item NOT lost, queue unchanged" | **Precise** — exact return value, item preservation semantics |
| `rtrb` push-full | "Returns Err(item) — item NOT lost, buffer unchanged" | **Precise** — exact return value, item preservation semantics |
| `ArrayQueue` capacity boundary | "1024 items pushed; Err on 1025th" | **Precise** — exact count + error condition |
| `ArrayQueue` is_full/len | "is_full()==false, len()==512" | **Precise** — concrete values |
| `rtrb` is_full/is_empty | "is_full()==false, is_empty()==false" | **Precise** — concrete values |
| `action_dispatch` (unknown) | "Returns ActionError::UnknownAction" | **Precise** — exact error variant |
| `action_dispatch` (mismatched) | "Returns ActionError::DispatchFailed" | **Precise** — exact error variant |

---

## Axis 3 — Trophy Allocation

**LETHAL — `memory_footprint` is not a Criterion benchmark**

The plan explicitly states (line 188):
> "This benchmark records memory via `memory_stats()` or `tracemalloc`. It is not a `Criterion` throughput benchmark — it reports peak RSS."

The metadata format string requires `tool=criterion-0.8`. Using tracemalloc/memfd does not produce Criterion-measurable output. The open question (OQ-1, line 711) acknowledges this is unresolved. **No benchmark can be approved when its measurement tool is undecided and the plan explicitly states it does not use the mandated tool.**

**MAJOR — `cold_start` 10-concurrent benchmark measures wrong thing**

Line 226–228: "10 `new_run_frame` calls are made concurrently. Then: All frames are created without contention errors."

This measures multi-threaded **throughput**, not single-threaded **latency**. Cold-start latency is a per-run metric; concurrent throughput is a different benchmark. The open question (OQ-2, line 713) acknowledges this. **Until resolved, this benchmark group does not measure what its name claims.**

---

## Axis 4 — Boundary Completeness

Most benchmarks define reasonable input boundaries (small/medium/large, empty/1/100/1024). The fixture strategy table is present and bounded.

**MINOR — `snapshot_restore` missing boundary: corrupted snapshot**

No scenario tests restoring from a snapshot with corrupted/partial bytes. This is a boundary case that could panic or misbehave.

**MINOR — `ArrayQueue` missing boundary: zero-capacity queue**

`ArrayQueue::<T>::new(capacity=0)` — does this panic or return an error? Not covered.

**MINOR — `rtrb` missing boundary: zero-capacity buffer**

Same — const generic `N=0` may not compile or may be a special case.

---

## Axis 5 — Mutation Survivability

Not applicable — benchmarks are not mutation-tested in the traditional sense. A timing measurement is inherently resistant to the class of mutations that flip boolean results.

---

## Axis 6 — Evidence Plan Audit

**Preconditions are stated** in each "Given:" block. ✓

**Bounded reproducible inputs** — fixture helpers (`save_chain_workflow(n)`, `timer_wheel_with_n_expired(n)`) produce deterministic fixtures. ✓

**Side effects named** — benchmarks create frames, enqueue commands, insert timers; cleanup is not needed for in-process measurements. ✓

** Holzmann Rule 5 (State Your Assumptions)** — "Given" blocks are explicit. ✓

** Holzmann Rule 2 (Bound Generated Coverage)** — All loop bounds and fixture sizes are explicit constants. ✓

---

## LETHAL FINDINGS (any single = REJECTED)

1. **`memory_footprint` is not a Criterion benchmark** — explicitly self-describes as non-Criterion, yet metadata requires `tool=criterion-0.8`. Open question OQ-1 unresolved.
2. **`pagination_cost` find-missing** — `Returns None without error` = `is_none()` assertion = LETHAL per Axis 2.
3. **Scope arithmetic unresolvable** — 17 existing + 12 proposed = 29, not 22. The claim "12 MISSING" is anchored to a "22 total" mandate from Section 39 that cannot be verified from contract alone.

---

## MAJOR FINDINGS (3 = automatic rejection)

1. **~16 of 17 benchmark assertions are non-quantitative** — "no panic", "correct order", "within bound", "populated", "preserved", "scales linearly" — none specify exact expected values or pass thresholds. Benchmarks that cannot fail cannot catch regressions.
2. **`cold_start` concurrent benchmark measures throughput not latency** — name claims "cold start latency" but the concurrent scenario measures multi-threaded frame-creation throughput.
3. **`collect_page` fixture dependency unresolved** — OQ-3 (line 715): if `collect_page` requires full runtime execution through `execute.rs` rather than being a standalone function, the benchmark group may need to be redesigned as an integration benchmark rather than a unit benchmark.

---

## OPEN QUESTIONS (unresolved by the plan — block approval)

| # | Question | Blocks |
|---|----------|--------|
| OQ-1 | memory_footprint: tracemalloc vs memfd vs Criterion? | `memory_footprint` entire group |
| OQ-2 | cold_start: single-threaded latency or multi-threaded throughput? | `cold_start` concurrent sub-benchmark |
| OQ-3 | collect_page: standalone function or requires full runtime execution? | `collect_page` entire group |
| OQ-4 | snapshot_restore: actual API surface (`try_from_snapshot()` or shard path)? | `snapshot_restore` entire group |
| OQ-5 | ArrayQueue push-on-full: does it return the item or drop it? | `ArrayQueue` error semantics |
| OQ-6 | rtrb capacity: N=128 vs N=1024? | `rtrb` group fixture sizing |

---

## MANDATE

The following must exist before resubmission:

1. **Fix the arithmetic**: Clarify whether the Section 39 mandate is 22 or 29 total benchmark groups. If 22, identify which 5 of the 17 existing `criterion_group!` entries are NOT Section 39 benchmarks. If 29, update the document header.
2. **Resolve OQ-1**: Decide whether `memory_footprint` uses Criterion (with a proxy memory-proxy metric) or is separated into a distinct `divan`/`iai` binary. Do not ship a benchmark that self-reports as non-Criterion under a `tool=criterion-0.8` metadata string.
3. **Resolve OQ-2**: Rename the concurrent cold-start benchmark to `cold_start_throughput` or remove it. "Cold start latency" and "concurrent cold starts" are different measurements.
4. **Resolve OQ-3**: Confirm `collect_page` API surface. If it requires `run_until_blocked` through a collect workflow, the benchmark must be structured as an integration benchmark.
5. **Resolve OQ-4**: Confirm `snapshot_restore` hydration API. If not implemented, the benchmark cannot be written.
6. **Convert all assertions to exact values**: Every "Then:" must be changed from qualitative ("correct", "populated", "preserved") to quantitative (`PC == 50`, `len() == 50`, `utilization_pct <= 80`). Benchmark assertions that cannot fail cannot catch regressions.
7. **Fix `pagination_cost` find-missing**: Replace `Returns None without error` with an assertion on the exact `Option` variant.

**All 7 items must be resolved. Resubmit for full re-review from Axis 1.**
