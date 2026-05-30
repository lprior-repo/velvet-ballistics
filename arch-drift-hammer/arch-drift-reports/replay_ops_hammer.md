# ARCHITECTURAL DRIFT HAMMER REPORT
## Target: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/replay/ops.rs`

---

## EXECUTIVE SUMMARY

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 2101 | 300 | **CATASTROPHIC FAIL** |
| Size Ratio | 7.0x | 1x | OVER LIMIT |
| Operation Types | ~20 | N/A | BLOAT |
| Test Lines | 1829 | N/A | 87% of file |

---

## VIOLATION #1: LINE COUNT (CRITICAL)

**File:** `crates/vb_core/src/replay/ops.rs`  
**Actual:** 2101 lines  
**Limit:** 300 lines  
**Ratio:** 7.0x over limit

### Breakdown by Section:
| Section | Lines |占比 |
|---------|-------|-----|
| Core operation fns (1-270) | 270 | 13% |
| Test module (272-2101) | 1830 | 87% |

---

## VIOLATION #2: SINGLE-FILE OPERATIONS BLOATED

### Current Structure:
```
ops.rs (2101 lines)
├── Core dispatch: eval_replay_op (13-44)
├── Load operations:
│   ├── eval_load_slot (46-64)
│   ├── eval_load_const (66-81)
│   └── eval_load_accessor (83-102)
├── Arithmetic operations:
│   ├── eval_add (134-142)
│   ├── eval_sub (144-152)
│   ├── eval_mul (154-162)
│   └── eval_div (164-172)
├── Comparison operations:
│   ├── eval_gt (174-177)
│   ├── eval_gte (179-182)
│   ├── eval_lt (184-187)
│   └── eval_lte (189-192)
├── Boolean operations:
│   ├── eval_and (114-119)
│   ├── eval_or (121-126)
│   └── eval_not (128-132)
├── Accessor evaluation:
│   └── eval_accessor_for_replay (194-240)
├── Stack helpers:
│   ├── pop_pair (242-246)
│   ├── pop_i64_pair (248-252)
│   ├── expect_bool_replay (254-261)
│   └── expect_i64_replay (263-270)
└── TESTS (272-2101)
    ├── Stack tests (366-432)
    ├── Arithmetic tests (512-772)
    ├── Comparison tests (774-950)
    ├── Boolean tests (952-1256)
    ├── LoadSlot tests (1258-1336)
    ├── LoadConst tests (1338-1440)
    ├── LoadAccessor tests (1442-1643)
    ├── Unsupported ops tests (1645-1673)
    ├── Integration tests (1675-1839)
    └── BLACKHAT security regression tests (1841-2101)
```

### Problem:
- 20+ distinct operations crammed into one file
- Replay is a WORKFLOW pattern - operations should be discrete
- Violates Single Responsibility Principle
- Impossible to navigate, review, or test in isolation

---

## VIOLATION #3: PRIMITIVE OBSESSION

### Finding PO-1: Raw `i64` Tuples as Operands
**Location:** Lines 134-192 (arithmetic/comparison ops)  
**Current:**
```rust
pub(crate) fn eval_add(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;  // (i64, i64) - raw primitives
    let result = left.checked_add(right)...;
}
```
**Issue:** No domain type for "numeric operand pair" - raw `(i64, i64)` lacks semantic meaning.

### Finding PO-2: `StepIdx::ZERO` as Error Placeholder
**Location:** Lines 138, 149, 159, 169, 257, 267  
**Current:**
```rust
Err(ReplayError::ExpressionEvalFailed {
    step: StepIdx::ZERO,  //-placeholder, not actual step context
})
```
**Issue:** `StepIdx::ZERO` is used as a dummy value in error contexts where the actual step context is unavailable. This conflates error reporting with domain indexing.

### Finding PO-3: Untyped Index Indices
**Location:** Throughout operations using `SlotIdx`, `AccessorIdx`, `ConstIdx`  
**Current:**
```rust
fn eval_load_slot(run: &RunFrame, slot: SlotIdx, stack: &mut ReplayExprStack, ...)
fn eval_load_const(plan: &CompiledWorkflow, constant: ConstIdx, ...)
```
**Issue:** While these are newtype wrappers, the operations treat them generically. No per-operation domain models (e.g., `SlotValue`, `ConstRef`).

### Finding PO-4: No Value Objects for Expression Results
**Current:**
```rust
fn eval_eq(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    stack.push(SlotValue::Bool(left == right))  // Bool wrapped in SlotValue
}
```
**Issue:** No `ComparisonResult`, `ArithmeticResult`, or `LogicalResult` value objects. Raw `SlotValue` used throughout.

---

## VIOLATION #4: WORKFLOW BOUNDARY VIOLATION

### Current Architecture:
```
replay/
├── mod.rs        (11.0K)
├── ops.rs        (72.5K) ← monolithic
├── step.rs       (20.1K)
├── step_tests.rs (66.0K)
├── tests.rs      (134.3K)
└── choose/
```

### Issue:
- `ops.rs` is 7x the size limit
- Other replay modules are reasonable size
- `ops.rs` is an outlier requiring urgent decomposition

---

## MANDATORY REFACTOR PLAN

### Phase 1: Split Operations by Category

Create `crates/vb_core/src/replay/ops/` directory:

```
replay/ops/
├── mod.rs           (dispatch + re-exports)
├── arithmetic.rs    (Add, Sub, Mul, Div - ~50 lines)
├── comparison.rs   (Gt, Gte, Lt, Lte - ~40 lines)
├── logical.rs      (And, Or, Not - ~40 lines)
├── equality.rs    (Eq, NotEq - ~20 lines)
├── load.rs        (LoadSlot, LoadConst, LoadAccessor - ~60 lines)
├── accessor.rs    (eval_accessor_for_replay - ~50 lines)
├── stack.rs       (pop_pair, pop_i64_pair, expect_* - ~40 lines)
└── tests/         (extracted from ops.rs)
    ├── arithmetic_tests.rs
    ├── comparison_tests.rs
    ├── logical_tests.rs
    ├── equality_tests.rs
    ├── load_tests.rs
    ├── accessor_tests.rs
    └── blackhat_tests.rs
```

### Phase 2: Introduce Value Objects

```rust
// arithmetic.rs - after refactor
pub struct NumericOperand {
    left: i64,
    right: i64,
}

impl NumericOperand {
    pub fn add(self) -> Result<i64, ArithmeticError> {
        self.left.checked_add(self.right)
            .ok_or(ArithmeticError::Overflow)
    }
}
```

### Phase 3: Fix StepIdx Placeholder

```rust
// Currently: Err(ReplayError::ExpressionEvalFailed { step: StepIdx::ZERO })
// Should be: Err(ReplayError::ExpressionEvalFailed { step: current_step })
```

---

## RISK ASSESSMENT

| Risk | Level | Impact |
|------|-------|--------|
| Navigation | CRITICAL | 2101-line file impossible to navigate |
| Testing | HIGH | Cannot isolate test categories |
| Review | HIGH | Architectural debt blocks future work |
| Maintenance | CRITICAL | Any change requires reading entire file |
| Compile Time | MEDIUM | Incremental compilation ineffective |

---

## EVIDENCE

```
$ wc -l crates/vb_core/src/replay/ops.rs
2101 crates/vb_core/src/replay/ops.rs

$ cloc crates/vb_core/src/replay/ops.rs
      2101 total
      272 production / 1829 tests
      13% code / 87% tests
```

---

## RECOMMENDATION

**IMMEDIATE REFACTOR REQUIRED**

1. Create `replay/ops/` module tree
2. Extract each operation category into separate files
3. Extract tests into `tests/` subdirectory
4. Introduce value objects for arithmetic/comparison results
5. Fix `StepIdx::ZERO` placeholder usage
6. Target: Each file < 300 lines

**ESTIMATED REDUCTION:**
- Current: 2101 lines in 1 file
- Target: ~500 lines across 8 files (operations) + ~1600 lines in tests

---

## COMPLIANCE GATE

Before this file can be merged:
- [ ] `ops.rs` deleted and replaced with `ops/` directory
- [ ] All operation functions < 300 lines each
- [ ] No `StepIdx::ZERO` in error placeholders
- [ ] Value objects introduced for arithmetic operations
- [ ] All tests pass
- [ ] BLACKHAT security tests still pass

---

**Report Generated:** 2026-05-29  
**Enforcer:** architectural-drift  
**Status:** ACTION REQUIRED
