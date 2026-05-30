# Architectural Drift Report: `resource_budget.rs`

**File:** `crates/vb_proof_kernels/src/resource_budget.rs`  
**Total Lines:** 1028  
**Line Limit:** 300  
**Violation:** CRITICAL — 342% of allowed limit (728 lines over)

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Actual lines | 1028 |
| Allowed | 300 |
| Excess | 728 lines |
| % Over limit | 342% |

**Required Action:** File MUST be split into at least 4 separate modules.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Budget Struct — 12 Naked `u64` Fields

```rust
pub struct Budget {
    pub steps: u64,           // ❌ StepCount(u64)
    pub actions: u64,         // ❌ ActionCount(u64)
    pub parallel: u64,       // ❌ ParallelDegree(u64)
    pub retries: u64,        // ❌ RetryCount(u64)
    pub gather_pages: u64,   // ❌ GatherPageCount(u64)
    pub gather_items: u64,   // ❌ GatherItemCount(u64)
    pub for_each_iters: u64, // ❌ ForEachIterCount(u64)
    pub together_branches: u64, // ❌ TogetherBranchCount(u64)
    pub repeat_attempts: u64,   // ❌ RepeatAttemptCount(u64)
    pub run_time_secs: u64,  // ❌ RunTimeSecs(u64)
    pub result_bytes: u64,   // ❌ ResultByteCount(u64)
    pub slots_written: u64,   // ❌ SlotsWrittenCount(u64)
}
```

**Scott Wlaschin Violation:** Every field is a raw `u64`. This is textbook primitive obsession. Each budget dimension should be a NewType wrapper that makes illegal states unrepresentable and prevents mixing dimensions (e.g., `Steps(5) + Parallel(3)` should be a type error).

### 2.2 Policy Struct — 5 Naked `u64` Fields

```rust
pub struct Policy {
    pub max_actions: u64,       // ❌ MaxActions(Budget<ActionCount>)
    pub max_parallel: u64,      // ❌ MaxParallel(Budget<ParallelDegree>)
    pub max_run_time: u64,      // ❌ MaxRunTime(Budget<RunTimeSecs>)
    pub max_result_bytes: u64,  // ❌ MaxResultBytes(Budget<ResultByteCount>)
    pub max_steps: u64,         // ❌ MaxSteps(Budget<StepCount>)
}
```

**Same violation.** Policy thresholds should be typed to match the budget dimensions they constrain.

---

## 3. SEMANTIC OPERATION CONFUSION

### 3.1 `sequential_add` Mixed Semantics

```rust
pub fn sequential_add(&mut self, other: &Budget) {
    self.steps = self.steps.saturating_add(other.steps);      // ✓ additive
    self.actions = self.actions.saturating_add(other.actions); // ✓ additive
    self.parallel = self.parallel.max(other.parallel);         // ❌ max?
    self.retries = self.retries.max(other.retries);           // ❌ max?
    self.gather_pages = self.gather_pages.saturating_add(other.gather_pages); // ✓ additive
    self.gather_items = self.gather_items.saturating_add(other.gather_items); // ✓ additive
    self.for_each_iters = self.for_each_iters.max(other.for_each_iters);      // ❌ max?
    self.together_branches = self.together_branches.max(other.together_branches); // ❌ max?
    self.repeat_attempts = self.repeat_attempts.max(other.repeat_attempts);    // ❌ max?
    self.run_time_secs = self.run_time_secs.saturating_add(other.run_time_secs); // ✓ additive
    self.result_bytes = self.result_bytes.max(other.result_bytes);            // ❌ max?
    self.slots_written = self.slots_written.saturating_add(other.slots_written); // ✓ additive
}
```

**Problem:** The function conflates two distinct operations under one name. Additive fields represent **sum accumulation** (sequential execution adds time/resources). Max fields represent **parallel resource allocation** (branching takes the max of each dimension). These should be separate functions: `sequential_add` (pure sum) and `branch_max` (already exists but is named differently).

**Correct naming:** `sequential_add` should ONLY use `saturating_add`. The max fields belong in `branch_max`, not `sequential_add`.

---

## 4. EXCESSIVE TEST DENSITY

| Section | Lines | Issue |
|---------|-------|-------|
| `Budget` impl | 6–71 | 66 lines production code |
| `Policy` impl | 73–112 | 40 lines production code |
| Free functions | 114–130 | 17 lines production code |
| **Tests** | **132–1028** | **897 lines** |
| **Test/Code ratio** | **~14:1** | **Overkill** |

**Observation:** The 897-line test suite is 11× longer than the 81-line implementation. While thorough, this violates single-responsibility: tests should live in `resource_budget_test.rs` or a `tests/` submodule, not inline.

---

## 5. PROPOSED REFACTORING PLAN

### 5.1 Split Into Modules

```
vb_proof_kernels/src/
├── lib.rs
├── resource_budget/
│   ├── mod.rs          (reexports)
│   ├── types.rs        (Budget, Policy newtypes)        ~80 lines
│   ├── operations.rs   (sequential_add, branch_max, loop_mul) ~60 lines
│   ├── composition.rs  (sequential_compose, branch_compose, loop_compose) ~20 lines
│   ├── policy.rs       (within, default_policy)         ~40 lines
│   └── tests/
│       └── integration_tests.rs   ~200 lines (condensed)
```

### 5.2 NewType Wrappers

```rust
// types.rs
#[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, PartialEq, Eq)]
pub struct Steps(u64);
#[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, PartialEq, Eq)]
pub struct Actions(u64);
#[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParallelDegree(u64);
// ... etc for all 12 dimensions

pub struct Budget {
    pub steps: Steps,
    pub actions: Actions,
    pub parallel: ParallelDegree,
    // ...
}

pub struct Policy {
    pub max_actions: MaxActions,
    pub max_parallel: MaxParallel,
    // ...
}
```

### 5.3 Semantic Function Naming

Rename `sequential_add` to `sequential_merge` (sum only), or remove max operations from it entirely.

---

## 6. VERDICT

| Violation | Severity |
|-----------|----------|
| Line count (1028 > 300) | **CRITICAL** |
| Primitive obsession (17 u64 fields) | **HIGH** |
| Semantic confusion in operations | **MEDIUM** |
| Test/implementation ratio | **LOW** ( organizational) |

**STATUS:** `ARCH-DRIFT-REFACTOR-REQUIRED`

**Priority 1:** Split file into 4+ modules  
**Priority 2:** Create NewType wrappers for all budget dimensions  
**Priority 3:** Fix `sequential_add` to not mix additive and max semantics
