# Architectural Drift Report: `kani_choose_replay.rs`

**File:** `crates/vb_core/src/verification/kani/kani_choose_replay.rs`
**Total Lines:** 340
**Status:** OVER LIMIT — 40 lines excess
**Date:** 2026-05-29

---

## Executive Summary

This Kani proof harness file violates the **<300 line rule** by 40 lines and exhibits multiple **primitive obsession** violations that weaken the TLA+ bridge contract. The file provides Kani proofs for `replay_choose_slot` (PO-vb282my-CR-KANI-001 through 006) but does so with naked magic numbers scattered throughout, making the verification obligations harder to audit and maintain.

---

## Violation 1: File Size (CRITICAL)

| Metric | Value |
|--------|-------|
| Actual | 340 lines |
| Limit  | 300 lines |
| Excess | 40 lines |

**Required Action:** Split this file into a harness module with shared builder utilities plus per-obligation harness files.

---

## Violation 2: Primitive Obsession (HIGH)

### 2.1 Magic Numbers in Bounded Generators (lines 28–50)

```rust
fn any_run_frame(slot_count: u16, step_count: u16) -> RunFrame {
    let run_id = RunId::new(kani::any::<u64>());
    match RunFrame::new(run_id, StepIdx::new(0), step_count, slot_count) { ... }
}
```

**Problem:** `slot_count` and `step_count` are passed as raw `u16`. These are not validated domain-typed parameters but unconstrained primitives. Any caller can pass `0` or `u16::MAX`, creating silent invalid states.

**Missing domain type:** `FrameBounds { max_slots: u16, max_steps: u16 }` or at minimum a `ValidSlotCount(u16)` newtype.

### 2.2 Magic Slot Count `16` (lines 60, 123, 169, 213, 257)

```rust
let slot_count: u16 = 16;
```

**Problem:** Appears in 5 of 6 proofs with no named constant. The value `16` has no explanation. Compare to production `replay/choose/mod.rs:12` which uses `branches.len()` without a slot count assumption.

**Missing constant:** `const DEFAULT_MAX_SLOTS: u16 = 16;`

### 2.3 Magic Step Count `200` (lines 61, 124, 170, 214, 303)

```rust
let step_count: u16 = 200;
```

**Problem:** Appears in 5 proofs. No explanation of why 200. In `kani_choose_replay_index_safety` (line 303), `step_count: u16 = 200` but the branch targets use `100 + i` which maxes at 163 — well within 200 but the headroom is unjustified.

**Missing constant:** `const DEFAULT_MAX_STEPS: u16 = 200;`

### 2.4 Branch Limit `64` (lines 77, 89, 133, 179, 267, 274, 313)

```rust
kani::assume(branches.len() <= 64);
```

**Problem:** Appears in all 6 proofs. The value `64` is the `TOGETHER_BRANCH_LIMIT` (per `errors.rs:570`). This semantic link is invisible in the harness — a reader cannot tell if `64` is an arbitrary round number or a domain constraint.

**Missing constant:** `pub const BRANCH_LIMIT: usize = 64;` imported from the errors module, not re-declared inline.

### 2.5 Magic Unwind Bounds (lines 58, 121, 168, 211, 255, 300)

```rust
#[kani::unwind(65)]  // vs loop bound of 64 — one extra
#[kani::unwind(70)]  // arbitrary +6 headroom
```

**Problem:** `65` for 64-bound loops gives exactly 1 extra. `70` for `slot_count: 128` gives no clear justification. These are empirical fudge factors, not derived constants.

**Missing constant:** `const UNWIND_FOR_64_BRANCHES: u32 = 65;` or a computed unwind based on `BRANCH_LIMIT + 1`.

### 2.6 Raw Casts in Branch Construction (lines 318–319)

```rust
condition: SlotIdx::new(u16::from(i)),
target: StepIdx::new(u16::from(100 + i)),
```

**Problem:** `u16::from(u8)` is unnecessary — `SlotIdx::new(i)` on a `u8` would compile if `StepIdx::new` accepts `u8`. The explicit `u16::from` suggests the types don't coerce naturally, hinting at a primitive obsession issue upstream in `SlotIdx::new`.

### 2.7 OOB Check Uses Raw Comparison (line 281)

```rust
if condition.get() >= slot_count {
```

**Problem:** `condition.get()` returns a raw `u16`. The comparison `>= slot_count` uses the same magic `slot_count: u16 = 16`. This is readable but the semantic intent (OOB vs valid) is lost in primitive comparison.

---

## Violation 3: DDD Cohesion — Repetitive Proof Structure

All 6 proofs follow an identical 4-phase pattern:

```
Phase 1: any_run_frame + write_slot loop
Phase 2: construct branches (with kani::any + assume)
Phase 3: call replay_choose_slot
Phase 4: match + assert/cover
```

This violates the **Bounded Context** principle — the repetition suggests a missing harness builder/abstraction layer. Each proof should not be a flat sequence of setup code; the common frame-initialization and slot-writing logic belongs in a shared `TestFixture` builder.

**Recommended split:**
```
verification/kani/
  choose_replay_harness.rs   # Shared fixture builder (<100 lines)
  kani_choose_true_branch.rs
  kani_choose_otherwise.rs
  kani_choose_no_match.rs
  kani_choose_non_bool.rs
  kani_choose_slot_unavailable.rs
  kani_choose_index_safety.rs
```

---

## Kani Choose Replay Responsibility Map

| Obligation | Target Behavior | Harness Entry Point | Lines |
|------------|-----------------|---------------------|-------|
| PO-vb282my-CR-KANI-001 | True branch selected → Continue(target) | `kani_choose_replay_true_branch` | 59–113 |
| PO-vb282my-CR-KANI-002 | All false + otherwise → Continue(fallback) | `kani_choose_replay_otherwise_fallback` | 122–160 |
| PO-vb282my-CR-KANI-003 | All false + no otherwise → Err(Internal) | `kani_choose_replay_no_match` | 169–203 |
| PO-vb282my-CR-KANI-004 | Non-Bool slot → Err(Internal) | `kani_choose_replay_non_bool_condition` | 212–247 |
| PO-vb282my-CR-KANI-005 | OOB/uninit slot → Err(SlotNotAvailable) | `kani_choose_replay_slot_not_available` | 256–292 |
| PO-vb282my-CR-KANI-006 | Index overflow safety | `kani_choose_replay_index_safety` | 301–340 |

---

## God Rule Compliance

| Rule | Status | Notes |
|------|--------|-------|
| GR1: All inputs use kani::any() with bounded assumptions | **PARTIAL** | `kani::any::<u64>()` for RunId is unbounded (could be u64::MAX). `kani::any::<u16>()` for slot/step indices are bounded via `kani::assume()` correctly. |
| GR2: Calls actual production replay_choose_slot | **PASS** | Directly calls `replay::choose::replay_choose_slot` from production module. |

---

## Summary of Required Fixes

| # | Severity | Issue | Fix |
|---|----------|-------|-----|
| 1 | CRITICAL | 340 lines > 300 line limit | Split into module + per-obligation files |
| 2 | HIGH | Magic `16` for slot_count | Extract `const DEFAULT_MAX_SLOTS: u16 = 16;` |
| 3 | HIGH | Magic `200` for step_count | Extract `const DEFAULT_MAX_STEPS: u16 = 200;` |
| 4 | HIGH | Magic `64` for branch limit | Import `TOGETHER_BRANCH_LIMIT` from errors module |
| 5 | HIGH | Magic unwind bounds `65`, `70` | Derive from `BRANCH_LIMIT + 1` or use named constants |
| 6 | MEDIUM | Raw `u16::from(i)` casts | Investigate if `SlotIdx::new(u8)` should exist |
| 7 | MEDIUM | Unbounded `kani::any::<u64>()` for RunId | Add `kani::assume(run_id.raw() != u64::MAX)` or bound to valid range |
| 8 | MEDIUM | Repetitive 4-phase proof structure | Introduce `TestFixture` builder for common setup |

---

## Drift Classification

**Type:** Structural + Primitive Obsession
**Scope:** Single file, self-contained
**Blast Radius:** Low — Kani harness only, no production code affected
**Remediation Difficulty:** Low — primarily extraction and constant naming, no algorithmic changes

---

*Report generated by arch-drift-hammer enforcer. Next action: Split file, extract constants, create harness builder module.*
