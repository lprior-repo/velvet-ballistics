# Test Plan: vb-qi37.2.2 — ValueStore Arena Cap Enforcement

## Overview

This test plan covers `crates/vb_core/src/value_store.rs` — the cold value arena backing handle-only runtime slot values. The primary feature under test is the **per-run aggregate arena cap enforcement**: a hard limit on the total number of arena entries across all four arenas (symbols, lists, objects, blobs).

---

## Test Strategy

### Layer 1: Unit Tests (`:verify-fast`)

**Location:** `crates/vb_core/src/value_store.rs` `#[cfg(test)]` module
**Execution:** `cargo test -p vb_core -- value_store`
**Coverage target:** 50+ tests covering all contract clauses
**Time budget:** < 30s

**Categories:**
- Happy path: 7 tests (HP-1 through HP-7)
- Error path: 12 tests (EP-1 through EP-12)
- Edge cases: 11 tests (EC-1 through EC-11)
- Contract verification: 3 tests (CV-1 through CV-3)
- Adversarial/violation: 6 tests (VA-1 through VA-6)

**Total:** 39+ named test scenarios, ~50 individual test functions

---

### Layer 2: Integration Tests (`:verify-standard`)

**Location:** `crates/vb_runtime/tests/`
**Execution:** `cargo test -p vb_runtime`
**Coverage target:** Shard-local store behavior, concurrent access patterns, budget propagation
**Time budget:** < 120s

**Key scenarios:**
- `ValueStore` within `Runtime` shard lifecycle
- Multi-run concurrent insertion stress
- Budget admission with `AggregateResourceBudget`

---

### Layer 3: Kani Model Checking (`:verify-deep`)

**Location:** `crates/vb_core/tests/aggregate_resource_budget_kani_red.rs` or dedicated Kani harness
**Execution:** `cargo kani --tests`
**Coverage target:** Cap enforcement correctness, handle bounds, no panic paths
**Time budget:** < 300s

**Kani-specific coverage:**
- Bounded model: max 4 inserts per arena type (16 total entries)
- Symbolic handles: `SymbolId`, `ListId`, `ObjectId`, `BlobId`
- Counterexample detection for cap violations

**Harnesses:**
1. `value_store_with_max_slots_allows_inserts_up_to_cap` — verify cap gates insert
2. `value_store_with_max_slots_one_rejects_second_insert` — verify rejection
3. All accessor functions with symbolic IDs

---

### Layer 4: Miri UB Detection (`:verify-all`)

**Location:** Same as unit tests
**Execution:** `cargo miri test -p vb_core -- value_store`
**Coverage target:** Single-threaded UB (use-after-free, invalid memory)
**Time budget:** < 600s

**Note:** ValueStore is `!Sync`, so Miri does not check data races. This is covered by integration tests.

---

## Test Inventory

### Critical Path Tests

| Test | Clause | Description |
|---|---|---|
| `value_store_with_max_slots_allows_inserts_up_to_cap` | C2, I1–I5 | Cap allows exactly `max_slots` inserts |
| `value_store_with_max_slots_one_rejects_second_insert` | C2, I1 | Cap rejects insert #2 when cap=1 |
| `value_store_new_has_no_cap_and_allows_unlimited_inserts` | C1 | Uncapped store allows 100 inserts |
| `value_store_rejected_*_over_max_does_not_mutate_arena` | INV1, INV3 | Rejected insert leaves store unchanged |
| `value_store_sequential_ids_are_monotonic` | INV2 | IDs increase monotonically |
| `arena_accessors_report_handle_bounds` | A1–A4 | Invalid IDs return out-of-bounds |
| `value_store_counts_track_insertions` | C3 | `total_arena_count()` equals sum |

### Boundary Tests

| Test | Clause | Boundary |
|---|---|---|
| `value_store_symbol_at_exact_max_length_is_accepted` | I1 | Exactly `MAX_SYMBOL_BYTES_PER_VALUE` |
| `value_store_list_at_exact_max_length_is_accepted` | I2 | Exactly `MAX_LIST_ITEMS_PER_VALUE` |
| `value_store_object_at_exact_max_fields_is_accepted` | I4 | Exactly `MAX_OBJECT_FIELDS_PER_VALUE` |
| `value_store_blob_at_exact_max_bytes_is_accepted` | I5 | Exactly `MAX_BLOB_BYTES_PER_VALUE` |
| `insert_symbol_rejects_payload_over_hard_bound` | I1 | `MAX_SYMBOL_BYTES_PER_VALUE + 1` |
| `insert_list_rejects_payload_over_hard_bound` | I2 | `MAX_LIST_ITEMS_PER_VALUE + 1` |
| `insert_object_rejects_payload_over_hard_bound` | I4 | `MAX_OBJECT_FIELDS_PER_VALUE + 1` |
| `insert_blob_rejects_payload_over_hard_bound` | I5 | `MAX_BLOB_BYTES_PER_VALUE + 1` |
| `value_store_blob_one_byte_over_limit_is_rejected` | I5 | `MAX_BLOB_BYTES_PER_VALUE + 1` |
| `value_store_list_index_at_exact_length_is_rejected` | A5 | Index == list.len() |
| `value_store_list_item_max_u32_index_rejected` | A5 | Index == u32::MAX |

### Adversarial Tests

| Test | Attack Vector |
|---|---|
| `value_store_symbol_handle_high_id_rejected` | High ID on populated store |
| `value_store_list_handle_high_id_rejected` | High ID on populated store |
| `value_store_object_handle_high_id_rejected` | High ID on populated store |
| `value_store_blob_handle_high_id_rejected` | High ID on populated store |
| `value_store_empty_store_rejects_symbol_id_zero` | Zero ID on empty store |
| `value_store_empty_store_rejects_list_id_zero` | Zero ID on empty store |
| `value_store_empty_store_rejects_object_id_zero` | Zero ID on empty store |
| `value_store_empty_store_rejects_blob_id_zero` | Zero ID on empty store |
| `value_store_blob_id_that_was_never_inserted_returns_out_of_bounds` | Never-inserted ID |
| `value_store_object_field_on_wrong_object_returns_not_found` | Key cross-contamination |

### Empty Value Tests

| Test | Description |
|---|---|
| `value_store_insert_symbol_empty_string_is_valid` | Empty symbol round-trips |
| `value_store_insert_list_empty_is_valid` | Empty list round-trips |
| `value_store_insert_object_empty_is_valid` | Empty object round-trips |
| `value_store_insert_blob_empty_is_valid` | Empty blob round-trips |
| `value_store_list_item_index_zero_on_empty_list_fails` | Index 0 on empty list fails |

### Structural Tests

| Test | Description |
|---|---|
| `value_store_default_is_same_as_new` | `Default::default() == ValueStore::new()` |
| `value_store_clone_is_equal` | Clone equals original |
| `value_store_list_with_mixed_slot_value_types` | All `SlotValue` variants in list |
| `value_store_object_field_linear_scan_respects_insertion_order` | Duplicate key first-wins |
| `value_store_exact_max_list_accesses_edges_without_unchecked_indexing` | No panic at max list |
| `value_store_exact_max_object_preserves_duplicate_first_wins_index` | No panic at max object |

---

## Mutation Coverage

Per `cargo-mutants` and `cargo-llvm-cov` requirements:

### Must-Kill Mutations

1. **Cap check removal** — Remove `check_arena_cap()?` from any insert → must cause test failure
2. **Push ordering swap** — Swap `check_arena_cap` and `next_*_id` → must cause test failure
3. **Count computation** — Change `total_arena_count` to return wrong sum → must cause test failure
4. **Bounds check removal** — Remove `.get()` safety on arena vectors → must cause UB/Miri failure

### Evidence

```
cargo mutants -- cargo test -p vb_core -- value_store
cargo llvm-cov -- cargo test -p vb_core -- value_store
```

---

## Evidence Requirements

| Artifact | Gate |
|---|---|
| `cargo test -p vb_core -- value_store` stdout | `:verify-fast` |
| `cargo test -p vb_runtime` stdout | `:verify-standard` |
| `cargo kani --tests` report | `:verify-deep` |
| `lean-contract.md` WAIVER-001 | `:verify-proof` |
| `cargo miri test -p vb_core -- value_store` stdout | `:verify-all` |

---

*End of test plan.*