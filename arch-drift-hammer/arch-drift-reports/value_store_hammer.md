# ARCHITECTURAL DRIFT REPORT: `value_store.rs`

**File:** `crates/vb_core/src/value_store.rs`
**Total Lines:** 2552
**Limit:** 300 lines
**Severity:** CATASTROPHIC — 8.5× over budget

---

## EXECUTIVE SUMMARY

This file is a **monolithic cold-value arena** that triples as:
1. Arena implementation (lines 1–418)
2. Kani proof harnesses (lines 420–449)
3. **2,100-line test suite** (lines 452–2552) — 82% of the file

The test suite is the primary line-count offender. The production code itself (~418 lines) is borderline acceptable but still violates DDD cohesion by lumping validation helpers, ID-to-index converters, and arena logic into one undifferentiated blob.

---

## VIOLATION 1: LINE COUNT BREACH (PRIMARY)

| Section | Lines | Status |
|---------|-------|--------|
| `ValueStore` struct + impl | 1–330 | 330 — OVER (needs split) |
| Free helpers (`validate_*`, `next_*_id`, `*_index`) | 332–418 | 87 — NEEDS ISOLATION |
| Kani harnesses | 420–449 | 30 — ISOLATE |
| Test module (`mod tests`) | 452–2552 | **2,101 — MASSIVE TEST BLOATEDNESS** |

**Required splits:**

```
crates/vb_core/src/value_store/
├── mod.rs              # Re-exports only, ~20 lines
├── arena.rs            # ValueStore struct + impl, ~330 lines
├── object_field.rs     # ObjectField struct + impl, ~42 lines
├── validation.rs       # All validate_* fns, ~40 lines
├── id_conversion.rs    # next_*_id + *_index fns, ~60 lines
├── kani_harnesses.rs  # PO-012 proof, ~30 lines
└── tests.rs            # All tests, moved to crate root or integration
```

The `mod tests` (2,101 lines) should be:
- Unit tests: kept inline but trimmed to essential coverage
- Extended tests: moved to `crates/vb_core/src/value_store/extended_tests.rs`
- Proptest property tests: moved to `crates/workspace_tests/`

---

## VIOLATION 2: REPOSITORY PATTERN — SHOULD BE ISOLATED

`ValueStore` is a **repository** (stores and retrieves domain values by handle). It should live in its own module, not be in `vb_core/src/value_store.rs` directly.

**Current:** `crates/vb_core/src/value_store.rs`
**Should be:** `crates/vb_core/src/value_store/mod.rs` (or `value_store/arena.rs`)

This is already partially recognized by the file existing as `value_store.rs` (singular, not plural `value_stores.rs`), but it needs to become a directory module.

---

## VIOLATION 3: PRIMITIVE OBSESSION

### 3a. Raw `usize` used for all index calculations

Functions like `next_symbol_id(len: usize)`, `symbol_index(id: SymbolId) -> CoreResult<usize>` use raw `usize` instead of domain-typed indices.

**Should be:**
- `SymbolIndex(u32)` newtype wrapping the index
- `SymbolArena` (or similar) owning the conversion, not free functions

### 3b. `SlotValue` exposed as raw handle type

`SlotValue` is used throughout but never wrapped in a domain type for the specific value kinds stored:
- `Box<[SlotValue]>` — raw vector, should be `ValueList`
- `Box<[ObjectField]>` — raw vector, should be `FieldSlice`
- `Bytes` (for blobs) — raw bytes, should be `BlobData`

### 3c. `Box<str>` for symbols — should be `SymbolText`

The arena stores `Vec<Box<str>>`. This is fine internals but the public API exposes raw `Box<str>` in `insert_symbol(value: impl Into<Box<str>>)`. Should wrap in a `SymbolText` newtype.

### 3d. `u32` / `u64` raw IDs in free functions

`next_blob_id(len: usize) -> CoreResult<BlobId>` — the `len` parameter is raw `usize`. Should use `BlobIndex(u64)` or similar.

---

## VIOLATION 4: MISSING DOMAIN TYPES (Scott Wlaschin)

### 4a. No `SymbolIndex` — free function `symbol_index` uses raw `usize`

**Current:**
```rust
fn symbol_index(id: SymbolId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::SymbolOutOfBounds { symbol: id })
}
```

**Should be:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolIndex(u32);

impl SymbolId {
    pub fn to_index(self) -> CoreResult<SymbolIndex> { ... }
}
```

### 4b. No `BlobIndex` — free function `blob_index` uses raw `usize`

Same pattern as 4a.

### 4c. No `ListIndex` / `ObjectIndex`

### 4d. No `SymbolText` newtype

`insert_symbol(&mut self, value: impl Into<Box<str>>)` — the `Box<str>` leaks to callers. Should be:
```rust
pub fn insert_symbol(&mut self, value: SymbolText) -> CoreResult<SymbolId>
```

### 4e. No `ValueList` newtype for `Box<[SlotValue]>`

### 4f. No `FieldSlice` newtype for `Box<[ObjectField]>`

### 4g. No `BlobBytes` newtype for `Bytes`

---

## VIOLATION 5: FREE FUNCTIONS THAT SHOULD BE METHODS

All these live at module scope but logically belong to the arena or to domain types:

| Function | Should Be |
|----------|-----------|
| `validate_list_len` | `impl ValueStore { fn validate_list_len }` or `List::validate_len` |
| `validate_symbol_len` | `impl SymbolText` |
| `validate_blob_len` | `impl BlobBytes` |
| `validate_object_len` | `impl ObjectFields` |
| `checked_len_to_u64` | private helper inside `ValueStore::total_arena_count` |
| `next_symbol_id` | `impl ValueStore` (private) |
| `next_list_id` | `impl ValueStore` (private) |
| `next_object_id` | `impl ValueStore` (private) |
| `next_blob_id` | `impl ValueStore` (private) |
| `symbol_index` | `SymbolId::to_index` |
| `list_index` | `ListId::to_index` |
| `object_index` | `ObjectId::to_index` |
| `blob_index` | `BlobId::to_index` |

---

## VIOLATION 6: `checked_len_to_u64` USES `unwrap_or`

```rust
fn checked_len_to_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)  // UNWRAP - prohibited by project rules
}
```

This violates the engineering rule "No `unwrap`, `expect`, `panic`". Even if unreachable, this must be `expect` or a proper error conversion.

---

## TEST SUITE BLOAT ANALYSIS

The test module (lines 452–2552, ~2,100 lines) is **82% of the file**. Problems:

1. **115+ individual `#[test]` functions** — most are trivially short (3–8 lines)
2. **Duplication**: Many tests follow identical patterns (e.g., `value_store_*_empty_store_rejects_*_id_zero` — four tests with near-identical structure)
3. **Proptest harness** (lines 1989–2026) should be in `crates/workspace_tests/`
4. **Security regression tests** (lines 1505–1971) could be moved to a dedicated security test module

**Examples of bloat:**
- Lines 628–673: Four near-identical "empty store rejects ID zero" tests
- Lines 676–759: Four near-identical "handle high ID rejected" tests
- Lines 2430–2528: ~100 lines of trivial `next_*_id` and `*_index` unit tests that test nothing beyond ID construction

---

## REQUIRED REFACTORING ACTIONS

### Priority 1: Split the file

```
value_store/
├── mod.rs          # pub use arena::*; pub use object_field::*;
├── arena.rs        # ValueStore (~330 lines)
├── object_field.rs # ObjectField (~42 lines)
├── validation.rs   # validate_* fns (~40 lines)
├── id_conversion.rs# next_*_id + *_index (~60 lines)
└── kani_harnesses.rs # PO-012 (~30 lines)
```

Move `mod tests` to `crates/workspace_tests/value_store_tests.rs` or similar.

### Priority 2: Eliminate `unwrap_or` in `checked_len_to_u64`

Replace with proper error conversion or `expect`.

### Priority 3: Create domain newtypes

- `SymbolIndex(u32)` — replace raw `usize` returns from index functions
- `BlobIndex(u64)` — same
- `SymbolText` — wrap `Box<str>` in public API
- `ValueList` — wrap `Box<[SlotValue]>`
- `FieldSlice` — wrap `Box<[ObjectField]>`

### Priority 4: Move validation helpers onto types

`validate_list_len` → `impl ValueList` or `ValueStore::validate_list_len`

### Priority 5: Trim test suite

- Deduplicate the 4× "empty store rejects ID zero" pattern into a helper + single parameterized test
- Move proptest to workspace_tests
- Move security regression tests to a dedicated `security_tests.rs` behind a feature flag

---

## ARCHITECTURAL COHERENCE SCORE

| Criterion | Score | Notes |
|-----------|-------|-------|
| Line count | 0/10 | 2552 vs 300 limit |
| Single responsibility | 2/10 | Arena + validation + ID conversion + tests |
| Primitive obsession | 3/10 | Raw usize, Box<str>, Box<[SlotValue]> leak through |
| Domain type coverage | 2/10 | Missing SymbolIndex, BlobIndex, ValueList, etc. |
| Free function isolation | 1/10 | 15+ module-level free functions |
| Test cohesion | 0/10 | 2,100-line test blob inline |

**OVERALL: 1.3/10 — CATASTROPHIC DRIFT**

---

## RECOMMENDATION

**IMMEDIATE SPLIT REQUIRED.** This file cannot remain as-is. The production code (~418 lines) should be refactored into `value_store/` module directory with proper domain newtypes. The test suite should be moved to workspace-level integration tests and aggressively deduplicated.
