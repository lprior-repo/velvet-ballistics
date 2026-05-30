# Architectural Drift Report: `primitives/helpers.rs`

**File**: `crates/vb_runtime/src/primitives/helpers.rs`
**Line count**: 526 (VIOLATION: > 300 limit)
**Status**: 🔨 HAMMER REQUIRED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 526 | 300 | 🔴 OVER |
| Production code | ~86 | — | — |
| Test code | ~440 | — | — |

The test module alone (lines 87–526) is **440 lines** — larger than the entire budget for a single file. This is structural rot.

---

## 2. RESPONSIBILITY MAP

### Production Helpers (lines 1–85)

| Function | Responsibility | Domain | Violation |
|----------|----------------|--------|-----------|
| `expect_list` | Type narrow — extract `ListId` from `SlotValue` | List domain | Primitive obsession: raw `SlotValue` in, raw `ListId` out |
| `empty_list` | Construct empty list slice | List domain | Primitive obsession: returns `Box<[SlotValue]>`, not a `List` newtype |
| `tail_items` | Extract tail of list slice | List domain | Primitive obsession: operates on raw `&[SlotValue]`; manual index arithmetic |
| `jump_to` | Set PC + increment executed | Control flow | Workflow smear: belongs on `RunFrame` as a method |
| `jump_to_body` | Conditionally mark pending then jump | Control flow | Workflow smear: two concerns crammed into free function |
| `jump_to_next` | Unwrap `Option<StepIdx>` or error | Control flow | Primitive obsession: `Option` unwrap logic rather than typed transition |
| `require_output` | Unwrap `Option<SlotIdx>` or error | Control flow | Primitive obsession: same `Option` pattern — zero domain semantics |

### Test Block (lines 87–526)

440 lines of tests for ~80 lines of production code. Ratio is **5.5:1**. This is a red flag: the helpers themselves are poorly abstracted, requiring excessive test coverage to reason about.

---

## 3. DDD VIOLATIONS

### 3.1 Primitive Obsession (Scott Wlaschin — "Make illegal states unrepresentable")

#### `tail_items` — Raw slice arithmetic
```rust
pub(crate) fn tail_items(items: &[SlotValue]) -> Result<Box<[SlotValue]>, EngineError>
```
- Takes `&[SlotValue]` — a raw Rust slice, not a domain list type
- Manually computes `tail_len = len() - 1` with checked arithmetic
- Manually loops with index `1..len()` instead of using `&items[1..]`
- Returns `Box<[SlotValue]>` — a raw boxed slice, not a `List` value object

**Should be**: A `List` (or `ListSlice`) newtype with a `.tail()` method that returns `List`. The arithmetic is already validated by Rust's slice semantics; only the empty-list case needs a branch.

#### `expect_list` — Type tag extraction, not parsing
```rust
pub(crate) fn expect_list(value: SlotValue) -> Result<ListId, EngineError>
```
- "Parse, don't validate" would mean: parse `SlotValue::List(id)` into a domain-typed `ListRef` that **carries** the `ListId` and guarantees list-ness through the type system
- Current design: every caller receives a raw `ListId` and must re-validate on every use

#### `empty_list` — Returns untyped slice
```rust
pub(crate) fn empty_list() -> Box<[SlotValue]>
```
- A `Box<[SlotValue]>` carries zero domain semantics
- Should return a proper `List` newtype (potentially zero-length variant)

### 3.2 Workflow Smear (free functions over domain methods)

`jump_to`, `jump_to_body`, `jump_to_next` operate on `RunFrame` but live in a helper module:

```rust
pub(crate) fn jump_to(run: &mut RunFrame, target: StepIdx) -> Result<vb_core::EngineSignal, EngineError>
```

**DDD Principle violated**: Behavior belongs to the aggregate root. `RunFrame` is the execution context — these control-flow transitions should be `RunFrame` methods:
- `RunFrame::jump_to(target)`
- `RunFrame::jump_to_body(body)`
- `RunFrame::jump_to_next(next, step)`

Free functions scatter domain logic across the codebase. When `RunFrame` changes, you must hunt helpers across multiple files.

### 3.3 `require_output` — Trivial wrapper with no domain meaning

```rust
pub(crate) fn require_output(output: Option<SlotIdx>, step: StepIdx) -> Result<SlotIdx, EngineError>
```

This is `output.ok_or(Error::MissingOutputSlot { step })`. It adds no behavior — it only adds a location in the code. Callers should use `Option::ok_or_else` directly or the error variant should be `From<Option<T>>` implementable.

---

## 4. PRESCRIBED REFACTOR

### File Split Plan

```
primitives/
├── mod.rs                      # unchanged (add new mod lines)
├── helpers/                    # NEW — split helper module
│   ├── mod.rs                  # pub mod list_helpers; pub mod jump_helpers;
│   ├── list_helpers.rs         # expect_list, empty_list, tail_items (~50 lines)
│   ├── jump_helpers.rs         # jump_to, jump_to_body, jump_to_next, require_output (~35 lines)
│   └── tests/
│       └── helpers_tests.rs    # extracted 440-line test block
└── helpers.rs                 # DELETE after split
```

### Step 1 — Create `helpers/` directory and `helpers/mod.rs`

```rust
#![forbid(unsafe_code)]
//! Extracted helpers — split from helpers.rs (was 526 lines)

pub(crate) mod list_helpers;
pub(crate) mod jump_helpers;

#[cfg(test)]
pub(crate) mod helpers_tests;
```

### Step 2 — Extract `list_helpers.rs`

Move `expect_list`, `empty_list`, `tail_items`. Tests move to `helpers_tests.rs`.

### Step 3 — Extract `jump_helpers.rs`

Move `jump_to`, `jump_to_body`, `jump_to_next`, `require_output`. Tests move to `helpers_tests.rs`.

### Step 4 — Update `primitives/mod.rs`

Replace:
```rust
pub(crate) mod helpers;
```
With:
```rust
pub(crate) mod helpers;
```

### Step 5 — Delete original `helpers.rs`

### Step 6 — Domain Types to Add (follow-up beads)

| NewType | File | Rationale |
|---------|------|-----------|
| `List<'a>` | `vb_core` or `vb_runtime::primitives` | Wraps `ListId + &[SlotValue]`; `.tail()` method |
| `NonEmptyList<'a>` | `vb_core` or `vb_runtime::primitives` | Guarantees at least one element; no tail edge case |
| `RunFrame::jump_to(target)` | `RunFrame` impl | Eliminates free function workflow smear |
| `RunFrame::jump_to_body(body)` | `RunFrame` impl | Same |
| `RunFrame::jump_to_next(next, step)` | `RunFrame` impl | Same |

---

## 5. SUMMARY

| Violation | Severity | Lines Affected |
|-----------|----------|----------------|
| Line count > 300 | 🔴 CRITICAL | 526 total (226 over) |
| Test block = 440 lines | 🔴 CRITICAL | 87–526 |
| Primitive obsession: `tail_items` | 🟡 MODERATE | 23–49 |
| Primitive obsession: `expect_list` | 🟡 MODERATE | 9–17 |
| Primitive obsession: `empty_list` | 🟡 MODERATE | 19–21 |
| Workflow smear: jump helpers | 🟡 MODERATE | 51–78 |
| Workflow smear: `require_output` | 🟢 LOW | 80–85 |

---

## 6. VERDICT

**🔴 UNACCEPTABLE** — File is 75% over the line limit. The test module is grotesquely oversized and the production helpers violate every Scott Wlaschin principle: primitive obsession, workflow-as-methods, and parse-don't-validate.

**Mandatory actions**:
1. Split into `list_helpers.rs`, `jump_helpers.rs`, `helpers_tests.rs` under `primitives/helpers/` — **THIS SESSION**
2. File follow-up bead: Add `List` / `NonEmptyList` newtypes — **NEXT SESSION**
3. File follow-up bead: Migrate jump helpers to `RunFrame` methods — **NEXT SESSION**
