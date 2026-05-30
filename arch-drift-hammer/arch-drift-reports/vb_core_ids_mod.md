# Architectural Drift Report: `vb_core/src/ids/mod.rs`

## File: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/ids/mod.rs`

---

## 1. Line Count Analysis

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **1101** | 300 | **VIOLATION** |

**Severity**: CRITICAL — file is **3.67x** over the 300-line limit.

---

## 2. DDD Cohesion Analysis

### What This Module Does
- Defines **14 numeric ID types** via `numeric_id!` macro: `WorkflowId`, `StepIdx`, `SlotIdx`, `ExprIdx`, `ActionId`, `AccessorIdx`, `ConstIdx`, `SymbolId`, `ListId`, `ObjectId`, `BlobId`, `RunId`, `EventSeq`, `SeqNo`
- Defines **7 additional manually-implemented types**: `BranchIdx`, `FanoutLimit`, `MaxAttempts`, `RetryCount`, `BranchCount`, `WorkflowDigest`
- Provides arithmetic utilities (`checked_add`, `as_usize`, `next()`)
- Provides parsing via `FromStr` ("parse, don't validate")
- Includes extensive test suite

### Cohesion Score: **HIGH** (topic-wise)
All types serve the single purpose of **type-safe identifiers** for the hot runtime. The module is thematically unified.

### DDD Quality Assessment

| Aspect | Rating | Notes |
|--------|--------|-------|
| NewType for IDs | ✅ EXCELLENT | No primitive obsession; all IDs are wrapped |
| Parse, don't validate | ✅ GOOD | `FromStr` implemented for all macro-generated IDs |
| Constexpr constructors | ✅ GOOD | `const fn new()` throughout |
| Checked arithmetic | ✅ GOOD | `checked_add`, `as_usize` prevent overflow |
| Zero constants | ✅ GOOD | `ZERO`, `MIN`, `MAX` provided where appropriate |
| Encapsulation | ✅ GOOD | `#[repr(transparent)]` with accessors only |

---

## 3. Violations

### CRITICAL

1. **FILE SIZE EXCEEDS 300 LINES (1101 lines)**
   - This is the primary architectural drift violation
   - The file bundles production code (lines 1–356), inline tests (lines 358–1092), and Kani stubs (lines 1094–1101) into a single 1101-line file
   - **Required action**: Split into separate files

### MINOR

2. **Kani module stubs inline at end of file**
   - Lines 1094–1101: `#[cfg(kani)] pub mod kani_id_bounds;` etc.
   - These are empty stub declarations that reference external modules; they should live in a dedicated `verification/` subdirectory, not inline in the production module

3. **Tests co-located with implementation**
   - Inline `#[cfg(test)] mod tests` at lines 358–1092
   - Best practice: tests belong in `tests/ids_generated.rs` or `ids/tests.rs`

---

## 4. DDD Smell Assessment

| Smell | Present | Severity |
|-------|---------|----------|
| Primitive Obsession | ❌ None | IDs properly wrapped |
| Primitive Hoarding | ❌ None | No raw `u32`/`u64` leaking |
| Type Envy | ❌ None | All behavior is on the types themselves |
| Inappropriate Intimacy | ❌ None | Clean public API |
| Feature Envy | ❌ None | Each ID type is self-contained |
| shotgun surgery | ⚠️ **YES** | 14 ID types defined in one file means any change to the macro touches this file |

### Overall DDD Smell: **LOW** (for the types themselves), **HIGH** (for file organization)

The code quality of the ID types themselves is exemplary. The drift is purely structural (file size + test placement).

---

## 5. Priority

| Priority | Rationale |
|----------|-----------|
| **P1 — MANDATORY** | File at 1101 lines violates hard 300-line architectural constraint |
| Split required before any further feature work lands on this file |

---

## 6. Recommended Refactoring

```
crates/vb_core/src/ids/
├── mod.rs           # 65 lines: re-exports + macro defs only
├── types.rs         # ~290 lines: BranchIdx, FanoutLimit, MaxAttempts, RetryCount, BranchCount, WorkflowDigest
├── generated.rs     # ~200 lines: macro expanded ID types (or keep macro in mod.rs)
├── run_id.rs        # ~50 lines: RunId specific impls (shard_index)
├── seq_no.rs        # ~50 lines: SeqNo specific impls (checked_add)
├── step_idx.rs      # ~50 lines: StepIdx specific impls
├── slot_idx.rs      # ~50 lines: SlotIdx specific impls
└── tests.rs         # ~735 lines: all tests moved here
```

---

## Summary

| Category | Result |
|----------|--------|
| Line Count | **1101 / 300** — VIOLATION |
| DDD Cohesion | **HIGH** — thematically unified, excellent type design |
| Violations | 1 critical (file size), 2 minor (test placement, kani stubs) |
| DDD Smell | LOW for types, HIGH for file organization |
| Priority | **P1 — MANDATORY REFACTOR** |

**STATUS**: `REFACTOR REQUIRED`
