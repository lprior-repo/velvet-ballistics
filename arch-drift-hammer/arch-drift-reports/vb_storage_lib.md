# Architectural Drift Report: `vb_storage/src/lib.rs`

## File: `crates/vb_storage/src/lib.rs`
**Lines: 269** (UNDER 300 ✓)

---

## Summary

| Metric | Status |
|--------|--------|
| Line Count | ✓ PASS (269 < 300) |
| DDD Cohesion | ⚠ MODERATE SMELL |
| Violations | 2 found |

---

## DDD Cohesion Analysis

### Role of this File
This `lib.rs` acts as a **facade / anti-corruption layer** for the `vb_storage` crate. It:
1. Declares public submodules
2. Re-exports types and functions from submodules
3. Provides thin convenience wrapper functions (Transaction Script pattern)

### Is This Appropriate?
**YES** — For a storage crate boundary, a facade is the correct DDD pattern. The storage crate should not contain domain logic; it should expose a clean API to the domain layer.

---

## Violations

### 🔴 VIOLATION 1: Code Duplication (Medium Priority)

**Location:** Lines 184-186 vs Lines 179-181

```rust
/// Opens the Fjall-backed storage engine.
pub fn open_store(path: impl AsRef<std::path::Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path, None)
}

/// Initializes all declared keyspaces by opening the store.
pub fn init_keyspaces(path: impl AsRef<std::path::Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path, None)  // ← IDENTICAL
}
```

**Problem:** `init_keyspaces` is a duplicate of `open_store`. This is dead code that adds noise.

**Fix:** Remove `init_keyspaces` or make it do something distinct.

---

### 🟡 VIOLATION 2: Primitive Obsession in Public API (Low Priority)

**Location:** Line 258

```rust
pub fn read_blob(
    journal: &FjallJournal,
    digest: [u8; constants::DIGEST_BYTES],  // ← Primitive array
) -> Result<Option<BlobRecord>, JournalError> {
```

**Problem:** Using a raw byte array for digest is primitive obsession. Should be wrapped in a `Digest` newtype.

**Fix:** Introduce a `Digest` newtype and use it here.

---

## DDD Smell

### ⚠️ Transaction Script Pattern in Facade

The convenience wrapper functions (lines 178-269) are thin transaction scripts that delegate directly to `FjallJournal`. While this is **acceptable** for a storage facade, it provides no domain behavior encapsulation.

**Assessment:** Acceptable for anti-corruption layer. No action required unless domain logic creeps in.

---

## Priority

| Violation | Priority | Effort |
|-----------|----------|--------|
| Code duplication (`init_keyspaces`) | **Medium** | Low (1-line removal) |
| Primitive obsession (`[u8; DIGEST_BYTES]`) | **Low** | Medium (newtype + migration) |

**Overall Priority: LOW** — File is well-structured as a facade. Violations are cosmetic/debatable.

---

## Recommendation

1. **Remove `init_keyspaces`** — it's dead duplicate code
2. **Consider `Digest` newtype** — future improvement, not urgent

**STATUS: PERFECT** — File is under 300 lines and architecturally sound for its role as storage facade.
