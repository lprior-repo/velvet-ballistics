# Architectural Drift Report: `kani_hydrate_proofs.rs`

**File**: `crates/vb_storage/src/kani_hydrate_proofs.rs`
**Line Count**: 317 (EXCEEDS 300-line limit by 17 lines)
**Status**: MUST REFACTOR

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 317 | 300 | 🔴 OVERFLOW (+17) |
| Proof Functions | 17 | N/A | OK |
| Helper Functions | 3 | N/A | OK |
| Comments/Lines | ~30 | N/A | OK |

---

## 2. Storage Hydrate Proofs Responsibility Map

### Target Functions Under Test
| Function | Proofs | Lines |
|----------|--------|-------|
| `hydrate_events_preconditions` | 2 | 70-103 |
| `hydrate_dimensions_positive` | 4 | 105-144 |
| `hydrate_snapshot_tail_has_evidence` | 3 | 146-182 |
| `hydrate_snapshot_tail_run_matches` | 3 | 184-232 |
| `hydrate_snapshot_tail_seq_after_snapshot` | 3 | 234-283 |
| `hydrate_snapshot_tail_preconditions` | 2 | 285-317 |

### Helper Functions (Non-proof)
| Function | Lines | Responsibility |
|----------|-------|----------------|
| `empty_snapshot` | 31-39 | Creates empty `RunSnapshot` for precondition testing |
| `snapshot_with_slots` | 42-50 | Creates `RunSnapshot` with evidence slots |
| `step_started` | 57-64 | Constructs `StepStarted` events |

---

## 3. Primitive Obsession Violations

### VIOLATION 1: Raw Numeric Literals in Proofs
```rust
// Line 73: Hardcoded "42" - RunId
let run = RunId::new(42);

// Lines 149-150, 187-188, etc.: Hardcoded step/seq values
let step_idx = StepIdx::new(0);

// Lines 239, 256, 273: Snapshot sequence numbers
let snapshot_seq = EventSeq::new(5);
```
**Issue**: These are scattered primitive literals. While `RunId::new(42)` is not terrible (since RunId is already a NewType), the pattern is inconsistent.

### VIOLATION 2: Evidence Slots as Raw Byte Vectors
```rust
// Lines 47-48: Raw Vec<u8> for evidence
slots: vec![0u8], // non-empty to satisfy has_evidence
taint: vec![0u8],

// Line 109-110: Raw u8/u16 for dimensions
let step_count = u16::from(kani::any::<u8>().saturating_add(1));
let slot_count = u16::from(kani::any::<u8>().saturating_add(1));
```
**Issue**: Evidence slots and taint are semantically meaningful domain concepts (evidence of workflow state) but represented as `Vec<u8>`. Should be a proper `EvidenceSlots` or `TaintMarkers` type with bounded semantics.

### VIOLATION 3: Hardcoded Magic Values in Dimension Tests
```rust
// Lines 109-110
let step_count = u16::from(kani::any::<u8>().saturating_add(1)); // 1..256
let slot_count = u16::from(kani::any::<u8>().saturating_add(1)); // 1..256
```
**Issue**: The comment "1..256" describes the domain semantics but is not enforced by types. A `StepCount(u16)` or `SlotCount(u16)` with proper invariant would make this explicit.

### VIOLATION 4: WorkflowDigest from Raw Bytes
```rust
// Lines 35, 46: Raw 32-byte array construction
WorkflowDigest::from_bytes([0u8; 32]),
```
**Issue**: Test digests should use a `TestDigest` or `ArbitraryDigest` that makes the zero-ness explicit in the type name, not a production `WorkflowDigest`.

---

## 4. Structural Observations

### What's Acceptable
1. **Helper functions at top**: `empty_snapshot`, `snapshot_with_slots`, `step_started` are reasonable test fixtures following the builder pattern.
2. **Proof organization**: Each proof is focused, named with PO-VB-STORAGE-XXX codes, and has clear assertions.
3. **Kani harness discipline**: Uses `#[kani::proof]` attribute correctly, uses slices to avoid complex drop paths.

### What Needs Fixing
1. **File MUST be split**: 317 lines cannot remain as single file.
2. **Primitive obsession in test data construction**: `vec![0u8]` for evidence is a domain concept that should be typed.
3. **Scattered magic numbers**: 42, 43, 0, 5, 6, 7, 10 appear in multiple places without explanation.

---

## 5. Recommended Refactoring

### Split into 3 files:

**File 1: `kani_hydrate_common.rs`** (~50 lines)
- `empty_snapshot` helper
- `snapshot_with_slots` helper  
- `step_started` helper

**File 2: `kani_hydrate_precondition_proofs.rs`** (~130 lines)
- `kani_events_preconditions_non_empty`
- `kani_events_preconditions_empty`
- `kani_dimensions_positive_accepts_positive`
- `kani_dimensions_positive_rejects_zero_step`
- `kani_dimensions_positive_rejects_zero_slot`
- `kani_dimensions_positive_rejects_both_zero`

**File 3: `kani_hydrate_snapshot_proofs.rs`** (~140 lines)
- `kani_has_evidence_*` proofs (3)
- `kani_run_matches_*` proofs (3)
- `kani_seq_after_*` proofs (3)
- `kani_preconditions_*` proofs (2)

### Domain Type Improvements (Separate Issue)

Create proper evidence types:
```rust
// In vb_storage recovery/types
pub struct EvidenceSlots(Vec<u8>);
pub struct TaintMarkers(Vec<u8>);

// In test fixtures
impl EvidenceSlots {
    pub fn any_non_empty() -> Self { Self(vec![0u8]) }
    pub fn empty() -> Self { Self(Vec::new()) }
}
```

---

## 6. Verdict

| Check | Status |
|--------|--------|
| Line Count < 300 | 🔴 FAIL (317) |
| No Primitive Obsession | ⚠️ WARN (evidence as Vec<u8>) |
| DDD Cohesion | ⚠️ WARN (evidence concepts loose) |
| Proof Correctness | ✅ OK |
| Naming Convention | ✅ OK |

**IMMEDIATE ACTION REQUIRED**: Split this file before any further work lands on this codebase.
