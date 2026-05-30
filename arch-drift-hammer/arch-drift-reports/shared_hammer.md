# Architectural Drift Report: `vb_validate/src/shared.rs`

**File**: `crates/vb_validate/src/shared.rs`
**Line Count**: 305 (exceeds 300-line limit by 5 lines)
**Severity**: HIGH - Structural violation requiring mandatory refactor
**Date**: 2026-05-29
**Agent**: architectural-drift enforcer

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 305 | 300 | 🔴 OVER by 5 lines |

---

## 2. SHARED VALIDATION RESPONSIBILITIES MAP

```
shared.rs (305L)
├── ValidationPipeline struct (lines 34-53)
│   ├── 9 boolean gate flags (gate_07 through gate_15)
│   ├── Default impl (lines 55-59)
│   ├── all_gates() constructor (lines 64-76)
│   ├── no_gates() constructor (lines 82-94)
│   ├── validate() method (lines 104-130)
│   └── validate_with_contracts() method (lines 139-152)
├── validate() convenience fn (lines 159-161)
├── validate_with_contracts() convenience fn (lines 168-173)
├── Public re-exports of 9 gate functions (lines 17-26)
└── Test module (lines 180-305)
    ├── make_parts() helper (lines 185-199)
    ├── finish_node() helper (lines 201-212)
    ├── 5 pipeline behavior tests (lines 214-305)
```

**Orchestration Model**: `ValidationPipeline` is the **director** - it sequences 9 gates in order. Each gate is a **specialist** performing one structural check.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `ValidationPipeline` — 9 Boolean Flags

**Problem**: Nine `bool` fields encode gate activation state. This is a textbook primitive obsession anti-pattern.

```rust
pub struct ValidationPipeline {
    pub gate_07_expression_stack: bool,    // raw bool
    pub gate_08_accessor_paths: bool,      // raw bool
    pub gate_09_slot_references: bool,     // raw bool
    pub gate_10_node_kind_specific: bool,   // raw bool
    pub gate_11_loop_body_graph: bool,     // raw bool
    pub gate_12_action_contracts: bool,     // raw bool
    pub gate_13_no_slot_cycles: bool,      // raw bool
    pub gate_14_slot_type_consistency: bool,// raw bool
    pub gate_15_determinism_proof: bool,    // raw bool
}
```

**Scott Wlaschin Fix**: Replace with a `GateFlags` bitflags struct:

```rust
use bitflags::bitflags;
bitflags! {
    pub struct GateFlags: u16 {
        const EXPRESSION_STACK  = 1 << 0;  // Gate 7
        const ACCESSOR_PATHS    = 1 << 1;  // Gate 8
        const SLOT_REFERENCES   = 1 << 2;  // Gate 9
        const NODE_KIND_SPECIFIC= 1 << 3;  // Gate 10
        const LOOP_BODY_GRAPH   = 1 << 4;  // Gate 11
        const ACTION_CONTRACTS  = 1 << 5;  // Gate 12
        const NO_SLOT_CYCLES    = 1 << 6;  // Gate 13
        const SLOT_TYPE_CONSIST = 1 << 7;  // Gate 14
        const DETERMINISM_PROOF = 1 << 8; // Gate 15
    }
}
```

**Why**: Gate numbers (7-15) encoded in field names are primitive obsession. The gates are an **ordered set** with sequence semantics, not 9 independent booleans.

### 3.2 Test Code — Raw `u16` Slot Indices

**Problem**: Test helpers wrap raw `u16` values into `SlotIdx`/`StepIdx` instead of using domain-typed builders.

```rust
// Lines 209, 252, 271, 292 — raw u16 passed to SlotIdx::new()
kind: CompiledNodeKind::Finish {
    result: SlotIdx::new(result_slot),  // result_slot is u16
},

// Lines 252, 271, 292 — magic number 99
output: Some(SlotIdx::new(99)),  // What does 99 mean? No context.
```

**Fix**: Create domain-typed test builders:
```rust
fn make_invalid_slot_parts() -> WorkflowParts { ... } // Returns parts with OOB slot
fn finish_node(index: u16, result_slot: SlotIdx) -> CompiledNode { ... } // SlotIdx not u16
```

### 3.3 `make_parts` Helper — Raw `u16` Slot Count

**Problem** (lines 185-199):
```rust
fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        ...
        slot_count,  // raw u16, no validation this is reasonable
        symbols_count: 0,
        entry: StepIdx::new(0),  // assumes index 0 is valid
    }
}
```

**Fix**: `SlotCount(u16)` newtype wrapper that validates the value is non-zero and reasonable.

---

## 4. DDD COHESION VIOLATIONS

### 4.1 God Struct: `ValidationPipeline`

`ValidationPipeline` violates Single Responsibility Principle — it has **three distinct roles**:

| Role | Evidence |
|------|----------|
| **Configuration** | 9 gate boolean fields, `all_gates()`, `no_gates()` |
| **Director/Orchestrator** | `validate()` method sequences gate execution |
| **Factory** | Creates default pipeline via `Default` |

**Fix**: Split into:
- `GateFlags` — configuration (bitflags)
- `ValidationPipeline` — pure director (accepts `GateFlags` + gate implementations)
- `ValidationPipelineBuilder` — factory with sensible defaults

### 4.2 Module Re-exports Leak Coupling

**Problem** (lines 17-26):
```rust
pub use gates::validate_gate_07_expression_stack_depth;
pub use gates::validate_gate_08_accessor_path_segments;
pub use gates::validate_gate_09_slot_references;
pub use gates::validate_gate_10_node_kind_specific;
pub use gates::validate_gate_11_loop_body_graph;
pub use gates::validate_gate_12_action_contract_completeness;
pub use gates::validate_gate_13_no_slot_cycles;
pub use gates::validate_gate_14_slot_type_consistency;
pub use gates::validate_gate_15_determinism_proof;
```

**Why it's a problem**: External callers can bypass `ValidationPipeline` and call individual gates directly. This breaks the **encapsulation** of the validation pipeline. Callers should only see `validate()` / `validate_with_contracts()`.

**Fix**: Remove re-exports. Gates are internal implementation details.

### 4.3 Test Module Mixed with Production Code

**Problem**: Lines 180-305 (~127 lines, 42% of file) are test code embedded in the same file as production code.

**Scott Wlaschin Principle**: "Functions should be small, files should be small too."

**Fix**: Move tests to `crates/vb_validate/src/shared_test.rs` or `crates/vb_validate/tests/shared_pipeline_tests.rs`. Production and test code should be in separate files.

---

## 5. GATE NUMBER ENCODING — PRIMITIVE OBSESSION

**Problem**: Gate numbers 7-15 are embedded in field names and function names. The numbers have **ordinal meaning** (execution order: 7→8→9→10→11→13→14→15, then 12 last).

```rust
// Line 104 comment admits non-sequential execution order:
// Gates execute in ascending order (7, 8, 9, 10, 11, 13, 14, 15).
// Gate 12 (action contract completeness) requires external action contract
// data and is skipped by this method
```

**Fix**: Define gates as an ordered `enum` with `#[derive(Ordering)]`:
```rust
#[derive(EnumSetType, Debug)]
pub enum ValidationGate {
    ExpressionStackDepth,     // was gate 7
    AccessorPathSegments,     // was gate 8
    SlotReferences,            // was gate 9
    NodeKindSpecific,          // was gate 10
    LoopBodyGraph,             // was gate 11
    ActionContractCompleteness,// was gate 12
    NoSlotCycles,              // was gate 13
    SlotTypeConsistency,       // was gate 14
    DeterminismProof,          // was gate 15
}
```

---

## 6. REMEDIATION PLAN (Priority Order)

| Priority | Action | Target | Effort |
|----------|--------|--------|--------|
| **P0** | Move test module to separate file | `shared_test.rs` | 1 refactor |
| **P0** | Remove `pub use` re-exports | Keep gates internal | 1 edit |
| **P1** | Extract `GateFlags` bitflags type | Replace 9 bools | 3 edits |
| **P1** | Add `SlotCount(u16)` newtype for test helper | `make_parts` | 1 newtype |
| **P2** | Replace `SlotIdx::new(u16)` magic numbers in tests | Typed test builders | 1 helper fn |
| **P2** | Define `ValidationGate` enum with ordering | Replace gate name encoding | 2 edits |

---

## 7. METRICS SUMMARY

| Metric | Value |
|--------|-------|
| Lines over limit | 5 (305 - 300) |
| Primitive obsession instances | 4 (9 bools, 3 raw u16 usages, gate numbers in names, raw u16 slot_count) |
| DDD violations | 4 (god struct, coupling leak, mixed test/prod, ordinal gate numbers) |
| Files needing change | 1 (`shared.rs`) + 1 new test file |
| Estimated refactor units | 8 |

---

## 8. VERDICT

**STATUS**: 🔴 RED — MANDATORY REFACTOR REQUIRED

The file violates the <300 line rule and contains multiple primitive obsession violations per Scott Wlaschin DDD principles. The `ValidationPipeline` struct is a god struct performing configuration, orchestration, and factory roles. Nine boolean fields encode an ordered set of gates with ordinal semantics baked into names.

**Required Actions**:
1. Extract test module to `shared_test.rs` (reduces by ~127 lines → 178 lines total)
2. Remove `pub use` re-exports (removes ~10 lines)
3. Replace 9 booleans with `GateFlags` bitflags (reduces struct to 1 field)
4. Apply `SlotCount` newtype to `make_parts`
5. Define `ValidationGate` enum to replace ordinal encoding

After refactor, estimated line count: **~180 lines** (40% reduction).
