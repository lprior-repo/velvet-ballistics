# eval_append O(n²) cumulative fix — design scoping

**Bead**: vb-jf1c1 (design only — no production code change)
**Follow-up bead**: vb-jim32 (implementation)
**Date**: 2026-06-20

## Problem statement

`eval_append` in `crates/vb_core/src/engine/expr_eval/ops_text_list.rs:166` clones the entire list on every single append because lists are immutable arena values (`Box<[SlotValue]>`):

```rust
pub(crate) fn eval_append(stack, store) -> Result<(), EngineError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list(list)?;
    let items = store.list(list_id).map_err(...)?;
    let mut new_items: Vec<SlotValue> = items.to_vec();   // full clone
    new_items.push(item);
    let new_list = store.insert_list(new_items.into_boxed_slice()).map_err(...)?;
    push_value(stack, SlotValue::List(new_list))
}
```

For agent fan-out loops doing N appends, this is O(n²) cumulative.

**Measured gap** (from `crates/vb_core/benches/expr_eval_micro.rs`):
- Single `eval_append` at N=65536: 695ms cumulative
- Pre-built `Vec` then `insert_list`: 138μs
- **Speedup opportunity: 5036×**

## Constraint (non-negotiable)

The runtime value model is **immutable-arena**: every list is a `ListId` handle into an append-only arena of `Box<[SlotValue]>` slices. Recovery/replay determinism depends on stable list identity — the journal records `ListId` not the list contents, and replay must rebuild the same `ListId` for the same sequence of insert operations.

Any design that:
- mutates a `Box<[SlotValue]>` in place after arena allocation, OR
- introduces a non-deterministic iteration order, OR
- changes the identity (`ListId`) of a list that has already been journaled,
  **rejects the option outright**.

## Options considered

### Option A: Persistent vector (`im::Vector`, `rpds::Vector`, `imbl::Vector`)

**Idea**: Replace `Box<[SlotValue]>` with a persistent vector that supports O(log n) or O(1) amortized append via structural sharing. The `ValueStore` would store `Box<im::Vector<SlotValue>>` instead of `Box<[SlotValue]>`.

**Pros**:
- O(log n) amortized append — directly fixes the O(n²) pathology.
- No semantic change to the user-visible workflow IR (still `ExprOp::Append`).
- `im::Vector` and `rpds::Vector` are already implicit in the workspace as workspace dependencies in some crates.

**Cons**:
- Adds a new workspace dependency (currently the `Cargo.toml` workspace dep list does not include `im` or `rpds`).
- Persistent vectors allocate ~3-5× more than `Box<[T]>` due to trie branching (32-way for `im`, 64-way for `rpds`).
- Persistence overhead is wasted for the common case where a list is built once and then read many times.
- Replay determinism depends on `im`/`rpds` HashMap-style iteration being canonical across versions; this is generally true but adds a new trust dependency.

**Allocation profile**: worst case ~5× a single `Box<[T]>`; amortized ~2× per append due to trie node reuse.

**Verdict**: viable but expensive; rejected as the primary path because the second option below does the job with zero new deps.

### Option B: Loop-aware builder/accumulator node (RECOMMENDED)

**Idea**: Add a new accumulator state to the run frame. A new `ExprOp::ListAccumulate(slot, item)` appends to a per-run builder `Vec<SlotValue>` in O(1) amortized (real Vec::push). A new `ExprOp::ListMaterialize(slot)` materializes the builder into a single arena `Box<[SlotValue]>` and yields a `ListId`. Workflow authors express fan-out loops using a `ForEachStart` with `accumulator: Option<SlotIdx>` field.

**Pros**:
- Zero new dependencies.
- O(1) amortized append (real `Vec::push`).
- O(1) materialize at end of loop (single arena allocation).
- Builder state is per-run (lives in `RunFrame`), automatically discarded on completion.
- Replay-safe: the journal records `ListAccumulate`/`ListMaterialize` events with the same digest-binding discipline as every other op; replay reconstructs the same `Box<[T]>` in the same arena slot.
- No change to the `ListId` semantics for already-journaled lists — old `ExprOp::Append` still works for legacy workflows and for non-loop single appends.

**Cons**:
- New IR ops require engine-side branching in `eval_expr_operator` (a 2-line `match` arm).
- New `accumulator` field on `ForEachStart` (or a sibling op) is a compiler-side addition.
- Builder `Vec<SlotValue>` must be bounded by `MAX_LIST_ITEMS_PER_VALUE` to preserve the existing arena invariants — this is a per-run cap that the engine must enforce.
- Materialize must produce a list with stable `ListId` ordering so replay sees the same digest; trivially achievable since the builder is `Vec` (deterministic).

**Allocation profile**: 1× `Vec` + 1× `Box<[T]>` at materialize; amortized O(1) per append; no trie overhead.

**Verdict**: RECOMMENDED primary path.

### Option C: `AppendAll` batch op

**Idea**: A new `ExprOp::AppendAll(target_list, items_list)` that takes an already-built list of items and concatenates them in one arena operation. Caller builds the items list first (perhaps via a builder or pre-existing list).

**Pros**:
- No new deps.
- One arena allocation for N appends.
- Replay-safe.
- Composable with Option B: a loop body uses `ListAccumulate` + `ListMaterialize` to produce the items, then a single `AppendAll` to attach them.

**Cons**:
- Doesn't directly fix O(n²) for the loop-internal append case (the loop body still has to build up the items somehow).
- Most useful as a defensive optimization for callers that already have the items in hand.
- Adds IR surface for marginal gain if Option B is also implemented.

**Verdict**: RECOMMENDED as a secondary defence (combined with Option B); independent enough to ship as a separate bead if Option B alone proves insufficient.

## Options explicitly rejected

| Option | Reject reason |
|---|---|
| Mutate `Box<[T]>` in place after arena allocation | Breaks immutable-arena contract; replay cannot distinguish "never appended" from "appended once"; journal hashes diverge. |
| Switch arena to `Vec<SlotValue>` (growable) | Breaks `ListId` stability — `ListId` is the arena slot index, which changes if a later append forces a reallocation. |
| Use `RefCell<Vec<SlotValue>>` in the arena | Mutable shared state violates the runtime model (no interior mutability in `vb_core`). |
| Cache "last list id per slot" so subsequent appends reuse it | Non-deterministic across replay vs initial run; replay would rebuild a different identity. |
| `OnceCell<Vec<SlotValue>>` builder attached to the existing `ListId` | Same `ListId`-stability objection as the mutate-in-place option. |
| Worker-thread offload | Adds async/sync complexity for a fixed-bound loop; violates Power-of-Ten fixed loop bounds; runtime is synchronous by design. |

## Tradeoff matrix

| Option | Dep cost | Semantic change | Allocation | Determinism | Complexity |
|---|---|---|---|---|---|
| A — persistent vector | +1 dep (`im` or `rpds`) | none to IR | ~2-5× per list | depends on dep version | low |
| **B — accumulator node** | 0 | new IR ops + field | 1× Vec + 1× Box | replay-safe by construction | medium |
| C — `AppendAll` batch | 0 | new op | 1× Box | replay-safe | low |

## Recommendation: 2-stage rollout

**Stage 1 (vb-jim32, P1 feature bead)**: Implement Option B. Add `ExprOp::ListAccumulate`, `ExprOp::ListMaterialize`, an `accumulator: Option<SlotIdx>` field on `ForEachStart`, and engine-private builder state on `RunFrame`. Expected 5036× speedup on fan-out loops.

**Stage 2 (deferred, P2)**: If profiling shows other list-mutation hot paths beyond the loop-body case, add Option C as `ExprOp::AppendAll`.

Option A is deferred indefinitely — the dep cost and trie overhead are not justified when Option B solves the problem with zero new deps.

## Validation strategy for vb-jim32

1. Extend `crates/vb_core/benches/expr_eval_micro.rs` with a `BENCH-CANDIDATE-SKETCH` (Option B candidate, `#[ignore]`d) using a per-run builder `Vec`. Confirm the 5036× speedup is reproducible.
2. Add tests at `crates/vb_core/src/engine/expr_eval/ops_text_list_tests.rs`:
   - `accumulate_then_materialize_yields_same_content_as_repeated_append`
   - `accumulate_respects_max_list_items_per_value_bound`
   - `materialize_on_empty_accumulator_returns_empty_list`
   - `accumulate_into_one_slot_does_not_affect_another_slot_accumulator`
   - `replay_yields_identical_digest_to_initial_run` (proptest over a fixed workflow)
3. Add proof seed (`proof-seeds.jsonl` if used) noting the replay-byte-equality invariant.

## Replay-safety checklist

- [ ] Builder state lives in `RunFrame`, not in the arena.
- [ ] `ExprOp::ListAccumulate` and `ExprOp::ListMaterialize` are journaled as regular op events with the same digest-binding discipline as every other op.
- [ ] `MAX_LIST_ITEMS_PER_VALUE` is enforced on the builder before materialize.
- [ ] The materialized `ListId` is the arena slot index of the inserted `Box<[T]>`; deterministic given a deterministic sequence of events.
- [ ] No change to the `ListId` of any list that was materialized before the change lands (legacy compatibility).
