# Lean Contract Projection: vb-qi37.2.2 — ValueStore Arena Cap Enforcement

## Boundary

- **Rust/runtime shell:** `crates/vb_core/src/value_store.rs` — mutable cold value arenas with interior mutability backing handle-only runtime slot values.
- **Lean-owned kernel:** None. No Lean projection applies to this module.

## No Lean Projection

**Reason:** `ValueStore` involves mutable Rust data structures with interior mutability:

1. **Mutable Vec collections** — `symbols: Vec<Box<str>>`, `lists: Vec<Box<[SlotValue]>>`, `objects: Vec<Box<[ObjectField]>>`, `blobs: Vec<Bytes>`. All insertions mutate the Vec in-place via `push`.
2. **Mutable IndexMap secondary indices** — `object_field_index: Vec<IndexMap<SymbolId, SlotValue>>` and `object_taint_index: Vec<IndexMap<SymbolId, Taint>>`. Both use `entry().or_insert()` for in-place hash-map mutations during object insertion.
3. **Interior mutability pattern** — The arena cap check `check_arena_cap()` reads from `self.max_arena_entries` and `self.total_arena_count()` while subsequent insert operations mutate the same struct's interior collections. The struct is not frozen during these operations.
4. **Handle-based mutation semantics** — IDs are assigned monotonically via `next_*_id` functions that read the current length of mutable collections to compute the next ID, then push to those same collections.

Lean is designed for pure deterministic kernels with immutable data. The combination of mutable collection traversal (`Vec::push`, `IndexMap::entry`), monotonic ID assignment dependent on current collection length, and the arena cap enforcement gating mutations makes this module unsuitable for a Lean refinement projection.

## Compensating Evidence

The absence of Lean proof is compensated by:

| Concern | Tool | Coverage |
|---|---|---|
| Arena cap enforcement correctness | unit + integration + Kani | `value_store` tests: `value_store_with_max_slots_allows_inserts_up_to_cap`, `value_store_with_max_slots_one_rejects_second_insert`, `value_store_new_has_no_cap_and_allows_unlimited_inserts` |
| Handle monotonicity | unit | `value_store_sequential_ids_are_monotonic` |
| Bounds safety on all accessors | unit + Kani | All `*_count()`, `*_index()` functions use checked arithmetic; no unchecked indexing |
| Rejection atomicity (no partial mutation) | unit | `value_store_rejected_*_over_max_does_not_mutate_arena` series |
| ID invalidation after drop | unit | `value_store_*_id_that_was_never_inserted_returns_out_of_bounds` series |
| Concurrent shard-local access | integration | `vb_runtime` shard lifecycle integration tests |

## Waiver

**WAIVER-001: ValueStore mutable Rust data structures — not Lean-owned.**
- Owner: vb-qi37.2.2 contract synthesizer
- Reason: ValueStore is a mutable Rust collection wrapper with interior mutability. Proving functional correctness of the arena cap enforcement in Lean would require modeling Rust's `Vec`, `IndexMap`, and interior mutability semantics, which is outside Lean's modeling scope for pure deterministic kernels.
- Compensating evidence: 20+ unit tests covering cap enforcement, handle monotonicity, bounds safety, and rejection atomicity. Integration tests in `vb_runtime` verify shard-local store behavior under concurrent access patterns.
- Expiry: None — permanent architectural waiver.
