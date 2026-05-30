# Architectural Drift Report: `gate_11_loop.rs`

**File:** `crates/vb_validate/src/gate_11_loop.rs`
**Total Lines:** 636
**Enforcement Action:** ZERO-TOLERANCE — FILE EXCEEDS 300-LINE LIMIT BY 112%

---

## EXECUTIVE SUMMARY

This file is a **PRIMITIVE OBSESSION HALL OF SHAME** and a **SINGLETON GOD-MODULE** violation. Every validation rule is implemented with raw `usize` comparisons and stringly-typed labels instead of domain types. The test module alone is 470 lines — larger than the entire validation logic.

---

## VIOLATION 1: File Size (CRITICAL)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 636 | 300 | **FAIL** (+112%) |
| Production code | 166 | 300 | PASS |
| Test code | 470 | — | ISOLATE REQUIRED |

**Required Action:** Extract test module to `gate_11_loop_test.rs` in same directory, use `#[cfg(test)] mod tests;` pattern.

---

## VIOLATION 2: Primitive Obsession — `label: &str`

**Location:** Lines 94-109

```rust
fn check_step_in_range(
    step: StepIdx,
    node_count: usize,        // VIOLATION: raw primitive
    source_index: usize,      // VIOLATION: raw primitive
    label: &str,              // VIOLATION: stringly-typed
) -> ValidationResult<()>
```

**Problem:** The `label` parameter is a raw `&str` passed from call sites like:
- `"for_each body"`
- `"for_each done"`
- `"together branch {bi}"`
- `"loop body must be after loop start"`

**Scott Wlaschin Violation:** This is textbook primitive obsession. These labels encode **domain roles** (`LoopRole`, `StepLabel`, `GraphPosition`).

**Fix:** Introduce a labeled domain type:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepLabel {
    ForEachBody,
    ForEachDone,
    ForEachNextBody,
    ForEachNextDone,
    TogetherBranch(usize),
    TogetherJoin,
    CollectBody,
    CollectDone,
    // ... etc
}
```

---

## VIOLATION 3: Primitive Obsession — `node_count: usize`

**Location:** Throughout, e.g., lines 11, 100, 117

```rust
let node_count = parts.nodes.len();  // returns usize
if step.as_usize() >= node_count {   // raw comparison
```

**Problem:** `usize` has no domain meaning. `node_count` represents a **graph size** with invariants (e.g., must be > 0, must be < StepIdx::MAX).

**Fix:** Wrap in a domain type:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeCount(usize);

impl NodeCount {
    pub fn new(n: usize) -> Self { Self(n) }
    pub fn get(self) -> usize { self.0 }
    pub fn contains(self, step: StepIdx) -> bool {
        step.as_usize() < self.0
    }
}
```

---

## VIOLATION 4: Primitive Obsession — `source_index: usize`

**Location:** Throughout, e.g., line 97

```rust
fn check_step_in_range(
    // ...
    source_index: usize,  // VIOLATION
```

**Problem:** This is a **node index**, not a raw integer. It has domain meaning — it's always relative to the workflow graph.

**Fix:** Use `NodeIdx` wrapper type (already partially used via `StepIdx` pattern in the codebase):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeIdx(u32);
```

---

## VIOLATION 5: Primitive Obsession — `body_usize`, `done_usize`

**Location:** Lines 117-134

```rust
let body_usize = body.as_usize();
let done_usize = done.as_usize();
if body_usize <= start_index {
    // ...
}
if done_usize <= body_usize {
    // ...
}
```

**Problem:** Converting `StepIdx` to raw `usize` and comparing is a sign the domain logic is leaking. The comparisons `body <= start_index` and `done <= body` are **graph ordering constraints** that should be expressed at the type level.

**Fix:** Implement `StepIdx::is_after()`, `StepIdx::is_between()`:

```rust
impl StepIdx {
    pub fn is_after(self, other: StepIdx) -> bool {
        self.as_usize() > other.as_usize()
    }
    
    pub fn is_between(self, start: StepIdx, end: StepIdx) -> bool {
        self.as_usize() > start.as_usize() && self.as_usize() < end.as_usize()
    }
}
```

---

## VIOLATION 6: Stringly-Typed Error Messages

**Location:** Lines 124, 132, 152, 160

```rust
label: "loop body must be after loop start".to_owned(),
label: "loop done must be after loop body".to_owned(),
label: format!("together branch {bi} must be after start"),
label: format!("together join must be after branch {bi}"),
```

**Problem:** Using `format!` for error labels is a code smell. The error context should be a structured enum, not a string.

**Fix:** Expand `ValidationError::LoopBodyStepOutOfRange` to carry structured context:

```rust
pub enum LoopPosition {
    Body { node: NodeIdx },
    Done { node: NodeIdx },
    Branch { index: usize, node: NodeIdx },
    Join { node: NodeIdx },
}
```

---

## VIOLATION 7: Test Code Bloat — 470 Lines

**Location:** Lines 167-636

**Problem:** The test module is **larger than the entire production codebase**. This violates single responsibility. Test code should be in `gate_11_loop_test.rs` or a `tests/` integration module.

**Evidence:**
- 25 test functions (some 50+ lines each with verbose node construction)
- Redundant helper functions: `make_parts`, `nop_node`, `finish_node`
- Test for `TogetherJoin` that does nothing: line 39

**Fix:** Move to `gate_11_loop_test.rs`. Use a test builder pattern:

```rust
fn for_each_start(body: u16, done: u16) -> CompiledNodeKind {
    CompiledNodeKind::ForEachStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: 10,
        body: StepIdx::new(body),
        done: StepIdx::new(done),
    }
}
```

---

## VIOLATION 8: No Value Objects for Loop Configuration

**Location:** Lines 14-87 (repeated pattern)

```rust
CompiledNodeKind::ForEachStart { body, done, .. } => {
    check_step_in_range(*body, node_count, index, "for_each body")?;
    check_step_in_range(*done, node_count, index, "for_each done")?;
    check_loop_span(index, *body, *done, node_count)?;
}
```

**Problem:** Every loop kind has body/done semantics but they're handled individually. This is a **repeated case statement** that should use **polymorphism** or a **visitor pattern**.

**Fix:** Introduce a `LoopNode` trait:

```rust
pub trait LoopNode {
    fn body(&self) -> StepIdx;
    fn done(&self) -> StepIdx;
    fn role_label(&self) -> StepLabel;
}
```

---

## RESPONSIBILITY MAP

| Function | Lines | Responsibility | Status |
|----------|-------|----------------|--------|
| `validate_gate_11_loop_body_graph` | 10-92 | Main dispatcher | OK |
| `check_step_in_range` | 94-109 | Single step validation | PRIMITIVE OBSESSION |
| `check_loop_span` | 111-136 | Loop body/done ordering | PRIMITIVE OBSESSION |
| `check_together_span` | 138-165 | Branch/join ordering | PRIMITIVE OBSESSION |
| `mod tests` | 167-636 | All test cases | ISOLATE REQUIRED |

---

## REQUIRED REFACTORING

1. **Extract test module** → `gate_11_loop_test.rs`
2. **Create `StepLabel` enum** for all label variants
3. **Create `NodeCount` wrapper** around `usize`
4. **Create `NodeIdx` wrapper** around `u32`
5. **Add domain methods on `StepIdx`** for ordering checks
6. **Remove `format!` from error labels** → structured `LoopPosition` enum
7. **Implement `LoopNode` trait** to unify body/done handling

---

## VERDICT

**ARCHITECTURAL DRIFT: CONFIRMED**

This file commits multiple mortal sins:
- 636 lines (212% of limit)
- 6 distinct primitive obsession violations
- 470-line test monolith
- Stringly-typed error context

**Recommended Action:** Reject for landing. Return to author with refactoring mandate.
