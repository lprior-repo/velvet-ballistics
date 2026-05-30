# Architectural Drift Report: `gate_10_node.rs`

**File**: `crates/vb_validate/src/gate_10_node.rs`
**Status**: GUILTY — 747 lines (violates <300 line rule)
**Date**: 2026-05-29

---

## Executive Summary

This file is a **PRIMITIVE OBSESSION SAMPLER** masquerading as validation logic. Every line is a testament to the sin of avoiding domain types. The phrase `as_usize()` appears **26 times** — a code smell loud enough to wake the dead.

---

## Violation 1: LINE COUNT (747 > 300)

| Metric | Value | Limit | Overage |
|--------|-------|-------|---------|
| Total lines | 747 | 300 | +447 (+149%) |
| Production code | ~230 | - | - |
| Test code | ~517 | - | - |

**The file cannot be shippable. It must be carved up.**

---

## Violation 2: Primitive Obsession — `as_usize()` Plague

### Offenders (26 occurrences):

```
Line 13:  let slot_count = usize::from(parts.slot_count);
Line 14:  parts.constants.len()         → usize (no conversion needed but raw)
Line 15:  parts.accessors.len()         → usize (raw)
Line 16:  parts.expressions.len()       → usize (raw)
Line 17:  parts.nodes.len()             → usize (raw)
Line 25:  result.as_usize() >= slot_count
Line 39:  branch.condition.as_usize() >= expr_count
Line 47:  branch.target.as_usize() >= node_count
Line 57:  o.as_usize() >= node_count
Line 72:  branch.condition.as_usize() >= slot_count
Line 80:  branch.target.as_usize() >= node_count
Line 90:  o.as_usize() >= node_count
Line 101: value.as_usize() >= const_count
Line 111: expr.as_usize() >= expr_count
Line 121: input.as_usize() >= slot_count
Line 127: action.get() == u16::MAX  ← SENTINEL MAGIC NUMBER
Line 141: input.as_usize() >= slot_count
Line 149: item_slot.as_usize() >= slot_count
Line 157: body.as_usize() >= node_count
Line 165: done.as_usize() >= node_count
Line 176: branch.as_usize() >= node_count
Line 185: join.as_usize() >= node_count
Line 196: symbol.get() >= symbols_count
Line 205: slot.as_usize() >= slot_count
Line 217: slot.as_usize() >= slot_count
Line 260: let const_usize = value.as_usize();
Line 277: let accessor_usize = accessor.as_usize();
```

### Root Cause

`ConstIdx`, `AccessorIdx`, `SlotIdx`, `StepIdx`, `ExprIdx` all exist in `vb_core::ids` but validation is performed on **raw integer values after extraction**. This is textbook primitive obsession: we have typed indices but treat them as dumb integers.

### Fix Required

Introduce a **`Bounds<T>`** value object:

```rust
pub struct Bounds<T> {
    pub index: T,
    pub max: usize,
}

impl<T: AsUsize> Bounds<T> {
    pub fn check(&self) -> ValidationResult<()> {
        if self.index.as_usize() >= self.max {
            Err(/* ... */)
        } else {
            Ok(())
        }
    }
}
```

Or better: implement **`InBounds<T>`** trait on all index types with a blanket `check(&self, max: usize)` method.

---

## Violation 3: God Function — `validate_gate_10_node_kind_specific`

**Lines 12–231**: 219 lines. Handles **11+ node kinds** in a single match statement with repetitive boundary checks.

### Crime

```rust
pub fn validate_gate_10_node_kind_specific(parts: &WorkflowParts) -> ValidationResult<()> {
    // ... setup ...
    for (node_index, node) in parts.nodes.iter().enumerate() {
        match &node.kind {
            CompiledNodeKind::Finish { result } => { /* 8 lines */ }
            CompiledNodeKind::Choose { branches, otherwise } => { /* 29 lines */ }
            CompiledNodeKind::ChooseSlot { branches, otherwise } => { /* 29 lines */ }
            CompiledNodeKind::SetConst { value } => { /* 8 lines */ }
            CompiledNodeKind::EvalExpr { expr } => { /* 8 lines */ }
            CompiledNodeKind::Do { action, input } => { /* 12 lines */ }
            CompiledNodeKind::ForEachStart { input, item_slot, body, done } => { /* 33 lines */ }
            CompiledNodeKind::TogetherStart { branches, join } => { /* 19 lines */ }
            CompiledNodeKind::BuildObject { fields } => { /* 19 lines */ }
            CompiledNodeKind::BuildList { items } => { /* 10 lines */ }
            _ => {}
        }
    }
}
```

### Scott Wlaschin Says

> "A function should do one thing. If it does multiple things, you need multiple functions."

This function does **one thing** (validates node-kind constraints) but handles **10 subtypes** with identical boundary-check patterns. The pattern is:

```
if index.as_usize() >= bound { return Err(...) }
```

### Prescribed Refactoring

Split into per-kind validators:

```
gate_10_node/
  ├── mod.rs          (reexports, orchestration)
  ├── finish.rs       (validate_finish_node)
  ├── choose.rs       (validate_choose_node)
  ├── choose_slot.rs  (validate_choose_slot_node)
  ├── set_const.rs    (validate_set_const_node)
  ├── eval_expr.rs    (validate_eval_expr_node)
  ├── do.rs           (validate_do_node)
  ├── for_each.rs     (validate_for_each_node)
  ├── together.rs     (validate_together_node)
  ├── build_object.rs (validate_build_object_node)
  ├── build_list.rs   (validate_build_list_node)
  └── shared.rs       (InBounds trait, Bounds util)
```

Each file: **<60 lines**.

---

## Violation 4: Test Code Mixed with Production Code

**Lines 289–747**: 459 lines of tests **inline** in the production module.

### Problem

- Tests should be in `crates/vb_validate/tests/gate_10_node_test.rs` or similar
- `#[cfg(test)] mod tests` inside a lib module is acceptable for unit tests of private helpers
- But 459 lines of test for a 230-line module is a **63%/37% production-to-test ratio** — inverted
- The `make_parts` helper is copy-paste bait if other modules need similar scaffolding

### Fix

Move tests to `tests/` directory. Keep only trivial internal test helpers (if any) in the module.

---

## Violation 5: Magic Numbers

| Location | Magic | Should Be |
|----------|-------|-----------|
| Line 127 | `action.get() == u16::MAX` | `ActionId::is_sentinel()` method |

**Sentinel values should be encapsulated in the type, not checked at use sites.**

---

## Summary of Required Fixes

| # | Issue | Severity | Fix |
|---|-------|----------|-----|
| 1 | 747 lines | **CRITICAL** | Split into `gate_10_node/` dir module |
| 2 | 26× `as_usize()` | **HIGH** | Add `InBounds` trait to index types |
| 3 | God function | **HIGH** | Extract per-kind validators |
| 4 | Tests inline | **MEDIUM** | Move to `tests/` directory |
| 5 | Magic sentinel | **MEDIUM** | `ActionId::is_sentinel()` |

---

## Verification Command

```bash
# After refactoring:
moon ci
# OR
cargo test -p vb_validate -- gate_10
cargo clippy -p vb_validate
```

---

## Conclusion

** Hammer applied.** This file is a structural crime scene. The primitive obsession is so severe it suggests the index types were designed but never integrated — validation code was written as if `ConstIdx` and `SlotIdx` don't exist. Fix the type integration first, then split the file.
