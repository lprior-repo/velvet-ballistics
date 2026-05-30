# Architectural Drift Report: `gate_08_accessor.rs`

**File**: `crates/vb_validate/src/gate_08_accessor.rs`
**Status**: GUILTY — 596 lines (violates <300 line rule)
**Date**: 2026-05-29

---

## Executive Summary

This file validates accessor path segments for the workflow compiler. It suffers from **severe primitive obsession** — raw integer comparisons with `symbols_count: u32`, `u32::MAX` sentinel checks, and `as_usize()` extraction are endemic. The domain types `SymbolId`, `SlotIdx`, and `PathSegment` exist but their validation logic is performed on raw values at use sites rather than encapsulated in the types themselves.

---

## Violation 1: LINE COUNT (596 > 300)

| Metric | Value | Limit | Overage |
|--------|-------|-------|---------|
| Total lines | 596 | 300 | +296 (+98.7%) |
| Production code | ~75 | - | - |
| Test code | ~521 | - | - |
| Production-to-test ratio | 12.4% / 87.6% | 70% / 30% | **INVERTED** |

**The file cannot be shippable. It must be split.**

---

## Violation 2: Primitive Obsession — Raw Integer Comparisons

### Offender 1: `symbol.get()` vs `symbols_count: u32`

**Line 37**: `if symbol.get() < symbols_count`

```rust
fn validate_field_symbol(
    acc_index: usize,
    seg_index: usize,
    symbol: vb_core::ids::SymbolId,  // Domain type
    symbols_count: u32,              // Raw integer
) -> ValidationResult<()> {
    if symbol.get() < symbols_count {  // EXTRACTED raw value
        Ok(())
    } else {
        Err(ValidationError::AccessorSymbolOutOfBounds { ... })
    }
}
```

**Problem**: `SymbolId` is a domain type but we extract its raw value to compare against a raw count. This should be:

```rust
impl SymbolId {
    pub fn is_valid_for_symbols_count(self, count: u32) -> bool {
        self.get() < count
    }
}
```

Or better: `InBounds` trait on index types.

---

### Offender 2: `u32::MAX` Sentinel Magic

**Line 50**: `if idx == u32::MAX`

```rust
fn validate_index_segment(acc_index: usize, seg_index: usize, idx: u32) -> ValidationResult<()> {
    if idx == u32::MAX {  // MAGIC SENTINEL
        Err(ValidationError::AccessorPathInvalid { ... })
    } else {
        Ok(())
    }
}
```

**Problem**: `u32::MAX` is a magic number representing "invalid index." This sentinel value should be:

1. A named constant: `const INVALID_INDEX: u32 = u32::MAX;`
2. Encapsulated in a domain method: `impl PathSegment::Index { pub fn is_sentinel(self) -> bool }`
3. Better yet, make `PathSegment::Index` use a newtype that rejects sentinel at construction

---

### Offender 3: `accessor.root.as_usize()` vs `slot_count: u16`

**Line 65**: `if accessor.root.as_usize() >= usize::from(slot_count)`

```rust
fn validate_accessor_root(
    acc_index: usize,
    accessor: &AccessorProgram,
    slot_count: u16,
) -> ValidationResult<()> {
    if accessor.root.as_usize() >= usize::from(slot_count) {  // EXTRACTED + CAST
        return Err(ValidationError::AccessorSlotOutOfRange { ... });
    }
    Ok(())
}
```

**Problem**: `SlotIdx` has `as_usize()` because `checked_index!` macro adds it. But validation is still at use site with raw comparisons. Should be:

```rust
impl SlotIdx {
    pub fn is_valid_slot(self, slot_count: u16) -> bool {
        self.as_usize() < usize::from(slot_count)
    }
}
```

---

### Offender 4: `PathSegment::Index(u32)` — No Domain Invariant

**Line 315 in workflow/mod.rs**: `Index(u32)` stores raw `u32` directly.

```rust
pub enum PathSegment {
    Field(SymbolId),
    Index(u32),  // RAW INTEGER — sentinel check pushed to use site
}
```

**Problem**: The sentinel validation (`idx == u32::MAX`) is done at every use site in `gate_08_accessor.rs`. It should be enforced at construction or via a smart constructor.

**Fix**: Create a `IndexIdx` newtype:

```rust
pub struct IndexIdx(u32);

impl IndexIdx {
    pub fn new(value: u32) -> Option<Self> {
        if value == u32::MAX { None } else { Some(Self(value)) }
    }
    
    pub fn is_valid(self) -> bool {
        self.0 != u32::MAX
    }
}
```

---

## Violation 3: God Function — `validate_gate_08_accessor_path_segments`

**Lines 10–29**: 19 lines. Single function validates 3 things:

1. Accessor root bounds (delegate to `validate_accessor_root`)
2. Field symbol bounds (delegate to `validate_field_symbol`)
3. Index segment validity (delegate to `validate_index_segment`)

### Scott Wlaschin Says

> "A function should do one thing. If it does multiple things, you need multiple functions."

This function does one thing (validates accessor path segments) but handles 3 segment types. The delegation is correct, but the file structure mixes validation logic with 521 lines of tests.

### Prescribed Refactoring

```
gate_08_accessor/
├── mod.rs              (reexports, orchestration, main validator)
├── root.rs             (validate_accessor_root)
├── field.rs            (validate_field_symbol)
├── index.rs            (validate_index_segment)
└── shared.rs           (InBounds trait, validation helpers)
```

Each file: **<60 lines**.

---

## Violation 4: Test Code Inline with Production Code

**Lines 75–596**: 521 lines of tests inline in the production module.

### Problems

- Tests should be in `crates/vb_validate/tests/gate_08_accessor_test.rs` or similar
- The `workflow_parts_with_accessors`, `one_accessor_parts_with_segment`, `accessor_allocating_boxed_path` helpers are **copy-paste bait** for other modules
- 521 lines of test for ~75 lines of production is **87%/13% ratio** — catastrophically inverted
- The `#[cfg(kani)] mod verification` block (lines 518–595) should be in `vb_validate/kani/` or behind a feature flag

### Evidence of Copy-Paste

Compare the test helpers with `gate_10_node.rs` test helpers — likely duplicated.

---

## Violation 5: Magic Sentinel Without Named Constant

| Location | Magic | Should Be |
|----------|-------|-----------|
| Line 50 | `idx == u32::MAX` | `IndexIdx::INVALID` or `IndexIdx::is_sentinel(idx)` |
| Line 253 | `u32::MAX.saturating_sub(1)` | Same, but with context |
| Line 483 | `u32::MAX` | Same |

**Sentinel values should be encapsulated in the type, not checked at use sites.**

---

## Violation 6: Duplicate Validation Logic

`vb_core/src/workflow/mod.rs` lines 1287–1330 contains `validate_accessors` and `validate_accessor_path_symbols` — **duplicate validation logic**.

```rust
// In workflow/mod.rs (core):
pub(crate) fn validate_accessors(accessors: &[AccessorProgram], slot_count: u16) -> Result<(), WorkflowError> { ... }
pub(crate) fn validate_accessor_path_symbols(accessors: &[AccessorProgram]) -> Result<(), WorkflowError> { ... }

// In gate_08_accessor.rs (validate crate):
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> { ... }
```

**Which one is authoritative?** This is a split-brain validation problem.

---

## Summary of Required Fixes

| # | Issue | Severity | Fix |
|---|-------|----------|-----|
| 1 | 596 lines | **CRITICAL** | Split into `gate_08_accessor/` dir module |
| 2 | 3× `as_usize()` + raw comparisons | **HIGH** | Add `InBounds` trait to index types |
| 3 | `u32::MAX` sentinel magic | **HIGH** | `IndexIdx` newtype with `is_sentinel()` |
| 4 | Tests inline | **MEDIUM** | Move to `tests/` directory |
| 5 | Duplicate validation | **HIGH** | Deduplicate with core; one authoritative source |
| 6 | Production-to-test ratio | **CRITICAL** | Move tests out |

---

## Ideal File Structure After Refactor

```
crates/vb_validate/src/
├── gate_08_accessor/
│   ├── mod.rs          (~30 lines: reexports + main validator)
│   ├── root.rs         (~20 lines: validate_accessor_root)
│   ├── field.rs        (~25 lines: validate_field_symbol)  
│   ├── index.rs        (~20 lines: validate_index_segment)
│   └── shared.rs       (~40 lines: InBounds trait, constants)
├── gate_08_accessor.rs (deleted after split)
└── tests/
    └── gate_08_accessor_test.rs  (~200 lines: focused tests)
```

---

## Verification Command

```bash
# After refactoring:
moon ci
# OR
cargo test -p vb_validate -- gate_08
cargo clippy -p vb_validate
```

---

## Conclusion

** Hammer applied.** This file is structurally guilty of primitive obsession and line count violation. The `SymbolId`, `SlotIdx`, and `PathSegment::Index` types exist but their validation is performed on raw integers at use sites. The fix is not just file splitting — it requires **proper domain method integration** on the ID types themselves. Until `InBounds` trait exists and is used consistently, validation code will remain scattered and inconsistent.

**Recommended bead**: Create `gate_08` refactor bead for proper domain integration.
