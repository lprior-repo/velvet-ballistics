# ARCHITECTURAL DRIFT HAMMER REPORT
## Target: `vb_storage/src/kani_hydrate_proofs.rs`
## Line Count: 317 (EXCEEDS 300-LINE MANDATE BY 17 LINES)
## Severity: CRITICAL

---

## EXECUTIVE SUMMARY

This file is a **KANNI proof harness file** that has ballooned to 317 lines through:
1. **<300 violation**: 17 lines over the hard limit
2. **Primitive Obsession violations**: Unabated use of raw numeric literals where named types should be used
3. **DDD violations**: Thin helper functions that create "smart" names over raw primitives without reducing coupling
4. **Repetition without abstraction**: Every proof rebuilds event/snapshot structures from scratch instead of using shared builders

---

## SECTION 1: LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 317 | 300 | **EXCEEDED** |
| Overflow | +17 | 0 | **FAIL** |
| Proofs | 17 | N/A | N/A |
| Helper Lines | ~70 | N/A | N/A |

---

## SECTION 2: KANI PROOF RESPONSIBILITY MAP

### 2.1 Proof Inventory

| ID | Function | Proof Target | Lines |
|----|----------|--------------|-------|
| PO-VB-STORAGE-001 | `kani_events_preconditions_non_empty` | `hydrate_events_preconditions` | 72-94 |
| PO-VB-STORAGE-002 | `kani_events_preconditions_empty` | `hydrate_events_preconditions` | 97-103 |
| PO-VB-STORAGE-003 | `kani_dimensions_positive_accepts_positive` | `hydrate_dimensions_positive` | 106-114 |
| PO-VB-STORAGE-004 | `kani_dimensions_positive_rejects_zero_step` | `hydrate_dimensions_positive` | 117-124 |
| PO-VB-STORAGE-005 | `kani_dimensions_positive_rejects_zero_slot` | `hydrate_dimensions_positive` | 127-134 |
| PO-VB-STORAGE-006 | `kani_dimensions_positive_rejects_both_zero` | `hydrate_dimensions_positive` | 137-144 |
| PO-VB-STORAGE-007 | `kani_has_evidence_tail_non_empty` | `hydrate_snapshot_tail_has_evidence` | 147-160 |
| PO-VB-STORAGE-008 | `kani_has_evidence_slots_non_empty` | `hydrate_snapshot_tail_has_evidence` | 163-171 |
| PO-VB-STORAGE-009 | `kani_has_evidence_all_empty` | `hydrate_snapshot_tail_has_evidence` | 174-182 |
| PO-VB-STORAGE-010 | `kani_run_matches_true` | `hydrate_snapshot_tail_run_matches` | 185-198 |
| PO-VB-STORAGE-011 | `kani_run_matches_snapshot_differs` | `hydrate_snapshot_tail_run_matches` | 201-215 |
| PO-VB-STORAGE-012 | `kani_run_matches_event_differs` | `hydrate_snapshot_tail_run_matches` | 217-232 |
| PO-VB-STORAGE-013 | `kani_seq_after_true` | `hydrate_snapshot_tail_seq_after_snapshot` | 235-249 |
| PO-VB-STORAGE-014 | `kani_seq_after_false_before` | `hydrate_snapshot_tail_seq_after_snapshot` | 252-266 |
| PO-VB-STORAGE-015 | `kani_seq_after_false_equal` | `hydrate_snapshot_tail_seq_after_snapshot` | 269-283 |
| PO-VB-STORAGE-016 | `kani_preconditions_all_met` | `hydrate_snapshot_tail_preconditions` | 286-300 |
| PO-VB-STORAGE-017 | `kani_preconditions_false_run_mismatch` | `hydrate_snapshot_tail_preconditions` | 302-317 |

### 2.2 Proof Grouping Analysis

```
Proof Groups:
├── hydrate_events_preconditions (2 proofs)
├── hydrate_dimensions_positive (4 proofs)
├── hydrate_snapshot_tail_has_evidence (3 proofs)
├── hydrate_snapshot_tail_run_matches (3 proofs)
├── hydrate_snapshot_tail_seq_after_snapshot (3 proofs)
└── hydrate_snapshot_tail_preconditions (2 proofs)
```

---

## SECTION 3: PRIMITIVE OBSESSION VIOLATIONS

### 3.1 Raw Numeric Literal Inventory

| Line(s) | Raw Value | Context | Should Be |
|---------|-----------|---------|-----------|
| 74 | `StepIdx::new(0)` | Proof setup | `StepIdx::ZERO` or constant |
| 109 | `kani::any::<u8>().saturating_add(1)` | dimension generation | `DimensionGenerator::positive()` |
| 110 | same | slot dimension | same |
| 119 | `0u16` | step_count zero | `StepCount::ZERO` |
| 120 | `kani::any::<u8>().saturating_add(1)` | slot dimension | same as above |
| 130 | `kani::any::<u8>().saturating_add(1)` | step dimension | same as above |
| 130 | `0u16` | slot_count zero | `SlotCount::ZERO` |
| 139 | `0u16` | step_count | same as 119 |
| 140 | `0u16` | slot_count | same as 130 |
| 150 | `StepIdx::new(0)` | same as 74 | same |
| 152 | `EventSeq::new(1)` | event seq | Named: `FIRST_EVENT_SEQ` |
| 153 | `EventSeq::new(2)` | event seq | Part of range |
| 188 | `StepIdx::new(0)` | same as 74 | same |
| 193 | `EventSeq::new(5)` | snapshot seq | `SNAPSHOT_SEQ` |
| 239 | `EventSeq::new(5)` | snapshot seq | same |
| 241-242 | `EventSeq::new(6)`, `EventSeq::new(7)` | event seqs | part of event range |
| 256 | `EventSeq::new(10)` | snapshot seq | snapshot seq |
| 258-259 | `EventSeq::new(5)`, `EventSeq::new(7)` | event seqs | event range |
| 273 | `EventSeq::new(5)` | snapshot seq | same |
| 275-276 | `EventSeq::new(5)`, `EventSeq::new(6)` | event seqs | event range |

### 3.2 Violation Pattern Analysis

**Pattern A: "Zero as Literal"**
- `StepIdx::new(0)` appears 5 times
- `0u16` appears 5 times as dimension input
- **Root Cause**: No zero-valued constants on types

**Pattern B: "Sequence Numbers as Magic Numbers"**
- `EventSeq::new(1)`, `EventSeq::new(5)`, `EventSeq::new(10)` scattered
- No semantic naming (e.g., `FIRST_SEQUENCE`, `SNAPSHOT_SEQUENCE`)

**Pattern C: "Dimension Generation is Ad-Hoc"**
- `u16::from(kani::any::<u8>().saturating_add(1))` repeated 4 times
- Should be: `Dimension::any_positive()` or similar

---

## SECTION 4: DDD VIOLATIONS (Scott Wlaschin)

### 4.1 Thin Helpers (No Real Abstraction)

```rust
// Lines 31-39: empty_snapshot
fn empty_snapshot(run: RunId, seq: EventSeq) -> RunSnapshot {
    RunSnapshot {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([0u8; 32]), // RAW [u8; 32]
        slots: Vec::new(),
        taint: Vec::new(),
    }
}
```

**Problem**: Creates a "smart" name but still uses raw `[0u8; 32]` for `WorkflowDigest`. The helper doesn't hide primitive complexity.

### 4.2 Repetition Without Value Objects

Every proof constructs `JournalEvent::StepStarted` manually:
```rust
// Lines 57-64: step_started helper (only used in SOME proofs)
fn step_started(run: RunId, seq: EventSeq, step: StepIdx, attempt: u16) -> JournalEvent {
    JournalEvent::StepStarted { run, seq, step, attempt }
}
```

But then OTHER proofs inline this:
```rust
let event1 = JournalEvent::StepStarted {
    run,
    seq: EventSeq::new(0),
    step: step_idx,
    attempt: 1,
};
```

**DDD Violation**: No `StepStarted` factory or builder. Raw struct construction scattered everywhere.

### 4.3 Missing Value Object: `StepStartedBuilder`

```rust
// SHOULD EXIST but doesn't:
impl StepStartedBuilder {
    fn new(run: RunId, seq: EventSeq) -> Self { ... }
    fn step(mut self, idx: StepIdx) -> Self { ... }
    fn attempt(mut self, n: u16) -> Self { ... }
    fn build(self) -> JournalEvent { ... }
}
```

### 4.4 Missing Value Objects for Dimensions

```rust
// SHOULD EXIST but doesn't:
pub struct PositiveStepCount(u16);
pub struct PositiveSlotCount(u16);

impl PositiveStepCount {
    pub fn new(v: u16) -> Option<Self> { 
        if v > 0 { Some(Self(v)) } else { None }
    }
}
```

---

## SECTION 5: STRUCTURAL DRIFT

### 5.1 File Structure Analysis

```
Lines 1-28: Module docs + imports (28 lines)
Lines 30-50: RunSnapshot helpers (21 lines)
Lines 56-64: Event construction helper (9 lines)
Lines 70-317: 17 individual proofs (~248 lines)
```

### 5.2 Structural Violation

The file violates the **single responsibility** of a proof harness file. It contains:
1. Two RunSnapshot construction helpers (concern: test fixture building)
2. One event construction helper (concern: test fixture building)
3. Seventeen Kani proofs (concern: verification)

These should be in separate files:
```
vb_storage/
  verification/
    fixtures.rs        # Snapshot and event builders
    hydrate_events_preconditions.rs
    hydrate_dimensions_positive.rs
    hydrate_snapshot_tail_has_evidence.rs
    hydrate_snapshot_tail_run_matches.rs
    hydrate_snapshot_tail_seq_after_snapshot.rs
    hydrate_snapshot_tail_preconditions.rs
```

---

## SECTION 6: SPECIFIC VIOLATIONS

| # | Violation | Type | Severity |
|---|-----------|------|----------|
| 1 | File exceeds 300 lines | Size | CRITICAL |
| 2 | `StepIdx::new(0)` hardcoded 5 times | Primitive Obsession | HIGH |
| 3 | `0u16` raw zero used for dimensions | Primitive Obsession | HIGH |
| 4 | `EventSeq::new(N)` magic numbers scattered | Primitive Obsession | HIGH |
| 5 | `WorkflowDigest::from_bytes([0u8; 32])` raw array | Primitive Obsession | HIGH |
| 6 | `kani::any::<u8>().saturating_add(1)` repeated 4x | DRY | MEDIUM |
| 7 | No `StepStartedBuilder` value object | DDD | HIGH |
| 8 | No dimension value objects | DDD | HIGH |
| 9 | `RunId::new(42)` magic number | Primitive Obsession | MEDIUM |
| 10 | No module separation for proof groups | Structure | HIGH |

---

## SECTION 7: RECOMMENDED REFACTORING

### 7.1 Immediate Fixes (No Architecture Change)

1. **Extract magic numbers to constants**:
```rust
const DEFAULT_RUN_ID: RunId = RunId::new(42);
const DEFAULT_STEP_IDX: StepIdx = StepIdx::new(0);
const FIRST_EVENT_SEQ: EventSeq = EventSeq::new(1);
```

2. **Add dimension generators**:
```rust
fn any_positive_u16() -> u16 {
    u16::from(kani::any::<u8>().saturating_add(1))
}
```

### 7.2 Short-Term Fixes (Module Splitting)

Split into one file per proof group:
```
crates/vb_storage/src/verification/
├── mod.rs
├── fixtures.rs          // Shared builders
├── events_preconditions.rs
├── dimensions_positive.rs
├── snapshot_tail_has_evidence.rs
├── snapshot_tail_run_matches.rs
├── snapshot_tail_seq_after_snapshot.rs
└── snapshot_tail_preconditions.rs
```

### 7.3 Long-Term Fixes (Value Objects)

1. Create `StepStartedEvent(RunId, EventSeq, StepIdx, Attempt)` wrapper
2. Create `StepCount(u16)` and `SlotCount(u16)` with bounded constructors
3. Create `SequenceNumber(u64)` with ordering semantics
4. Create `WorkflowDigest` constants for test fixtures

---

## SECTION 8: VERDICT

| Check | Result |
|-------|--------|
| <300 Line Rule | **FAIL** (317 lines, +17) |
| Primitive Obsession | **FAIL** (15+ violations) |
| DDD Cohesion | **FAIL** (thin helpers, no value objects) |
| File Responsibility | **FAIL** (3 concerns in 1 file) |

### Overall: **ARCHITECTURAL DRIFT CONFIRMED**

This file must be refactored before landing. The combination of:
1. Exceeding the 300-line hard limit
2. Unabated primitive obsession
3. No real DDD abstractions

...constitutes active drift from the canonical architecture.

---

## SECTION 9: MANDATORY ACTIONS

1. **SPLIT** this file into `verification/` subdirectory with one file per proof group
2. **EXTRACT** magic numbers to named constants
3. **CREATE** `fixtures.rs` with shared builders
4. **INTRODUCE** value object types for `StepCount`, `SlotCount`, `SequenceNumber`
5. **RERUN** `kani` on each split file to verify proofs still pass
6. **UPDATE** `moon ci` to reflect new verification structure

**This file is NOT approved for landing in current form.**
