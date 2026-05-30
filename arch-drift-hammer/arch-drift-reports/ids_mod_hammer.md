# ARCH-DRIFT REPORT: `crates/vb_core/src/ids/mod.rs`

**File**: `crates/vb_core/src/ids/mod.rs`
**Total Lines**: 1098
**Status**: `VIOLATION — MUST REFACTOR`

---

## 1. LINE COUNT VIOLATION

| Rule | Limit | Actual |
|------|-------|--------|
| Max lines per `.rs` file | 300 | 1098 |

**Severity**: CRITICAL

The file is **3.66× over the line limit**. It must be split.

### Breakdown

| Section | Lines | Category |
|---------|-------|----------|
| Macro definitions + ID types + impl blocks | 1–356 | PRODUCTION CODE |
| Unit tests (inline `#[cfg(test)]`) | 358–1092 | TEST CODE |
| Kani module stubs | 1094–1098 | VERIFICATION GATE |

The production code (lines 1–356) is **within budget** (~356 lines < 300 is still over, but closer).

The inline tests (734 lines) and empty kani stubs (5 lines) blow it way over.

---

## 2. DDD / PRIMITIVE OBSESSION ANALYSIS

### 2.1 What the file does RIGHT (Wlaschin Newtype Pattern)

The `numeric_id!` macro is **excellent** — it implements the canonical Wlaschin "making illegal states unrepresentable" pattern:

```rust
macro_rules! numeric_id {
    ($name:ident, $inner:ty, $accessor:ident) => {
        #[repr(transparent)]
        pub struct $name($inner);
        impl $name {
            pub const fn new(value: $inner) -> Self { Self(value) }
            pub const fn $accessor(self) -> $inner { self.0 }
        }
        impl FromStr for $name { ... }
    };
}
```

This correctly fights primitive obsession by wrapping:
- `WorkflowId(u32)`, `SymbolId(u32)`, `ListId(u32)`, `ObjectId(u32)`
- `StepIdx(u16)`, `SlotIdx(u16)`, `ExprIdx(u16)`, `ActionId(u16)`, `AccessorIdx(u16)`, `ConstIdx(u16)`
- `BlobId(u64)`, `RunId(u64)`, `EventSeq(u64)`, `SeqNo(u64)`

And the manually-defined newtypes (`BranchIdx`, `FanoutLimit`, `MaxAttempts`, `RetryCount`, `BranchCount`, `WorkflowDigest`) are all proper domain types.

### 2.2 Remaining Primitive Obsession Observations

#### ISSUE 1: `BranchIdx::new` accepts any `u16` — no zero-cost validation

`BranchIdx` is an index into branches of a `Together` block. The first branch is index 0. But `new(value: u16)` accepts ANY `u16` including values that may exceed the actual branch count at runtime.

**Contrast with `MaxAttempts::try_new`** which correctly rejects 0:
```rust
pub fn try_new(value: u16) -> Result<Self, super::errors::EngineError> {
    if value == 0 {
        return Err(super::errors::EngineError::InternalInvariantViolation {
            reason: "max_attempts_cannot_be_zero",
        });
    }
    Ok(Self(value))
}
```

`BranchIdx` should arguably have a `try_new` variant that validates against an actual `BranchCount`, or at minimum document the invariant.

#### ISSUE 2: `FanoutLimit::new` accepts 0 — may be invalid in context

The docstring says "A limit of 0 means no items are allowed" — but if `ForEach` is invoked with `FanoutLimit(0)`, is that a user error or a silent no-op? The semantic is ambiguous. Consider whether `FanoutLimit(0)` should be a `try_new` pattern.

#### ISSUE 3: `BranchCount::new` accepts 0 — ambiguous for `Together`

If a `Together` block has 0 branches, is that valid? The `Together` construct with no branches seems meaningless. `BranchCount::new` should potentially be `try_new` as well.

#### ISSUE 4: `RetryCount::new` accepts any `u16` including 0

`RetryCount` represents the *current* attempt number. Starting at 0 is fine, but is `RetryCount(u16::MAX)` ever valid before overflow? The `next()` saturates, which is fine, but the domain semantics are unclear.

#### ISSUE 5: `WorkflowDigest::from_bytes` accepts ANY 32 bytes

`WorkflowDigest([u8; 32])` wraps a SHA-256 output. It has no validation that the bytes represent a valid digest. If used as a key in a map, this is fine (any 32 bytes are valid). But if there's an expectation that `WorkflowDigest` must be a *valid* SHA-256 output (not just any 32 bytes), there's no enforcement.

### 2.3 `checked_index!` Macro Observations

The `checked_index!` macro adds `as_usize()` to index types (`StepIdx`, `SlotIdx`, `ExprIdx`, `AccessorIdx`, `ConstIdx`). This is appropriate — these are used for slice indexing and the conversion is checked/safe. Good pattern.

---

## 3. REFACTOR PLAN (Required)

### Step 1: Extract tests to `tests.rs` (or `ids/tests/mod.rs`)

Move all `#[cfg(test)]` content (lines 358–1092) to a separate test file:
- `crates/vb_core/src/ids/tests.rs` — all inline tests
- OR `crates/vb_core/src/ids/tests/mod.rs` with individual test modules

This reduces `mod.rs` from 1098 → ~357 lines (still over 300, see Step 2).

### Step 2: Split production code across submodules

The ~357 production lines should be split:

```
ids/
├── mod.rs      (~100 lines: re-exports + numeric_id! macro + checked_index! macro)
├── workflow.rs (~80 lines: WorkflowId, WorkflowDigest)
├── run.rs      (~80 lines: RunId, SeqNo, EventSeq)
├── step.rs     (~80 lines: StepIdx, SlotIdx, ConstIdx, ExprIdx, AccessorIdx, ActionId)
├── blob.rs     (~50 lines: BlobId, ListId, ObjectId, SymbolId)
├── branch.rs   (~50 lines: BranchIdx, BranchCount, FanoutLimit, MaxAttempts, RetryCount)
└── tests.rs    (~735 lines: all tests — or keep as inline #[cfg(test)] mod tests)
```

### Step 3: Address semantic validation gaps

| Type | Current | Suggested |
|------|---------|-----------|
| `BranchIdx::new` | `const fn new(u16) -> Self` | Add `try_new(branch_count: BranchCount, value: u16) -> Result<Self, EngineError>` or at minimum document invariant |
| `FanoutLimit::new` | `const fn new(u32) -> Self` | Consider `try_new(u32) -> Result<Self, EngineError>` rejecting 0 if 0-items-forEach is a domain error |
| `BranchCount::new` | `const fn new(u16) -> Self` | Consider `try_new(u16) -> Result<Self, EngineError>` rejecting 0 |
| `RetryCount::new` | `const fn new(u16) -> Self` | Document whether `RetryCount(u16::MAX)` is a valid pre-state |

---

## 4. VERDICT

| Category | Status |
|----------|--------|
| Line Count | ❌ FAIL (1098 > 300) |
| Primitive Obsession | ✅ PASS (newtypes throughout) |
| Wlaschin DDD | ⚠️ PARTIAL (semantic validation gaps) |
| `Parse, don't validate` | ✅ PASS (FromStr on all numeric IDs) |
| Zero-unsafe | ✅ PASS (`#![forbid(unsafe_code)]`) |

**IMMEDIATE ACTION REQUIRED**: Split the file. The line count violation is non-negotiable. The domain validation gaps are recommended fixes, not blockers.

---

## 5. EVIDENCE COMMANDS

```bash
# Line count verification
wc -l /home/lewis/src/velvet-ballistics/crates/vb_core/src/ids/mod.rs
# Expected: 1098

# Confirm no unsafe usage
grep -n 'unsafe' /home/lewis/src/velvet-ballistics/crates/vb_core/src/ids/mod.rs
# Expected: none

# Confirm all IDs are newtype-wrapped (not raw primitives)
grep -E 'pub struct (WorkflowId|StepIdx|SlotIdx|ExprIdx|ActionId|AccessorIdx|ConstIdx|SymbolId|ListId|ObjectId|BlobId|RunId|EventSeq|SeqNo|BranchIdx|FanoutLimit|MaxAttempts|RetryCount|BranchCount|WorkflowDigest)' /home/lewis/src/velvet-ballistics/crates/vb_core/src/ids/mod.rs
# Expected: all 20 types listed above, each as #[repr(transparent)] pub struct
```
