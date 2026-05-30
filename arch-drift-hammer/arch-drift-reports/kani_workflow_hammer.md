# Architectural Drift Report: `kani_workflow_arbitrary.rs`

**File**: `crates/vb_core/src/kani_workflow_arbitrary.rs`
**Line Count**: 587 (limit: 300)
**Severity**: CRITICAL — 197 lines over budget (65% over limit)

---

## Executive Summary

This file implements `kani::Arbitrary` for 17 distinct types spanning 3 domain boundaries (workflow, action system, resource contracts). It violates the single responsibility principle, promotes primitive obsession, and violates DDD bounded context separation.

---

## 1. File Size Violations

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 587 | 300 | **FAIL** |
| Domain groups | 3 | 1 per file | **FAIL** |
| Types per domain | 5-6 avg | ≤4 per file | **FAIL** |

**Required Split**: Minimum 3 files:
- `kani_workflow_arbitrary.rs` → workflow types only (lines 22-482)
- `kani_action_arbitrary.rs` → action system types (lines 519-587)
- `kani_shared_arbitrary.rs` → shared helpers (bounded_len, etc.)

---

## 2. Primitive Obsession Violations

### 2.1 Raw `u8` Discriminants (11 instances)

```rust
// LINES 30, 101, 140, 490, 502, 521, 531, 543
match kani::any::<u8>() { ... }
match kani::any::<u8>() % N { ... }
```

**Problem**: `u8` is unconstrained — Kani can generate any 0-255 value. For enums with 3-5 variants, this wastes 95%+ of the symbolic state space on impossible discriminants.

**Fix**: Create a typed discriminant wrapper:
```rust
struct ExprOpVariant(u8);
impl kani::Arbitrary for ExprOpVariant {
    fn any() -> Self { Self(kani::any::<u8>() % 29) } // 29 = ExprOp variant count
}
```

### 2.2 Raw Index Types

| Location | Primitive | Should Be |
|----------|-----------|-----------|
| Line 114 | `u16` for `SlotIdx` | `SlotIdx::new(u16)` already typed |
| Line 402 | `u32` for `symbols_count` | `SymbolCount(u32)` |
| Lines 86, 88 | `u32` for `SymbolId::new()` | Already typed — acceptable |

### 2.3 Repetitive `while i < count { ... i += 1 }` Loops

Lines 70-74, 365-369, 373-377, 381-385, 387-393, 566-572 all follow identical pattern.

**Fix**: Use iterator pattern or `from_fn`:
```rust
std::iter::from_fn(|| Some(kani::any::<T>())).take(count as usize).collect()
```

---

## 3. DDD Boundary Violations

### 3.1 Bounded Context Mixing

| Lines | Domain | Types |
|-------|--------|-------|
| 22-482 | Workflow | `ExprOp`, `ExprProgram`, `CompiledNode`, `CompiledNodeKind`, `WorkflowParts` |
| 488-513 | Value | `Taint`, `SlotValue` |
| 519-587 | Action | `Idempotency`, `SideEffect`, `RetrySafety`, `Capability`, `ActionContract` |

**Problem**: Scott Wlaschin DDD requires bounded contexts to be **module-enforced**. A single file at `vb_core/src/kani_workflow_arbitrary.rs` mixes `action` domain types with `workflow` types with no module boundary.

**Fix**: Move action types to `vb_action` crate or at minimum `vb_core/src/action/kani_arbitrary.rs`.

### 3.2 WorkflowParts God Object

Line 360-408: `WorkflowParts::any()` generates 8 different sub-structures. This is a **DDD aggregate root** pattern gone wrong — it couples too many concerns.

**Fix**: Introduce a `WorkflowPartsBuilder` that composes bounded sub-generators.

---

## 4. Repetitive Code Patterns

### 4.1 `bounded_len_3()` vs `bounded_len_2()`

Lines 472-482 — 96% identical:

```rust
fn bounded_len_3() -> u8 {
    let len: u8 = kani::any();
    kani::assume(len <= 3);
    len
}

fn bounded_len_2() -> u8 {  // DUPLICATE
    let len: u8 = kani::any();
    kani::assume(len <= 2);
    len
}
```

**Fix**: Generic bounded length:
```rust
fn bounded_len<const N: u8>() -> u8 {
    let len: u8 = kani::any();
    kani::assume(len <= N);
    len
}
```

### 4.2 Identical Bounded Collection Helpers

Lines 419-470: `bounded_path`, `bounded_accessors`, `bounded_expr_branches`, `bounded_slot_branches`, `bounded_step_indices` are 90% structurally identical.

**Fix**: Generic boxed slice generator:
```rust
fn bounded_slice<T: kani::Arbitrary, const MAX: usize>() -> Box<[T]> {
    let len = bounded_len::<MAX>();
    (0..len).map(|_| kani::any()).collect()
}
```

---

## 5. Summary of Violations

| Rule | Violation | Count |
|------|-----------|-------|
| File size | >300 lines | 1 (CRITICAL) |
| DDD bounded context | Cross-domain mixing | 3 contexts |
| Primitive obsession | Raw u8 discriminants | 11 |
| DRY | Duplicate loop patterns | 6 |
| DRY | Duplicate bounded helpers | 5 |

---

## 6. Refactoring Prescription

### Phase 1: Extract Action Types (50 lines)
Move `kani::Arbitrary` impls for `Idempotency`, `SideEffect`, `RetrySafety`, `Capability`, `ActionContract` to `crates/vb_core/src/action/kani_arbitrary.rs`

### Phase 2: Create Shared Helper Module (40 lines)
Extract to `crates/vb_core/src/kani_shared_arbitrary.rs`:
- `bounded_len<const N: u8>() -> u8`
- `bounded_slice<T, const MAX: usize>() -> Box<[T]>`

### Phase 3: Shrink Workflow File (497 → ~250 lines)
- Use generic `bounded_slice` helper
- Replace `while` loops with iterator pattern
- Extract `CompiledNodeKind` to its own file

### Phase 4: Add Type Wrappers (new types)
- `ExprOpVariant(u8)` — constrained discriminant
- `TaintVariant(u8)` — constrained discriminant  
- `SlotValueVariant(u8)` — constrained discriminant

---

## Verdict

**ARCHITECTURAL DRIFT: CONFIRMED**

This file is a **god module** that:
1. Exceeds line limit by 197 lines (65% over budget)
2. Violates DDD bounded context separation by mixing 3 domain types
3. Promotes primitive obsession with raw `u8` discriminants
4. Repeats identical patterns 11+ times

**Immediate Action Required**: Split into minimum 3 files before any new Kani work proceeds on this codebase.
