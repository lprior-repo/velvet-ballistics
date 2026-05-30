# ARCHITECTURAL DRIFT HAMMER REPORT
## Target: `vb_runtime/src/verification/kani/kani_ask_answer_lifecycle.rs`
## Severity: CRITICAL
## Date: 2026-05-29

---

## EXECUTIVE SUMMARY

**File Status**: 320 lines — EXCEEDS 300-line limit by 20 lines (VIOLATION)
**File Type**: Kani verification harness (proof obligations PO-vb282my-AA-KANI-001 through 006)
**Bead**: vb-282my

This file commits **6 primitive obsession violations** and **1 file-size violation**, enabling God Rule 1 violations in the test infrastructure.

---

## VIOLATION #1: FILE SIZE — 320 LINES (>300 LIMIT)

**Location**: Entire file (320 lines)
**Category**: File-size governance violation
**Impact**: Exceeds mandatory 300-line architectural limit by 20 lines (6.7% overage)

**Required Action**: Split into focused harness modules per obligation group:
- `kani_ask_answer_001_append_before_insert.rs`
- `kani_ask_answer_002_append_failure.rs`
- `kani_ask_answer_003_pending_timer_guard.rs`
- `kani_ask_answer_004_005_slot_written_ordering.rs` (004+005 share lifecycle state)
- `kani_ask_answer_006_journal_monotonicity.rs`

---

## VIOLATION #2: PRIMITIVE OBSESSION — Raw `u16` StepIdx

**Location**: Lines 145-148
**Current Code**:
```rust
let ask_step: u16 = kani::any();
kani::assume(ask_step < 32);
let timer_step: u16 = kani::any();
kani::assume(timer_step < 32);
```

**Domain Type Exists**: `vb_core::ids::StepIdx` with `kani::Arbitrary` impl (see `vb_core/src/ids/kani_id_arbitrary.rs:28-32`)

**Required Fix**:
```rust
let ask_step: StepIdx = kani::any();
let timer_step: StepIdx = kani::any();
// StepIdx invariants handled by constructor, not assume()
```

**God Rule Violation**: GOD RULE 1 states "All inputs use kani::any()". The primitive `u16` bypasses the domain-validated `StepIdx` constructor. This proves nothing about the actual `StepIdx` type used in production.

---

## VIOLATION #3: PRIMITIVE OBSESSION — Raw `u64` EventSeq

**Location**: Line 290
**Current Code**:
```rust
let raw_seq: u64 = kani::any();
let seq = EventSeq::new(raw_seq);
```

**Domain Type Exists**: `vb_core::ids::EventSeq` with `kani::Arbitrary` impl (see `vb_core/src/ids/kani_id_arbitrary.rs:22-26`)

**Required Fix**:
```rust
let seq: EventSeq = kani::any();
```

**God Rule Violation**: GOD RULE 1. The harness bypasses `EventSeq`'s domain invariants by using raw `u64`.

---

## VIOLATION #4: PRIMITIVE OBSESSION — Raw `u64::MAX` Overflow Test

**Location**: Lines 299-306
**Current Code**:
```rust
let max_seq = EventSeq::new(u64::MAX);
let max_get = max_seq.get();
let overflow = max_get.checked_add(1);
```

**Problem**: Uses raw `u64::MAX` instead of `EventSeq` boundary. The `EventSeq::new()` constructor may already clamp or reject `u64::MAX` — this test bypasses that invariant.

**Required Fix**:
```rust
// Test EventSeq overflow behavior through domain API
let max_seq = EventSeq::new(u64::MAX);
// Overflow behavior should be tested via checked_add on EventSeq type directly
let overflow = max_seq.get().checked_add(1);
```

**God Rule Violation**: GOD RULE 1 — tests the raw `u64` representation, not `EventSeq`'s actual boundary behavior.

---

## VIOLATION #5: PRIMITIVE OBSESSION — Raw `u64` for Monotonicity

**Location**: Lines 309-318
**Current Code**:
```rust
let low_raw: u64 = kani::any();
kani::assume(low_raw < u64::MAX);
let next = low_raw.checked_add(1);
```

**Required Fix**:
```rust
let low_seq: EventSeq = kani::any();
kani::assume(low_seq.get() < u64::MAX);
let next = low_seq.get().checked_add(1);
```

**God Rule Violation**: GOD RULE 1 — bypasses `EventSeq` domain type entirely.

---

## VIOLATION #6: PRIMITIVE OBSESSION — `kani::any::<u64>()` for RunId

**Location**: Lines 28-30
**Current Code**:
```rust
fn any_run_id() -> RunId {
    RunId::new(kani::any::<u64>())
}
```

**Domain Type Available**: `RunId: kani::Arbitrary` (see `vb_core/src/ids/kani_id_arbitrary.rs:10-14`)

**Required Fix**:
```rust
fn any_run_id() -> RunId {
    kani::any()
}
```

**God Rule Violation**: GOD RULE 1 — `kani::any::<u64>()` is not `kani::any::<RunId>()`.

---

## VIOLATION #7: MISSING DOMAIN TYPE COVERAGE — StepIdx at Line 87

**Location**: Line 87
**Current Code**:
```rust
step: vb_core::ids::StepIdx::new(0),
```

**Correct Usage Pattern**: Should use `kani::any::<StepIdx>()` for property-based coverage.

**Contrast with Correct Pattern in Same File**: Line 29 uses `RunId::new(kani::any::<u64>())` — inconsistent with `vb_core/src/ids/kani_id_arbitrary.rs` which provides `impl kani::Arbitrary for RunId`.

---

## SCOTT WLASCHIN DDD ASSESSMENT

### Primitive Obsession (Primitives instead of Value Objects)
**Severity**: CATASTROPHIC

The file defines `any_run_id()` as a wrapper that defeats the purpose of `kani::Arbitrary`:

```rust
fn any_run_id() -> RunId {
    RunId::new(kani::any::<u64>())  // ❌ Primitive obsession
}
```

Compare to correct pattern in `kani_id_arbitrary.rs:10-14`:
```rust
impl kani::Arbitrary for RunId {
    fn any() -> Self {
        Self::new(kani::any())  // ✅ Uses type's own constructor
    }
}
```

### Leaky Abstraction
**Severity**: HIGH

The harness tests `EventSeq` by extracting raw `u64` via `.get()` (line 292, 300, 317) rather than testing the `EventSeq` type's own monotonicity invariants. This leaks the internal representation.

### Invalidated Proof Obligation
**Severity**: CRITICAL

PO-vb282my-AA-KANI-006 claims to verify "Journal sequence monotonicity" for `EventSeq`, but:
- Lines 290-306 test raw `u64` arithmetic, not `EventSeq` behavior
- Line 299 creates `EventSeq::new(u64::MAX)` which may be a no-op or panic if `EventSeq` clamps
- The actual `EventSeq` overflow behavior is NOT tested through its domain API

**Evidence**: `EventSeq` constructor invariants are bypassed; the harness proves raw `u64` math, not `EventSeq` monotonicity.

---

## PROOF OBLIGATION MATRIX

| Obligation | Primitive Obsession Impact | Evidence |
|------------|---------------------------|----------|
| PO-vb282my-AA-KANI-001 | None (state enum test) | Passes |
| PO-vb282my-AA-KANI-002 | Medium (u64 sequence check at line 247) | Bypasses EventSeq |
| PO-vb282my-AA-KANI-003 | CRITICAL (lines 145-148 use raw u16) | StepIdx Arbitrary exists |
| PO-vb282my-AA-KANI-004 | None (state enum test) | Passes |
| PO-vb282my-AA-KANI-005 | None (idempotency test) | Passes |
| PO-vb282my-AA-KANI-006 | CRITICAL (lines 290-318 use raw u64) | EventSeq Arbitrary exists |

---

## RECOMMENDED REFACTORING

### Phase 1: Fix Primitive Obsession (Lines 28-30, 145-148, 290-318)

Replace all raw primitives with domain type `kani::any()`:

```rust
// Fix any_run_id()
fn any_run_id() -> RunId {
    kani::any()  // Uses RunId::any() from kani_id_arbitrary.rs
}

// Fix StepIdx usage (lines 145-148)
// OLD:
let ask_step: u16 = kani::any();
kani::assume(ask_step < 32);
// NEW:
let ask_step: StepIdx = kani::any();
let timer_step: StepIdx = kani::any();

// Fix EventSeq usage (lines 290-318)
// OLD:
let raw_seq: u64 = kani::any();
let seq = EventSeq::new(raw_seq);
// NEW:
let seq: EventSeq = kani::any();
```

### Phase 2: Split File (320 → ~60-80 lines each)

Split into 5 harness files per obligation group. Use shared helper module for `any_run_id()` and `new_shard()`.

### Phase 3: Re-run Kani Verification

After refactoring, re-run `bash scripts/kani-list.sh vb_runtime kani-shard-ask-answer-lifecycle` to verify all obligations still pass with domain types.

---

## ARCHITECTURAL COMPLIANCE SCORE

| Criterion | Score | Notes |
|-----------|-------|-------|
| File size (<300 lines) | 0/10 | 320 lines, 6.7% over |
| Primitive obsession (kani) | 0/10 | 6 violations |
| God Rule 1 compliance | 0/10 | All raw primitives |
| Domain type coverage | 3/10 | Only state enums use proper types |
| Proof validity | 2/10 | PO-006 tests u64, not EventSeq |

**OVERALL: 1/10 — ARCHITECTURAL DRIFT CRITICAL**

---

## VERDICT

**HAMMER STATUS**: 🔨 FULLY ARMED

This file violates:
1. Mandatory 300-line file size limit
2. God Rule 1 (all inputs use kani::any())
3. Scott Wlaschin primitive obsession ban
4. Proof validity (PO-006 proves u64, not EventSeq)

**Required before landing**:
- [ ] Split into ≤5 harness files
- [ ] Replace all raw `u64`/`u16` with domain type `kani::any()`
- [ ] Verify PO-003 and PO-006 pass with domain types
- [ ] Black-hat review sign-off

---

*Report generated by arch-drift-hammer (JJ workspace)*
*Next action: Run `bd show vb-282my` to update bead with findings*
