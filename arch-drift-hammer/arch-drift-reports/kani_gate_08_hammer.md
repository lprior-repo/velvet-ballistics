# ARCHITECTURAL DRIFT REPORT: kani_gate_08_structural.rs

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/kani_gate_08_structural.rs`
**Status:** VIOLATION DETECTED
**Line Count:** 589 (exceeds 300-line limit by 96%)

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 589 | 300 | **FAIL** (+96%) |
| Harnesses | 14 | N/A | N/A |
| Avg lines/harness | 42 | N/A | N/A |

The file is **nearly double** the 300-line architectural limit. This is a pure Kani harness file containing 14 proof functions. Each harness has nearly identical setup boilerplate, making this a textbook case for extraction into shared harness-building utilities.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Domain Types Exist But Are Underutilized

The codebase **already has** proper domain types in `vb_core::ids`:
- `SlotIdx` (wraps `u16`)
- `SymbolId` (wraps `u32`)
- `AccessorIdx` (wraps `u16`)
- `StepIdx` (wraps `u16`)

**However**, the harness file uses raw primitives in harness *setup*, creating a **domain-model/parity mismatch**:

| Location | Raw Type Used | Should Be |
|----------|---------------|-----------|
| Line 37 | `u16` for `slot_count` | `SlotCount` (newtype) |
| Line 38 | `u32` for `symbols_count` | `SymbolCount` (newtype) |
| Line 39 | `u16` for `root` | `SlotIdx` |
| Line 40 | `u32` for `symbol` | `SymbolId` |
| Line 41 | `u32` for `index` | `AccessorIndex` (newtype) |
| Line 53 | `kani::any::<u8>()` | `AccessorVariantSelector` (newtype) |
| Lines 104, 340-345 | Multiple `u32` for `idx0..idx4, idx100` | `AccessorIndex` |

### 2.2 Concrete Primitive Obsession Examples

**VIOLATION 1: Lines 36-49 - Raw numeric soup in harness builder**
```rust
let slot_count: u16 = kani::any();
let symbols_count: u32 = kani::any();
let root: u16 = kani::any();
let symbol: u32 = kani::any();
let index: u32 = kani::any();
```
These should be domain-typed to show the harness builder's *intent*. The raw types make it unclear whether `slot_count` is a `SlotCount` vs `SlotIdx` vs raw `u16`.

**VIOLATION 2: Lines 336-345 - Index Primitive Obsession**
```rust
let idx0: u32 = kani::any();
let idx1: u32 = kani::any();
let idx2: u32 = kani::any();
let idx3: u32 = kani::any();
let idx4: u32 = kani::any();
let idx100: u32 = kani::any();
```
These are **list indices** used in `PathSegment::Index(...)`. They should be wrapped in an `AccessorIndex` newtype that also encodes the `u32::MAX` sentinel rejection semantics. Currently, the sentinel check is scattered across `kani::assume()` calls (lines 355-361) rather than being enforced by the type.

**VIOLATION 3: Line 53 - Raw Discriminator**
```rust
match kani::any::<u8>() {
    0 => ...,
    1 => ...,
    2 => ...,
    _ => ...,
}
```
This is a hand-rolled enum variant selector. Should be a proper `enum AccessorPathStyle { Empty, Field, Index, FieldIndex }` with `kani::any::<AccessorPathStyle>()`.

### 2.3 Missing Domain Types for Kani Harness Builders

The following domain types **do not exist** but should for harness-building:

| Missing Type | Purpose | Invariants |
|--------------|---------|------------|
| `SlotCount` | Bounds for slot allocation | `> 0`, `<= u16::MAX` |
| `SymbolCount` | Bounds for symbol table | `> 0`, `<= u32::MAX` |
| `AccessorIndex` | Index within accessor path | `!= u32::MAX` (sentinel) |
| `AccessorPathStyle` | Discriminator for path construction | `Empty \| Field \| Index \| Mixed` |

---

## 3. GATE 08 STRUCTURAL VALIDATION RESPONSIBILITY MAP

### 3.1 What Gate 08 Validates

From `gate_08_accessor.rs`:
- `validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()>`
- For each `AccessorProgram` in `parts.accessors`:
  1. **Root validation**: `accessor.root.as_usize() >= slot_count` → `Err(AccessorSlotOutOfRange)`
  2. **Field symbol validation**: `symbol.get() >= symbols_count` → `Err(AccessorSymbolOutOfBounds)`
  3. **Index sentinel validation**: `idx == u32::MAX` → `Err(AccessorPathInvalid)`

### 3.2 Harnesses and Their Coverage

| Harness | Lines | Structural Coverage | Validation Target |
|---------|-------|-------------------|-------------------|
| `kani_gate_08_arbitrary_parts_valid_accessors_pass` | 25-73 | Full structural, bounded accessors | Happy path |
| `kani_gate_08_arbitrary_parts_root_oob_rejected` | 78-97 | Root OOB | `AccessorSlotOutOfRange` |
| `kani_gate_08_arbitrary_parts_symbol_oob_rejected` | 102-131 | Symbol OOB | `AccessorSymbolOutOfBounds` |
| `kani_gate_08_arbitrary_parts_index_sentinel_rejected` | 136-155 | Index sentinel | `AccessorPathInvalid` |
| `kani_gate_08_full_structure_no_panic` | 162-168 | All WorkflowParts fields | No panic |
| `kani_gate_08_structure_coverage` | 172-197 | Coverage tracking | Structural variety |
| `kani_gate_08_arbitrary_resource_contract` | 202-210 | `resource_contract` field | Must tolerate |
| `kani_gate_08_step_names_independent_of_slots` | 218-223 | `step_names` vs `slot_count` | Independence |
| `kani_gate_08_empty_nodes_valid_accessors_pass` | 231-254 | Empty nodes | Nodes not required |
| `kani_gate_08_expressions_with_accessor_refs` | 261-329 | `ExprOp::LoadAccessor` | Expression integration |
| `kani_gate_08_mixed_accessor_paths` | 336-420 | Field+Index chains | Path variety |
| `kani_gate_08_all_node_kinds_no_panic` | 425-431 | All `CompiledNodeKind` variants | No panic |
| `kani_gate_08_constants_with_symbols` | 438-494 | `ConstValue::Symbol` | Constants tolerated |
| `kani_gate_08_many_accessors_varied_depths` | 501-589 | Deep paths, many accessors | Iteration stress |

### 3.3 GOD RULE Compliance Assessment

The comments on lines 227, 258, 333, 435, 498 indicate **prior GOD RULE fixes** where hardcoded structures were replaced with `kani::any()`. This is **correct** — the issue is that the fixes still use raw primitives.

---

## 4. SCOTT WLASCHIN DDD VIOLATIONS

### 4.1 Primitive Obsession (Type-Driven Design Failure)

**Problem:** The harness builders construct domain objects (`WorkflowParts`, `AccessorProgram`, `PathSegment`) but use raw primitives for intermediate values. This creates a "type gap" where:

1. The **implementation** uses proper types (`SlotIdx`, `SymbolId`)
2. The **verification harness** uses raw primitives to *build* the same structures

This means the harnesses could construct **invalid domain objects** if the raw values are out of range, bypassing the type system's guarantees.

**Example - Lines 55-69:**
```rust
Box::new([AccessorProgram {
    root: SlotIdx::new(root),      // root is raw u16
    path: Box::new([PathSegment::Field(SymbolId::new(symbol))]),  // symbol is raw u32
}])
```
If `root >= slot_count`, this creates an invalid `AccessorProgram`. The harness then relies on `kani::assume()` constraints (lines 43-49) to prevent invalid construction, but these are **external to the type**, not enforced by construction.

### 4.2 "Parse, Don't Validate" Violation

**Problem:** `SlotIdx::new(root)` accepts any `u16` without range-checking. The validation happens **later** in `validate_gate_08_accessor_path_segments`. This is the "validate" pattern, not "parse."

**Better approach:** A `SlotIdx::new_checked(root, slot_count)` that returns `Option<SlotIdx>` — building the validated index in one step.

### 4.3 Value Object Missing: `AccessorPath`

The `PathSegment` enum has two variants:
```rust
pub enum PathSegment {
    Field(SymbolId),
    Index(u32),  // <-- primitive obsession
}
```

The `Index` variant should be `Index(AccessorIndex)` where `AccessorIndex` is a newtype that **rejects `u32::MAX`** at construction time. This would make invalid paths **unrepresentable**.

---

## 5. REFACTORING PRESCRIPTION

### 5.1 Required Newtypes (in `vb_core::ids`)

```rust
/// Index into an accessor path. Rejects u32::MAX sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct AccessorIndex(u32);

impl AccessorIndex {
    pub fn new(value: u32) -> Option<Self> {
        if value == u32::MAX { None } else { Some(Self(value)) }
    }
    pub fn get(self) -> u32 { self.0 }
}

/// Count of slots in a workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SlotCount(u16);

impl SlotCount {
    pub fn new(value: u16) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }
    pub fn get(self) -> u16 { self.0 }
}

/// Count of symbols in a workflow's symbol table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SymbolCount(u32);

impl SymbolCount {
    pub fn new(value: u32) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }
    pub fn get(self) -> u32 { self.0 }
}
```

### 5.2 File Splitting Prescription

The 14 harnesses should be organized into **3 files**:

| File | Harnesses | Lines Est. |
|------|-----------|------------|
| `kani_gate_08_base.rs` | Shared harness builders (`arbitrary_parts_with_valid_accessors`, `arb SlotCount`, `arb SymbolCount`, `arb AccessorIndex`, `arb AccessorPathStyle`) | ~120 |
| `kani_gate_08_positive.rs` | Harnesses 1, 5, 6, 7, 8, 9, 12 | ~200 |
| `kani_gate_08_negative.rs` | Harnesses 2, 3, 4, 10, 11, 13, 14 | ~250 |

### 5.3 Module Structure After Refactor

```
vb_validate/src/
├── kani/
│   ├── mod.rs
│   ├── kani_gate_08_base.rs      # NEW: shared builders
│   ├── kani_gate_08_positive.rs  # NEW: happy path
│   └── kani_gate_08_negative.rs  # NEW: rejection path
└── kani_gate_08_structural.rs    # RENAME: re-export or delegate
```

---

## 6. SUMMARY

| Category | Status | Finding Count |
|----------|--------|--------------|
| Line Count | **VIOLATION** | 1 (589 > 300) |
| Primitive Obsession | **VIOLATION** | 3 major + 2 minor |
| DDD Type Gaps | **VIOLATION** | 3 missing newtypes |
| GOD RULE Compliance | **PARTIAL** | Harnesses use `kani::any()` but with raw types |

**Priority Fix Order:**
1. Extract shared harness builder utilities into `kani_gate_08_base.rs` (~120 lines)
2. Split remaining harnesses into positive/negative files
3. Add `AccessorIndex`, `SlotCount`, `SymbolCount` newtypes to `vb_core::ids`
4. Refactor harness builders to use domain types throughout
5. Update `PathSegment::Index` to use `AccessorIndex` (breaking change to impl)

**Architectural Drift Confirmed:** YES
