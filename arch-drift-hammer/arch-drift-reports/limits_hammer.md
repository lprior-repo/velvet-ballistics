# Architectural Drift Report: `vb_core/src/limits.rs`

**File:** `crates/vb_core/src/limits.rs`
**Total Lines:** 462
**Status:** VIOLATION — Must Split

---

## 1. LINE COUNT VIOLATION

| Rule | Actual | Max Allowed | Verdict |
|------|--------|-------------|---------|
| <300 lines per file | 462 | 300 | **FAIL** |

**Required Action:** Split into minimum 2 files before any further review gates pass.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (Scott Wlaschin DDD)

### Violation A: No Value Newtypes

Every constant is a raw primitive (`usize`, `u8`, `u16`, `u32`, `u64`). Zero wrapping into domain-specific types.

**Examples of violations:**
```rust
// WRONG: Raw primitive used directly
pub const MAX_STEPS_PER_WORKFLOW: usize = 65_535;
pub const MAX_STEP_BUDGET: u64 = 10_000;
pub const MAX_INPUT_BYTES: u32 = 16_777_216;
pub const MAX_RETRY_ATTEMPTS: u16 = 10;
```

**Should be:**
```rust
// Newtype wrapper enforcing domain semantics
pub struct MaxStepsPerWorkflow(u16);
pub struct MaxStepBudget(u64);
pub struct MaxInputBytes(u32);
```

### Violation B: No Index/Ticket/Symbol Types

Constants reference indices that should have dedicated types:
- `StepIdx` (should wrap `u16`)
- `SlotIdx` (should wrap `u16`)
- `ConstIdx` (should wrap `u16`)
- `ExprIdx` (should wrap `u16`)
- `AccessorIdx` (should wrap `u16`)

But the limits reference these types without creating them.

### Violation C: No Value-Limit Types

Values like `MAX_LIST_ITEMS_PER_VALUE`, `MAX_OBJECT_FIELDS_PER_VALUE` have no corresponding runtime value types to enforce them.

---

## 3. RESPONSIBILITY CLUSTERING (Chaotic Monolith)

The file is a **flat namespace** of 39 constants with ZERO structural organization. Scott Wlaschin demands bounded contexts and clear module boundaries.

### Current chaotic clusters:

| Cluster | Constants | Domain Concept |
|---------|-----------|----------------|
| **Compilation** | `MAX_STEPS_PER_WORKFLOW`, `MAX_SLOTS_PER_WORKFLOW`, `MAX_SLOTS_PER_STEP`, `MAX_CONSTANTS` | Workflow compilation limits |
| **Expression** | `MAX_EXPRESSION_DEPTH`, `MAX_EXPRESSION_OPS`, `MAX_EXPRESSION_STACK`, `MAX_BYTECODE_OPS_PER_EXPRESSION`, `MAX_PATH_DEPTH`, `MAX_EXPRESSIONS`, `MAX_ACCESSORS` | Expression engine limits |
| **Runtime Values** | `MAX_LIST_ITEMS_PER_VALUE`, `MAX_OBJECT_FIELDS_PER_VALUE`, `MAX_SYMBOL_BYTES_PER_VALUE`, `MAX_BLOB_BYTES_PER_VALUE`, `MAX_VALUES_PER_RUN` | Arena value limits |
| **Execution** | `MAX_STEP_BUDGET`, `MAX_LANGUAGE_NESTING_DEPTH`, `MAX_RUN_NAME_LENGTH`, `MAX_SLOTS` | Runtime execution limits |
| **I/O** | `MAX_INPUT_BYTES`, `MAX_OUTPUT_BYTES`, `MAX_BLOB_BYTES`, `MAX_IPC_PAYLOAD_BYTES` | Admission/output limits |
| **Concurrency** | `MAX_RETRY_ATTEMPTS`, `MAX_FANOUT`, `MAX_COLLECT_ITEMS`, `MAX_QUEUE_DEPTH` | Concurrency limits |
| **Journal** | `MAX_JOURNAL_BATCH_BYTES` | Persistence limits |
| **Redundant** | `MAX_EXPRESSION_STACK_USIZE` | Duplicates `MAX_EXPRESSION_STACK` as usize |

### Missing types that should exist:
1. `WorkflowCompilationLimits` struct bundling compilation limits
2. `ExpressionLimits` struct bundling expression engine limits
3. `RuntimeValueLimits` struct bundling arena limits
4. `ExecutionLimits` struct bundling runtime execution limits
5. `IoLimits` struct bundling I/O limits
6. `ConcurrencyLimits` struct bundling concurrency limits

---

## 4. TEST CLUSTERING VIOLATION

The `#[cfg(test)]` module is **inline** (lines 132-462) — 330 lines of tests in a non-test file. This is 71% of the file.

Tests should be:
- In `crates/vb_core/tests/limits_tests.rs` (integration tests)
- Or in separate `limits_compilation_tests.rs`, `limits_relationship_tests.rs`

---

## 5. SPECIFIC DDD FAULTS

### Fault 1: `Parse, Don't Validate` Not Followed

The constants exist but nothing enforces at the type level that:
- `MAX_EXPRESSION_STACK` and `MAX_EXPRESSION_STACK_USIZE` stay in sync
- `MAX_BYTECODE_OPS_PER_EXPRESSION` equals `MAX_EXPRESSION_OPS`

These are only validated at test time, not enforced at compile time.

**Fix:** Single source of truth with a const assertion:
```rust
const _: () = assert!(MAX_BYTECODE_OPS_PER_EXPRESSION == MAX_EXPRESSION_OPS);
```

### Fault 2: No Nominal Types for Semantic Meaning

`u16::MAX` for `MAX_SLOTS` but `usize` for everything else — inconsistency. No `SlotIdx` type to make it explicit.

### Fault 3: Magic Numbers at Boundaries

`65_535` appears 5 times but should be derived from a base type constant like `U16_MAX` or `SlotIdx::MAX`.

---

## 6. MANDATORY REFACTORING PLAN

### Step 1: Split into Modules

```
crates/vb_core/src/limits/
├── mod.rs           # Re-exports all limits
├── compilation.rs   # WorkflowCompilationLimits
├── expression.rs    # ExpressionLimits  
├── runtime.rs       # RuntimeValueLimits + ExecutionLimits
├── io.rs            # IoLimits
├── concurrency.rs   # ConcurrencyLimits
└── journal.rs       # JournalLimits
```

### Step 2: Create Newtypes

```rust
// src/limits/newtypes.rs
use tell::Tell;

pub struct MaxStepsPerWorkflow(u16);
pub struct MaxSlotsPerWorkflow(u16);
pub struct MaxSlotsPerStep(u8);
pub struct MaxConstants(u16);
pub struct MaxStepBudget(u64);
// ... etc
```

### Step 3: Move Tests Out

Tests move to `crates/vb_core/tests/limits_integration_tests.rs`.

### Step 4: Add Const Assertions

Replace runtime tests with compile-time const assertions in each module.

---

## 7. SUMMARY

| Category | Verdict |
|----------|---------|
| Line Count | **FAIL** (462 > 300) |
| Primitive Obsession | **FAIL** (39 raw primitives, 0 newtypes) |
| DDD Cohesion | **FAIL** (flat namespace, no bounded contexts) |
| Test Placement | **FAIL** (330 lines inline) |
| Parse Don't Validate | **FAIL** (tests only, no compile-time enforcement) |

---

## 8. REQUIRED ACTIONS BEFORE GATE PASS

1. [ ] Split `limits.rs` into at least 2 files
2. [ ] Create index newtypes (`StepIdx`, `SlotIdx`, `ConstIdx`, etc.)
3. [ ] Create limit newtypes wrapping each constant
4. [ ] Move inline tests to `tests/` directory
5. [ ] Add const assertions for invariant relationships
6. [ ] Verify all files < 300 lines
7. [ ] Re-run `architectural-drift` gate

---

**Report Generated:** 2026-05-29
**Enforcer:** architectural-drift agent
**Repo:** velvet-ballistics
