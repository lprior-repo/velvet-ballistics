# ARCHITECTURAL DRIFT REPORT
## File: `crates/vb_compile/src/kani_foreach_parity.rs`
## Severity: CRITICAL
## Line Count: 587 (violates <300 line mandate by 287 lines, 96% over)

---

## EXECUTIVE SUMMARY

This file is a 587-line Kani proof harness module that verifies the `for_each` IR lowering fix. It **massively violates** the <300 line rule and exhibits multiple Scott Wlaschin DDD violations including **primitive obsession**, **feature envy**, and **duplicate code**.

---

## VIOLATION #1: <300 LINE RULE (CRITICAL)

**Line count: 587** (96% over the 300-line ceiling)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 587 | 300 | ❌ FAIL |
| Excess lines | 287 | 0 | ❌ FAIL |
| Overage % | 96% | 0% | ❌ FAIL |

### Line Count Breakdown

| Section | Lines | Type |
|---------|-------|------|
| Module docs + imports | 1-28 | Header |
| `build_foreach_parts` helper | 29-174 | Helper (146 lines!) |
| KANI-001 proof | 177-249 | Proof |
| KANI-002 proof | 251-308 | Proof |
| KANI-003 proof | 310-407 | Proof |
| KANI-004 proof | 409-529 | Proof |
| KANI-005 proof | 531-587 | Proof |

**The `build_foreach_parts` helper alone is 146 lines** — nearly half the entire 300-line budget consumed by a single helper function.

---

## VIOLATION #2: PRIMITIVE OBSESSION (MAJOR)

### 2.1 Raw Numeric Construction in `build_foreach_parts`

```rust
// Line 54: Raw modulus with inline bounds
let slot_count = slot_count % 16;

// Line 57: Raw cast
let const_count = (const_count % 4) as usize;

// Line 63: Raw kani::any() with modulus
let input_slot = SlotIdx::new(kani::any::<u16>() % std::cmp::max(slot_count, 1));

// Line 64: Raw min operation
let item_slot = SlotIdx::new(1.min(slot_count));

// Line 77: Raw arithmetic for limit
limit: kani::any::<u32>() % 4 + 1, // [1, 4]
```

**Problem**: These should use typed domain constants or builders. The arithmetic `(kani::any::<u32>() % 4 + 1)` produces `[1, 4]` but there's no type enforcing this bound.

### 2.2 Repeated `ResourceContract` Construction (PRIMITIVE OBSESSION + DUPLICATE CODE)

The file constructs `ResourceContract` **THREE TIMES** with raw inline values:

**Instance 1** (lines 152-171):
```rust
resource_contract: ResourceContract {
    max_steps: 256,
    max_slots: 256,
    max_constants: 256,
    // ... 16 more raw fields
},
```

**Instance 2** (lines 499-519):
```rust
resource_contract: ResourceContract {
    max_steps: 256,
    max_slots: 256,
    // ... identical to above
},
```

**Instance 3** (line 132-174 in `build_foreach_parts`):
```rust
ResourceContract {
    max_steps: 256,
    // ...
}
```

**Available but UNUSED**: `ResourceContract::DEFAULT` (compiled_workflow.rs:167-185)

**Scott Wlaschin Principle**: "Make illegal states unrepresentable." The default exists but is not used. The harness should use `ResourceContract::DEFAULT` instead of hand-roll 18 fields.

### 2.3 Raw `WorkflowDigest::from_bytes` Construction

```rust
// Lines 144, 491
digest: WorkflowDigest::from_bytes([0u8; 32]),
```

The domain likely provides a `WorkflowDigest::default()` or similar. Using raw byte array is primitive obsession.

---

## VIOLATION #3: FEATURE ENVY (MAJOR)

### 3.1 `build_foreach_parts` Exposes Internal Structure

**Location**: Lines 45-174 (146 lines!)

The `build_foreach_parts` function is **envying** the internals of `WorkflowParts`:

```rust
WorkflowParts {
    name: "foreach_harness".into(),
    digest: WorkflowDigest::from_bytes([0u8; 32]),
    nodes: nodes.into_boxed_slice(),       // Internal vec-to-box conversion
    expressions: Box::new([]),             // Internal detail
    accessors: Box::new([]),              // Internal detail
    constants: constants.into_boxed_slice(), // Internal detail
    slot_count,
    symbols_count: 0,                     // Magic number
    entry: StepIdx::new(0),
    resource_contract: /* inline */,
    step_names: step_names.into_boxed_slice(), // Internal detail
}
```

**DDD Principle Violated**: This knowledge of `WorkflowParts` internal structure (boxed slices, field names, magic numbers like `symbols_count: 0`) should be encapsulated in a **factory method** on `WorkflowParts` itself, not scattered across test code.

### 3.2 Missing Domain Factory: `ForEachIR` Type

The file manually constructs a 4-node for_each graph:
```
0 = ForEachStart { input, item_slot, body=1, done=3 }
1 = SetConst { value, next=2 }
2 = ForEachNext { body=1, done=3 }
3 = Finish { result }
```

**Scott Wlaschin Principle**: "Types express the domain." There should be a `ForEachIR` value object or aggregate that encapsulates this 4-node pattern. Instead, the harness re-implements this construction logic.

---

## VIOLATION #4: DUPLICATE CODE (MODERATE)

### 4.1 `ResourceContract` Duplication

As noted in 2.2, the same 18-field `ResourceContract` struct is constructed 3 times. This is **Copy-Paste programming**.

### 4.2 Node Construction Pattern Duplication

Every proof function calls `build_foreach_parts(4, 8, 2)` with the same arguments, but the function ignores `_node_count` and hardcodes 4 nodes. If a factory existed, this would be:

```rust
let parts = ForEachIR::minimal_kani_parts(); // instead of build_foreach_parts(4, 8, 2)
```

---

## VIOLATION #5: WRONG ABSTRACTION LEVEL (MODERATE)

### 5.1 Proofs Re-implement Domain Logic

The Kani proofs assert properties about workflow validation:

```rust
// Line 337: Exercise validation
let workflow_result = CompiledWorkflow::try_from_parts(parts.clone());

// Line 340: Assert it passes
kani::assert(workflow_result.is_ok(), ...);

// Lines 362-405: Manually re-check edge properties
kani::assert(node_1_next.is_some(), ...);
kani::assert(next.as_usize() == 2, ...);
```

**Problem**: The assertions re-check what `try_from_parts` already validates. This is redundant — the proof should focus on what `try_from_parts` **cannot** verify (e.g., the `next` edge being `Some(StepIdx(2))` specifically, not just "some forward edge").

### 5.2 Harness Uses Raw Indices Instead of Types

```rust
// Line 63: Raw u16 index
let input_slot = SlotIdx::new(kani::any::<u16>() % std::cmp::max(slot_count, 1));

// Line 86: Hardcoded StepIdx(2)
let next_step = StepIdx::new(2);

// Line 69: Hardcoded StepIdx(0)
id: StepIdx::new(0),
```

The harness should use domain-typed constants:
```rust
const FOR_EACH_START_IDX: StepIdx = StepIdx::new(0);
const FOR_EACH_BODY_IDX: StepIdx = StepIdx::new(1);
const FOR_EACH_NEXT_IDX: StepIdx = StepIdx::new(2);
const FOR_EACH_FINISH_IDX: StepIdx = StepIdx::new(3);
```

---

## RECOMMENDED REFACTORING

### Step 1: Extract `ForEachIR` Domain Type

Create a `ForEachIR` value object in `vb_core` that encapsulates the 4-node construction:

```rust
pub struct ForEachIR {
    pub parts: WorkflowParts,
}

impl ForEachIR {
    /// Build minimal for_each IR with kani-driven indices.
    pub fn from_kani(slot_count: u16, const_count: u8) -> Self { ... }

    /// Build a valid for_each IR for proof use.
    pub fn minimal() -> Self { ... }
}
```

### Step 2: Use `ResourceContract::DEFAULT`

Replace all 3 inline `ResourceContract` constructions with `ResourceContract::DEFAULT`.

### Step 3: Extract Typed Constants

Add module-level constants:
```rust
const FOREACH_BODY_STEP: StepIdx = StepIdx::new(1);
const FOREACH_NEXT_STEP: StepIdx = StepIdx::new(2);
const FOREACH_FINISH_STEP: StepIdx = StepIdx::new(3);
```

### Step 4: Split File

Current structure (587 lines) should be split into:
- `kani_foreach_parity.rs` (imports + shared helpers, ~100 lines)
- `kani_foreach_k001.rs` (KANI-001 proof, ~60 lines)
- `kani_foreach_k002.rs` (KANI-002 proof, ~40 lines)
- `kani_foreach_k003.rs` (KANI-003 proof, ~80 lines)
- `kani_foreach_k004.rs` (KANI-004 proof, ~110 lines)
- `kani_foreach_k005.rs` (KANI-005 proof, ~50 lines)

### Step 5: Inline `ResourceContract` Test Builder

Create a test-only builder:
```rust
#[cfg(test)]
impl ResourceContract {
    pub fn test_contract() -> Self {
        Self::DEFAULT
    }
}
```

---

## SUMMARY TABLE

| Violation | Severity | Lines Affected | Fix Complexity |
|-----------|----------|----------------|----------------|
| <300 line rule | CRITICAL | 587/587 | Medium (split file) |
| Primitive obsession | MAJOR | ~80 | Low (use DEFAULT + typed constants) |
| Feature envy | MAJOR | ~146 | High (extract ForEachIR type) |
| Duplicate code | MODERATE | ~60 | Low (extract to helper) |
| Wrong abstraction | MODERATE | ~70 | Medium (refactor assertions) |

---

## GOD RULE COMPLIANCE

The file **does comply** with GOD RULE #1 (no hardcoded Kani shapes) — it uses `kani::any()` with bounds. This is positive. The violations are purely architectural (DDD/line count).

---

## VERDICT

**ARCHITECTURAL DRIFT: CONFIRMED**

This file requires immediate refactoring:
1. Split into 6 files (~100 lines each)
2. Extract `ForEachIR` domain type
3. Replace inline `ResourceContract` with `ResourceContract::DEFAULT`
4. Add typed constants for node indices
5. Remove duplicate code

**Estimated refactoring effort**: 4-6 beads
