# Architectural Drift Report: `gate_11.rs`

**File**: `crates/vb_validate/src/gates/gate_11.rs`
**Status**: GUILTY — 443 lines (violates <300 line rule by +143 lines / +48%)
**Date**: 2026-05-29

---

## Executive Summary

Gate 11 validates loop body graph structure — entry points, next/error handlers, loop span correctness, and pairing relationships between loop start/continuation/finish nodes. The code is **repetitive to the point of being mechanical** — eight `is_matching_*` and `is_*_done` functions that differ only in which `CompiledNodeKind` variant they match. This is **pure mechanical duplication** begging to be replaced with a single parameterized function or a proper enum-based domain type.

---

## Violation 1: LINE COUNT (443 > 300)

| Metric | Value | Limit | Overage |
|--------|-------|-------|---------|
| Total lines | 443 | 300 | +143 (+48%) |
| Pure duplication (is_matching*/is_*_done) | 70 lines | - | 16% of file |
| Check functions (check_step/check_next/check_loop/check_together) | 87 lines | - | 20% of file |
| Remaining meaningful logic | ~286 lines | - | - |

**The file cannot be shippable as-is.**

---

## Violation 2: Primitive Obsession — `as_usize()` Plague

### Offenders (18 occurrences):

```
Line 19:  next.as_usize() > node_count
Line 22:  on_error.as_usize() >= node_count
Line 26:  body.as_usize() >= node_count
Line 27:  done.as_usize() >= node_count
Line 32:  body.as_usize() >= node_count
Line 33:  done.as_usize() >= node_count
Line 37:  branch.as_usize() >= node_count
Line 43:  join.as_usize() >= node_count
Line 47:  entry.as_usize() >= node_count
Line 48:  join.as_usize() >= node_count
Line 52:  body.as_usize() >= node_count
Line 53:  done.as_usize() >= node_count
Line 57:  body.as_usize() >= node_count
Line 58:  done.as_usize() >= node_count
Line 61:  body.as_usize() >= node_count
Line 62:  done.as_usize() >= node_count
Line 65:  body.as_usize() >= node_count
Line 66:  done.as_usize() >= node_count
Line 70:  body.as_usize() >= node_count
Line 71:  done.as_usize() >= node_count
Line 74:  body.as_usize() >= node_count
Line 75:  done.as_usize() >= node_count
Line 79:  body.as_usize() >= node_count
Line 80:  done.as_usize() >= node_count
Line 83:  done.as_usize() >= node_count
Line 88:  body.as_usize() >= node_count
Line 89:  exhausted.as_usize() >= node_count
Line 92:  body.as_usize() >= node_count
Line 93:  handler.as_usize() >= node_count
Line 117: body.as_usize() >= node_count
Line 118: done.as_usize() >= node_count
Line 126: done.as_usize() == index
Line 135: body.as_usize() >= node_count
Line 136: done.as_usize() >= node_count
Line 145: body.as_usize() >= node_count
Line 146: done.as_usize() >= node_count
Line 154: done.as_usize() == index
Line 157: body.as_usize() >= node_count
Line 158: done.as_usize() >= node_count
Line 165: done.as_usize() == index
Line 168: done.as_usize() == index
Line 202: done.as_usize() == done
Line 206: done.as_usize() == done
Line 207: step_in_loop_body(index, *body, *start_done)
Line 226: target.as_usize() == index
Line 245: join.as_usize() == index
Line 269: body.as_usize()
Line 270: done.as_usize()
Line 271: index >= body_index && index < done_index
Line 362: step.as_usize() >= node_count
Line 363: step.as_usize() >= node_count
Line 364: step.as_usize()
Line 378: step.as_usize() > node_count
Line 379: step.as_usize() > node_count
Line 380: step.as_usize()
Line 395: body.as_usize()
Line 396: done.as_usize()
Line 397: body_usize <= start_index
Line 405: done_usize <= body_usize
Line 422: join.as_usize()
Line 423: branch.as_usize()
Line 425: branch_usize <= start_index
Line 433: join_usize <= branch_usize
```

**~55 `as_usize()` calls.** This is a textbook case of "we have typed indices but extract them constantly."

### Root Cause

`StepIdx` exists in `vb_core::ids` but validation code pulls out the raw `usize` value repeatedly. The domain type has no `InBounds` or `CheckedAdd` or even a simple bounds-check method.

### Fix Required

Implement **`InBounds`** trait on `StepIdx`:

```rust
pub trait InBounds {
    fn in_bounds(&self, max: usize) -> bool;
    fn bounds_check(&self, max: usize) -> ValidationResult<()>;
}

impl InBounds for StepIdx {
    fn in_bounds(&self, max: usize) -> bool {
        self.as_usize() < max
    }
    fn bounds_check(&self, max: usize) -> ValidationResult<()> {
        if self.in_bounds(max) {
            Ok(())
        } else {
            Err(ValidationError::LoopBodyStepOutOfRange { ... })
        }
    }
}
```

Then every `step.as_usize() >= node_count` becomes `step.in_bounds(node_count)`.

---

## Violation 3: Mechanical Duplication — 8 Copy-Paste Functions

### Offenders (lines 284–354):

```rust
// These 8 functions are IDENTICAL except for which CompiledNodeKind variant they match.
fn is_matching_for_each_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool
fn is_matching_collect_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool
fn is_matching_reduce_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool
fn is_matching_repeat_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool

fn is_foreach_start_done(kind: &CompiledNodeKind, index: usize) -> bool
fn is_collect_start_done(kind: &CompiledNodeKind, index: usize) -> bool
fn is_reduce_start_done(kind: &CompiledNodeKind, index: usize) -> bool
fn is_repeat_start_done(kind: &CompiledNodeKind, index: usize) -> bool
```

All 8 follow the same `matches!(kind, CompiledNodeKind::XxxStart { field, .. } if ...)` pattern.

### Scott Wlaschin Says

> "Duplication is far cheaper than the wrong abstraction." — Sandi Metz

But this isn't abstraction — this is mechanical copy-paste with zero abstraction. The right fix is a **single parameterized function**:

```rust
fn matching_loop_start(
    kind: &CompiledNodeKind,
    body: StepIdx,
    done: StepIdx,
    variant: LoopVariant,
) -> bool {
    match (kind, variant) {
        (CompiledNodeKind::ForEachStart { body: b, done: d, .. }, LoopVariant::ForEach)
            if *b == body && *d == done => true,
        (CompiledNodeKind::CollectStart { body: b, done: d, .. }, LoopVariant::Collect)
            if *b == body && *d == done => true,
        // ... etc
        _ => false,
    }
}

enum LoopVariant { ForEach, Collect, Reduce, Repeat }
```

Or even better: a **visitor-style `LoopBodyPairing`** domain type that encapsulates the pairing logic.

---

## Violation 4: Near-Duplicate `require_matching_body_start` and `require_matching_done_start`

Lines 174–195: These two functions are 90% identical:

```rust
fn require_matching_body_start(
    parts: &WorkflowParts,
    index: usize,
    body: StepIdx,
    done: StepIdx,
    label: &str,
    start_matches: fn(&CompiledNodeKind, StepIdx, StepIdx) -> bool,
) -> ValidationResult<()> {
    let has_match = step_in_loop_body(index, body, done)
        && has_prior_matching_start(parts, index, |kind| start_matches(kind, body, done));
    require_pairing(has_match, index, format!("{label} has no matching start"))
}

fn require_matching_done_start(
    parts: &WorkflowParts,
    index: usize,
    label: &str,
    start_done_matches: fn(&CompiledNodeKind, usize) -> bool,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| start_done_matches(kind, index));
    require_pairing(has_match, index, format!("{label} has no matching start"))
}
```

The difference is `step_in_loop_body` check — this should be a **single function** with a `bool check_body_span` parameter, or better, a proper domain type `LoopPairing` with a `validate` method.

---

## Violation 5: God Function — `validate_gate_11_loop_body_graph`

Lines 11–100: 90 lines that handle entry check, node iteration with all node kinds, and loop pairings in a single function.

### What it actually does:

1. Check entry is in range (3 lines)
2. For each node: check next, on_error, node-kind-specific indices, loop spans (60+ lines match block)
3. Validate pairings (1 line)

This is **three responsibilities** in one function. Extract:

```
gate_11/
├── mod.rs                    (reexports, orchestration)
├── entry_validation.rs       (check_entry_in_range)
├── index_validation.rs       (check_step_in_range, check_next_step_in_range)
├── span_validation.rs        (check_loop_span, check_together_span)
├── pairing_validation.rs    (validate_loop_pairings, all is_matching_* and is_*_done)
└── shared.rs                 (InBounds trait, LoopVariant enum, require_pairing)
```

Each file: **<60 lines**.

---

## Violation 6: Raw Function Pointers Instead of Domain Types

Lines 180, 191:

```rust
start_matches: fn(&CompiledNodeKind, StepIdx, StepIdx) -> bool,
start_done_matches: fn(&CompiledNodeKind, usize) -> bool,
```

These `fn` pointers are the mechanical bridge between the duplicated functions. Replace with an **enum-based strategy**:

```rust
enum LoopPairingPredicate {
    BodyMatches(fn(&CompiledNodeKind, StepIdx, StepIdx) -> bool),
    DoneMatches(fn(&CompiledNodeKind, usize) -> bool),
}
```

Or better: a **`LoopPairingRule`** trait implemented by each loop variant:

```rust
trait LoopPairingRule {
    fn body_matches(&self, kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool;
    fn done_matches(&self, kind: &CompiledNodeKind, index: usize) -> bool;
}
```

---

## Summary of Required Fixes

| # | Issue | Severity | Fix |
|---|-------|----------|-----|
| 1 | 443 lines | **CRITICAL** | Split into `gate_11/` dir module |
| 2 | ~55× `as_usize()` | **HIGH** | Add `InBounds` trait to `StepIdx` |
| 3 | 8 copy-paste functions | **HIGH** | Single parameterized `matching_loop_start` + `LoopVariant` enum |
| 4 | Near-duplicate requires | **MEDIUM** | Single `require_pairing_check` with bool param |
| 5 | God function | **MEDIUM** | Extract entry, index, span, pairing validators |
| 6 | Raw fn pointers | **MEDIUM** | `LoopPairingRule` trait |

---

## Suggested Module Layout

```
crates/vb_validate/src/gates/gate_11/
├── mod.rs                    # Reexports + validate_gate_11_loop_body_graph orchestrator
├── entry_validation.rs      # check_entry_in_range — 10 lines
├── index_validation.rs       # check_step_in_range, check_next_step_in_range — 30 lines
├── span_validation.rs        # check_loop_span, check_together_span — 50 lines
├── pairing_validation.rs     # LoopVariant enum, matching functions, require_pairing — 80 lines
└── shared.rs                 # InBounds trait for StepIdx — 20 lines
```

**Total: ~190 lines across 5 files, each <80 lines.**

---

## Verification Command

```bash
# After refactoring:
moon ci
# OR
cargo test -p vb_validate -- gate_11
cargo clippy -p vb_validate
```

---

## Conclusion

**Hammer applied.** This file is a structural offense — 48% over the line limit, pure mechanical duplication, and constant `.as_usize()` extraction despite `StepIdx` being a perfectly good domain type. The duplication is the most offensive part: eight `is_matching_*` functions that are literally identical except for the struct field name. This is not abstraction — this is copy-paste that a first-year programmer should be embarrassed to submit. Fix it.

---
