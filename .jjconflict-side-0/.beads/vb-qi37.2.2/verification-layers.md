# Verification Layers: vb-qi37.2.2 — ValueStore Arena Cap Enforcement

## Layer Assignments

This document maps each contract clause to verification layers per the five-lane gauntlet.

| Layer | Tool | Scope | Clauses Covered |
|---|---|---|---|
| `:verify-fast` | unit tests (in-process) | All insert/reject paths, monotonicity, rejection atomicity | C1–C2, I1–I5, A1–A6, C3–C4, INV1–INV4 |
| `:verify-standard` | integration tests (vb_runtime) | Shard-local store behavior under concurrent access | Shard lifecycle integration |
| `:verify-deep` | Kani (bounded model checker) | Arena cap enforcement, handle bounds, no panic paths | I1–I5 (cap enforcement), A1–A6 (bounds), INV1 |
| `:verify-proof` | **WAIVED** | Not applicable — mutable Rust data structures | WAIVER-001 applies |
| `:verify-all` | Miri (UB detection) | Unsafe-sensitive paths, interior mutability semantics | All insert paths |

---

## Lane 1: `:verify-fast` — Unit Tests

**Tool:** Built-in Rust test harness (`#[test]`)
**Run:** `cargo test -p vb_core value_store`
**Time budget:** < 30s

### Clauses Covered

| Clause | Test(s) |
|---|---|
| C1: `ValueStore::new()` uncapped | `value_store_new_has_no_cap_and_allows_unlimited_inserts` |
| C2: `with_max_slots()` capped | `value_store_with_max_slots_allows_inserts_up_to_cap`, `value_store_with_max_slots_one_rejects_second_insert` |
| I1: Symbol insertion | `insert_symbol_rejects_payload_over_hard_bound`, `value_store_symbol_handle_high_id_rejected`, `value_store_insert_symbol_empty_string_is_valid` |
| I2: List insertion | `insert_list_rejects_payload_over_hard_bound`, `value_store_list_handle_high_id_rejected`, `value_store_insert_list_empty_is_valid` |
| I3: List with taint | `list_item_and_object_field_accessors_are_checked` (basic) |
| I4: Object insertion | `insert_object_rejects_payload_over_hard_bound`, `value_store_object_handle_high_id_rejected`, `value_store_insert_object_empty_is_valid` |
| I5: Blob insertion | `insert_blob_rejects_payload_over_hard_bound`, `value_store_blob_handle_high_id_rejected`, `value_store_insert_blob_empty_is_valid` |
| A1–A6: Accessors | `arena_accessors_report_handle_bounds`, `symbol_and_blob_accessors_return_payloads`, `list_item_and_object_field_accessors_are_checked` |
| C3–C4: Counts | `value_store_counts_track_insertions`, `value_store_default_equals_new` |
| INV1: Pre-mutation cap check | `value_store_rejected_symbol_over_max_does_not_mutate_arena`, `value_store_rejected_list_over_max_does_not_mutate_arena`, `value_store_rejected_object_over_max_does_not_mutate_arena`, `value_store_rejected_blob_over_max_does_not_mutate_arena` |
| INV2: Monotonic IDs | `value_store_sequential_ids_are_monotonic` |
| INV3: Rejection atomicity | Same as INV1 tests |
| INV4: Handle validity | `value_store_empty_store_rejects_symbol_id_zero`, `value_store_*_id_that_was_never_inserted_returns_out_of_bounds` |
| Edge: exact max limits | `value_store_symbol_at_exact_max_length_is_accepted`, `value_store_list_at_exact_max_length_is_accepted`, `value_store_object_at_exact_max_fields_is_accepted`, `value_store_blob_at_exact_max_bytes_is_accepted` |
| Edge: index bounds | `value_store_list_item_index_zero_on_empty_list_fails`, `value_store_list_item_max_u32_index_rejected`, `value_store_list_index_at_exact_length_is_rejected` |

**Minimum passing tests:** All 50+ tests in `value_store.rs` `#[cfg(test)]` module.

---

## Lane 2: `:verify-standard` — Integration Tests

**Tool:** `vb_runtime` integration test suite
**Run:** `cargo test -p vb_runtime`
**Time budget:** < 120s

### Clauses Covered

| Clause | Test(s) |
|---|---|
| Shard-local ValueStore behavior | `vb_runtime` shard lifecycle tests with value store |
| Concurrent insert safety | `value_store_with_max_slots_*` within multi-run context |
| Budget propagation | Integration with `AggregateResourceBudget` |

**Evidence:** Test artifacts in `crates/vb_runtime/tests/`

---

## Lane 3: `:verify-deep` — Kani Model Checker

**Tool:** Kani (`cargo kani`)
**Run:** `cargo kani --tests`
**Time budget:** < 300s

### Clauses Covered

| Clause | Kani Harness |
|---|---|
| I1–I5: Cap enforcement | `value_store_with_max_slots_allows_inserts_up_to_cap`, `value_store_with_max_slots_one_rejects_second_insert` |
| A1–A6: Bounds safety | All accessor functions with `SymbolId/ListId/ObjectId/BlobId` arguments |
| INV1: No partial mutation | Harness verifying store state unchanged after rejected insert |
| No panic paths | All index operations use checked arithmetic (no `unwrap` on hot paths) |

**Kani-specific:**
- Bounded loop unwinding (max 4 inserts per arena type = 16 total)
- Symbolic handles for all four ID types
- Counterexample detection for cap enforcement violations

**Note:** Kani proof is bounded. Full verification requires unit test coverage for edge cases.

---

## Lane 4: `:verify-proof` — Lean (WAIVED)

**Status:** WAIVER-001 applies

ValueStore involves mutable Rust data structures with interior mutability:
- Mutable `Vec` collections (`symbols`, `lists`, `objects`, `blobs`)
- Mutable `IndexMap` secondary indices
- Arena cap check reads while mutations occur
- Monotonic ID assignment dependent on collection length

No Lean projection applies. See `lean-contract.md` for full justification.

---

## Lane 5: `:verify-all` — Miri / cargo-careful

**Tool:** Miri (`cargo miri test`), cargo-careful
**Run:** `cargo miri test -p vb_core -- value_store`
**Time budget:** < 600s (Miri is slow)

### Clauses Covered

| Clause | Miri Check |
|---|---|
| All insert paths | UB detection for interior mutability patterns |
| Handle resolution | No use-after-free, no invalid memory |
| Cap check race (happen-before) | Miri does not detect data races (single-threaded); this is covered by integration tests |

**Note:** ValueStore is **not** thread-safe (`!Sync`). Miri checks are for single-threaded UB (use-after-free, invalid enum discriminant, etc.), not data races.

---

## Coverage Matrix

| Clause | Unit | Integration | Kani | Lean | Miri |
|---|---|---|---|---|---|
| C1: new() uncapped | ✓ | ✓ | ✓ | WAIVED | ✓ |
| C2: with_max_slots capped | ✓ | ✓ | ✓ | WAIVED | ✓ |
| I1: Symbol insert | ✓ | ✓ | ✓ | WAIVED | ✓ |
| I2: List insert | ✓ | ✓ | ✓ | WAIVED | ✓ |
| I3: List + taint | ✓ | ✓ | — | WAIVED | ✓ |
| I4: Object insert | ✓ | ✓ | ✓ | WAIVED | ✓ |
| I5: Blob insert | ✓ | ✓ | ✓ | WAIVED | ✓ |
| A1–A6: Accessors | ✓ | ✓ | ✓ | WAIVED | ✓ |
| C3–C4: Counts | ✓ | ✓ | ✓ | WAIVED | ✓ |
| INV1: Pre-mutation cap | ✓ | ✓ | ✓ | WAIVED | ✓ |
| INV2: Monotonic IDs | ✓ | ✓ | — | WAIVED | ✓ |
| INV3: Rejection atomicity | ✓ | ✓ | ✓ | WAIVED | ✓ |
| INV4: Handle validity | ✓ | ✓ | ✓ | WAIVED | ✓ |
| Edge: max limits | ✓ | ✓ | ✓ | WAIVED | ✓ |
| Edge: index bounds | ✓ | ✓ | ✓ | WAIVED | ✓ |
| WAIVER-001 | — | — | — | WAIVED | — |

---

## Evidence Artifacts

| Artifact | Location | Lane |
|---|---|---|
| Unit test results | `cargo test -p vb_core -- value_store` stdout | `:verify-fast` |
| Integration test results | `cargo test -p vb_runtime` stdout | `:verify-standard` |
| Kani report | `target/kani/*.html` | `:verify-deep` |
| Lean waiver | `.beads/vb-qi37.2.2/lean-contract.md` | `:verify-proof` |
| Miri report | `cargo miri test` stdout | `:verify-all` |

---

*End of verification layers.*