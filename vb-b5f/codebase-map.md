# Phase 1 Codebase Map — Core Types (vb-b5f)

**Bead:** vb-b5f
**Scope:** Phase 1 deliverables from `velvet-ballistics-MASTER.md`
**Generated:** 2026-04-29

---

## Overview

Phase 1 deliverable scope (per MASTER.md Section 16):
1. `ids.rs` — Compact numeric wrappers
2. `errors.rs` — Typed errors with stable diagnostic codes
3. `limits.rs` — New: resource limit constants
4. `span.rs` — New: source location tracking
5. `diagnostic.rs` — New: diagnostic code system
6. `value.rs` — Runtime slot values and taint

---

## 1. `ids.rs` — EXISTS, PARTIALLY COMPLETE

### File Location
`crates/vb-core/src/ids.rs`

### Types Present
| Type | Status | Notes |
|------|--------|-------|
| `WorkflowId` | ✅ Complete | u32 wrapper, `new()`, `get()` |
| `StepIdx` | ✅ Complete | u16 wrapper, `new()`, `as_usize()` |
| `SlotIdx` | ✅ Complete | u16 wrapper, `new()`, `as_usize()` |
| `ExprIdx` | ✅ Complete | u16 wrapper, `new()`, `as_usize()` |
| `ActionId` | ✅ Complete | u16 wrapper, `new()`, `get()` |
| `AccessorIdx` | ✅ Complete | u16 wrapper, `new()`, `as_usize()` |
| `ConstIdx` | ✅ Complete | u16 wrapper, `new()`, `as_usize()` |
| `RunId` | ⚠️ DISCREPANCY | Current: u128; Master spec: u64 |
| `WorkflowDigest` | ✅ Complete | [u8; 32] wrapper |

### What's Missing
- **`SeqNo`** (u64) — not present, needs addition
- **`ZERO` constants** — spec shows `RunId::ZERO`, `StepIdx::ZERO`, `SlotIdx::ZERO`; not present
- **`checked_add` methods** — spec calls for checked arithmetic, not yet implemented
- **`const` constructors** — `RunId::new()` is `const`, but `StepIdx::new()` and `SlotIdx::new()` are already `const` ✅

### Discrepancies
1. `RunId` is `u128` in code but `u64` per master spec (line 279 of MASTER.md)
2. No `SeqNo` type exists (spec line 307)
3. No `ZERO` constants (spec lines 326-334)

### Arc<Mutex Concern
**NO CONCERN** — No `Arc<Mutex>` usage found in `vb-core/src/`.

### Action Items
- [ ] Add `SeqNo` type
- [ ] Add `ZERO` constants to `RunId`, `StepIdx`, `SlotIdx`
- [ ] Add `checked_add`/`checked_sub` methods where arithmetic is needed
- [ ] Resolve `RunId` u128 vs u64 discrepancy (confirm with user which to use)

---

## 2. `error.rs` — EXISTS, NEEDS SIGNIFICANT EXTENSION

### File Location
`crates/vb-core/src/error.rs`

### Current State
```rust
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    InvalidProgramCounter { step: StepIdx },
    ConstOutOfBounds { constant: ConstIdx },
    SlotOutOfBounds { slot: SlotIdx },
    NonBoolCondition { slot: SlotIdx },
    StepBudgetExhausted,
    StepCounterOverflow,
    EmptyStepBudget,
}
```

### What's Missing
- **Name mismatch**: Code uses `EngineError`, spec uses `CoreError` (and file named `errors.rs`)
- **Stable diagnostic codes E0101-E0409**: Not implemented
- **Span + payload**: Errors don't carry source location
- **`CoreResult<T>` type alias**: Not present
- **Additional error variants** from spec:
  - `MissingNextStep { step: StepIdx }`
  - `ExprOutOfBounds { expr: ExprIdx }`
  - `TypeMismatch { expected: &'static str, found: &'static str }`
  - `DivisionByZero`
  - `NonFiniteNumber`
  - `QueueFull`
  - `ResourceLimitExceeded { resource: &'static str }`
  - `AllocationFailed`

### Discrepancies
1. File named `error.rs` but spec shows `errors.rs` (plural)
2. Enum named `EngineError` but spec shows `CoreError`
3. Missing diagnostic code mapping
4. Missing Span integration

### Action Items
- [ ] Rename `error.rs` → `errors.rs` (or keep but align naming)
- [ ] Rename `EngineError` → `CoreError`
- [ ] Add `CoreResult<T>` type alias
- [ ] Add missing error variants
- [ ] Add stable diagnostic codes (E0101-E0409 range)
- [ ] Integrate `Span` type once `span.rs` is created

---

## 3. `limits.rs` — **MISSING, NEEDS CREATION**

### File Location
`crates/vb-core/src/limits.rs` (does not exist)

### Required Constants (per spec)
```rust
MAX_STEPS: u32           // Maximum workflow steps
MAX_SLOTS: u32           // Maximum runtime slots
MAX_CONSTANTS: u32       // Maximum constant pool size
MAX_EXPRESSION_DEPTH: u32 // Maximum expression nesting
MAX_RUN_NAME_LENGTH: u32  // Maximum workflow name length
```

### Action Items
- [ ] Create `crates/vb-core/src/limits.rs`
- [ ] Define all limit constants with appropriate values
- [ ] Add `#![forbid(unsafe_code)]`
- [ ] Export from `lib.rs`

---

## 4. `span.rs` — **MISSING, NEEDS CREATION**

### File Location
`crates/vb-core/src/span.rs` (does not exist)

### Required Types (per spec)
```rust
pub struct Span {
    start: u32,
    end: u32,
}

pub struct Located<T> {
    value: T,
    span: Span,
}

pub struct Spanned<T> {
    value: T,
    span: Span,
}

pub struct SourceMap { /* ... */ }
```

### Action Items
- [ ] Create `crates/vb-core/src/span.rs`
- [ ] Define `Span`, `Located<T>`, `Spanned<T>` types
- [ ] Add `SourceMap` for mapping spans to source text
- [ ] Add `#![forbid(unsafe_code)]`
- [ ] Export from `lib.rs`

---

## 5. `diagnostic.rs` — **MISSING, NEEDS CREATION**

### File Location
`crates/vb-core/src/diagnostic.rs` (does not exist)

### Required Types (per spec)
```rust
pub struct DiagnosticCode(u16);

pub enum Severity {
    Error,
    Warning,
    Info,
}

pub struct Diagnostic {
    code: DiagnosticCode,
    severity: Severity,
    message: String,
    span: Option<Span>,
    // payload variant based on error code
}
```

### Diagnostic Code Ranges (per MASTER.md)
- **E0101-E0199**: YAML/parsing errors
- **E0201-E0299**: Schema validation errors
- **E0301-E0399**: Semantic/reference errors
- **E0401-E0409**: Runtime errors

### Action Items
- [ ] Create `crates/vb-core/src/diagnostic.rs`
- [ ] Define `DiagnosticCode`, `Severity`, `Diagnostic` types
- [ ] Map existing error variants to stable codes
- [ ] Add `#![forbid(unsafe_code)]`
- [ ] Export from `lib.rs`

---

## 6. `value.rs` — EXISTS, MOSTLY COMPLETE

### File Location
`crates/vb-core/src/value.rs`

### Types Present
| Type | Status | Notes |
|------|--------|-------|
| `Taint` | ✅ Complete | `Clean`, `Secret`, `DerivedFromSecret` |
| `SlotValue` | ⚠️ Needs extension | Missing `type_name()` method; has extra variants |

### Current `SlotValue` Variants (code)
```rust
pub enum SlotValue {
    Null,
    Bool(bool),
    I64(i64),
    Text(Box<str>),
    Bytes(Bytes),
    Object(Box<[(Box<str>, SlotValue)]>),  // extra, not in spec
    List(Box<[SlotValue]>),                // extra, not in spec
}
```

### Spec `SlotValue` Variants (MASTER.md lines 352-358)
```rust
pub enum SlotValue {
    Null,
    Bool(bool),
    I64(i64),
    Text(Box<str>),
    Bytes(bytes::Bytes),
    // Object and List NOT in spec (they're in the codebase already)
}
```

### What's Present But Not In Spec
- `Object` variant — present in codebase, not in spec's `value.rs` section
- `List` variant — present in codebase, not in spec's `value.rs` section

### What's Missing
- **`type_name()` method** — spec lines 362-369 show this method, not implemented

### Note
The `Object` and `List` variants exist in the codebase and appear to be used by `CompiledWorkflow` in `workflow.rs`. They may belong in a different module or the spec section is incomplete. The Phase 1 spec only calls out `Taint` and basic `SlotValue` types.

### Action Items
- [ ] Add `type_name()` method to `SlotValue`
- [ ] Clarify whether `Object`/`List` belong in `value.rs` or elsewhere

---

## Summary Matrix

| File | Status | Work Needed |
|------|--------|-------------|
| `ids.rs` | ✅ Exists | Extension: `SeqNo`, `ZERO` constants, `checked_add` |
| `errors.rs` | ⚠️ Exists | Major: rename `EngineError`→`CoreError`, add codes E0101-E0409, add `Span` |
| `limits.rs` | ❌ Missing | Full creation |
| `span.rs` | ❌ Missing | Full creation |
| `diagnostic.rs` | ❌ Missing | Full creation |
| `value.rs` | ✅ Exists | Extension: add `type_name()` |

---

## Key Discrepancies with Master Plan

1. **`RunId` type**: Code uses `u128`, spec shows `u64`
2. **Error naming**: `EngineError` vs `CoreError`
3. **Error file**: `error.rs` vs `errors.rs`
4. **Missing `SeqNo`** type entirely
5. **Missing `ZERO` constants** on ID types
6. **No diagnostic code system** implemented

---

## Dependencies

- `span.rs` must be created before `diagnostic.rs` (for `Span` type)
- `diagnostic.rs` needed by `errors.rs` for stable code mapping
- `limits.rs` is independent, can be created first

---

## No Arc<Mutex Concern

**Confirmed:** No `Arc<Mutex>` or other thread-locking primitives exist in `crates/vb-core/src/`. The codebase correctly uses lock-free structures and shard-owned state as specified.
