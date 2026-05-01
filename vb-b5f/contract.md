# Contract Specification — Phase 1: Core Types

**Bead:** vb-b5f
**Scope:** Foundational types for the hot runtime. No application logic.
**Status:** Contract-first; implementation not started.

---

## Context

- **Feature:** Phase 1 implementation of `vb_core` foundational types.
- **Domain terms:** `SeqNo`, `Span`, `DiagnosticCode`, `CoreError`, `SlotValue`, `Taint`, `RunId`, `StepIdx`, `SlotIdx`, `ConstIdx`, `AccessorIdx`, `ExprIdx`, `ActionId`, `WorkflowId`, `WorkflowDigest`.
- **Assumptions:**
  - `RunId` is `u64` per MASTER.md line 279 (discrepancy with current `u128` in code — code must be updated to match spec).
  - All new files live under `crates/vb-core/src/`.
  - Every file carries `#![forbid(unsafe_code)]`.
- **Open questions:**
  - None identified.

---

## Deliverable 1: `ids.rs` — Extend Existing

### Preconditions
- `ids.rs` exists at `crates/vb-core/src/ids.rs`.
- All existing ID types (`WorkflowId`, `StepIdx`, `SlotIdx`, `ExprIdx`, `ActionId`, `AccessorIdx`, `ConstIdx`, `RunId`, `WorkflowDigest`) are present.

### Postconditions

1. **`SeqNo(u64)` is added:**
   - `#[repr(transparent)]` wrapper around `u64`.
   - `SeqNo::ZERO = Self(0)` constant exists.
   - `SeqNo::MIN = Self(0)` and `SeqNo::MAX = Self(u64::MAX)` exist.
   - `checked_add(self, rhs: u64) -> Option<SeqNo>` method exists and returns `None` on overflow.

2. **`CheckedIndex` trait is added:**
   ```rust
   pub trait CheckedIndex {
       fn as_usize(self) -> usize;
   }
   ```
   Trait is implemented for `StepIdx`, `SlotIdx`, `ExprIdx`, `AccessorIdx`, `ConstIdx`.
   Each `as_usize()` converts the inner `uN` to `usize` via `usize::from()` — no bounds check is performed in the conversion (bounds checking is the caller's responsibility via `get()` or slice indexing).

3. **`ZERO` constants are added to:**
   - `RunId::ZERO = Self(0)` — `RunId` inner type changes from `u128` to `u64`.
   - `StepIdx::ZERO = Self(0)`.
   - `SlotIdx::ZERO = Self(0)`.

4. **`MIN`/`MAX` constants are added to `StepIdx` and `SlotIdx`:**
   - `StepIdx::MIN = Self(0)`, `StepIdx::MAX = Self(u16::MAX)`.
   - `SlotIdx::MIN = Self(0)`, `SlotIdx::MAX = Self(u16::MAX)`.

5. **`checked_add()` methods are added to index types:**
   - `StepIdx::checked_add(self, rhs: u16) -> Option<StepIdx>` — returns `None` on overflow.
   - `SlotIdx::checked_add(self, rhs: u16) -> Option<SlotIdx>` — returns `None` on overflow.
   - `ConstIdx::checked_add(self, rhs: u16) -> Option<ConstIdx>` — returns `None` on overflow.

6. **`FromStr` is implemented for all numeric ID types:**
   - `WorkflowId`, `StepIdx`, `SlotIdx`, `ExprIdx`, `ActionId`, `AccessorIdx`, `ConstIdx`, `RunId`, `SeqNo`.
   - Parses decimal string to inner numeric value.
   - Returns `Err` if string is not valid decimal or overflows target type.

7. **`RunId` inner type changes from `u128` to `u64`:**
   - `RunId::new(value: u64) -> Self`.
   - `RunId::as_u64(self) -> u64` replaces `as_u128()`.

### Invariants
- All ID wrappers are `#[repr(transparent)]`.
- All ID types derive `Serialize, Deserialize` (postcard-compatible).
- No `unsafe` code in the file.
- No `Arc<Mutex>` anywhere in `vb-core/src/`.

### Done Means
- `cargo build -p vb_core` succeeds with zero warnings.
- `SeqNo`, `StepIdx::ZERO`, `SlotIdx::ZERO`, `RunId::ZERO` are accessible as public constants.
- `StepIdx::checked_add`, `SlotIdx::checked_add`, `ConstIdx::checked_add` are callable and overflow-safe.
- `str::parse::<SeqNo>("123")` produces `Ok(SeqNo(123))`.
- `str::parse::<RunId>("0")` produces `Ok(RunId(0))`.

---

## Deliverable 2: `errors.rs` (rename from `error.rs`) — Extend/Rename

### Preconditions
- `error.rs` exists at `crates/vb-core/src/error.rs`.

### Postconditions

1. **File is renamed to `errors.rs`** and `mod error` is updated to `mod errors` in `lib.rs`.

2. **`CoreError` enum replaces `EngineError`:**
   - All existing `EngineError` variants are preserved and renamed to `CoreError`.
   - Additional variants added:
     - `MissingNextStep { step: StepIdx }`
     - `ExprOutOfBounds { expr: ExprIdx }`
     - `TypeMismatch { expected: &'static str, found: &'static str }`
     - `DivisionByZero`
     - `NonFiniteNumber`
     - `QueueFull`
     - `ResourceLimitExceeded { resource: &'static str }`
     - `AllocationFailed`
   - Each variant implements `std::error::Error` via `#[derive(thiserror::Error)]`.

3. **`CoreResult<T>` type alias is added:**
   ```rust
   pub type CoreResult<T> = Result<T, CoreError>;
   ```

4. **Stable diagnostic codes are assigned:**

   | Range | Category | Variants |
   |-------|----------|----------|
   | E0101–E0109 | Validation — structural | InvalidProgramCounter, MissingNextStep |
   | E0111–E0119 | Validation — bounds | SlotOutOfBounds, ExprOutOfBounds, ConstOutOfBounds |
   | E0201–E0209 | Type errors | TypeMismatch, NonFiniteNumber, DivisionByZero |
   | E0301–E0309 | Execution errors | StepBudgetExhausted, StepCounterOverflow, EmptyStepBudget |
   | E0401–E0409 | Resource/I/O errors | QueueFull, ResourceLimitExceeded, AllocationFailed |

   Each `CoreError` variant carries an optional `Span` and optional `SlotValue` payload for diagnostics.

5. **`From<EngineError> for CoreError` is implemented:**
   - Every `EngineError` variant maps to the equivalent `CoreError` variant.
   - Payload fields (`step`, `slot`, `constant`) are preserved.

6. **`EngineError` type alias is retained for backward compatibility:**
   ```rust
   pub type EngineError = CoreError;
   ```

### Invariants
- `CoreError` is `#[derive(Debug, Error, Clone, PartialEq, Eq)]`.
- No `unsafe` code in the file.
- All variants are documented with `#[error(...)]` Display strings.
- The diagnostic code for each variant is a constant associated item.

### Done Means
- `cargo build -p vb_core` succeeds.
- `CoreError::InvalidProgramCounter` has an associated `const DIAGNOSTIC_CODE: u16 = 0x0101`.
- `CoreResult::<()>::Ok(())` type-checks.
- `let err: EngineError = CoreError::SlotOutOfBounds { slot: SlotIdx::ZERO };` compiles (type alias works).
- `CoreError::from(EngineError::StepBudgetExhausted)` produces equivalent variant.

---

## Deliverable 3: `limits.rs` — New File

### Preconditions
- File does not exist at `crates/vb-core/src/limits.rs`.

### Postconditions

```rust
#![forbid(unsafe_code)]

pub const MAX_STEPS_PER_WORKFLOW: usize = 65_535;
pub const MAX_SLOTS_PER_STEP: usize = 256;
pub const MAX_CONSTANTS: usize = 65_535;
pub const MAX_EXPRESSION_DEPTH: usize = 64;
pub const MAX_RUN_NAME_LENGTH: usize = 1_024;
```

### Invariants
- All constants are `usize`.
- Values are compile-time constants (no `const fn` needed).
- File is exported from `lib.rs` as `pub mod limits`.

### Done Means
- `vb_core::limits::MAX_STEPS_PER_WORKFLOW == 65_535`.
- `vb_core::limits::MAX_SLOTS_PER_STEP == 256`.
- `vb_core::limits::MAX_CONSTANTS == 65_535`.
- `vb_core::limits::MAX_EXPRESSION_DEPTH == 64`.
- `vb_core::limits::MAX_RUN_NAME_LENGTH == 1_024`.

---

## Deliverable 4: `span.rs` — New File

### Preconditions
- File does not exist at `crates/vb-core/src/span.rs`.

### Postconditions

```rust
#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const ZERO: Self = Self { start: 0, end: 0 };
    pub const fn is_empty(self) -> bool { self.start == self.end }
    pub const fn new(start: u32, end: u32) -> Self { Self { start, end } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located<T> {
    pub value: T,
    pub span: Span,
}

pub type Spanned<T> = Located<T>;

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    _private: (),
}
```

### Invariants
- `Span` is `Copy` — `start` and `end` are plain `u32`.
- `Located<T>` is `Clone` regardless of `T: Clone`.
- `SourceMap` is a no-op placeholder (maps are added in later phases).

### Done Means
- `Span::ZERO.is_empty()` returns `true`.
- `Span::new(0, 5).end == 5`.
- `Located { value: 42u32, span: Span::ZERO }.value == 42`.
- `Spanned<u8>` is an alias for `Located<u8>`.

---

## Deliverable 5: `diagnostic.rs` — New File

### Preconditions
- `span.rs` exists and `Span` type is available.
- File does not exist at `crates/vb-core/src/diagnostic.rs`.

### Postconditions

```rust
#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCode(u16);

impl DiagnosticCode {
    pub const fn new(code: u16) -> Self { Self(code) }
    pub const fn code(self) -> u16 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity { Error, Warning, Info }

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: Box<str>,
    pub severity: Severity,
    pub span: Span,
}
```

### Invariants
- `DiagnosticCode` is `Copy`.
- `Severity` has exactly three variants.
- `Diagnostic` owns its `message` (no `&str` borrow).
- File is exported from `lib.rs` as `pub mod diagnostic`.

### Done Means
- `DiagnosticCode::new(0x0101).code() == 0x0101`.
- `Severity::Error`, `Severity::Warning`, `Severity::Info` are constructible.
- `Diagnostic { code: DiagnosticCode::new(0x0101), message: "test".into(), severity: Severity::Error, span: Span::ZERO }` compiles.

---

## Deliverable 6: `value.rs` — Extend Existing

### Preconditions
- `value.rs` exists at `crates/vb-core/src/value.rs` with `Taint` and `SlotValue` types.

### Postconditions

1. **`SlotValue::type_name(&self) -> &'static str` is implemented:**
   - `SlotValue::Null.type_name()` returns `"null"`.
   - `SlotValue::Bool(_).type_name()` returns `"boolean"`.
   - `SlotValue::I64(_).type_name()` returns `"number"`.
   - `SlotValue::Text(_).type_name()` returns `"text"`.
   - `SlotValue::Bytes(_).type_name()` returns `"bytes"`.
   - `SlotValue::Object(_).type_name()` returns `"object"`.
   - `SlotValue::List(_).type_name()` returns `"list"`.

### Invariants
- `type_name()` is `#[must_use]`.
- `type_name()` is `const fn` or marked `const`.

### Done Means
- `SlotValue::Null.type_name() == "null"`.
- `SlotValue::Bool(true).type_name() == "boolean"`.
- `SlotValue::I64(42).type_name() == "number"`.
- `SlotValue::Text("hello".into()).type_name() == "text"`.
- `SlotValue::Bytes(bytes::Bytes::new()).type_name() == "bytes"`.
- `SlotValue::Object(vec![].into_boxed_slice()).type_name() == "object"`.
- `SlotValue::List(vec![].into_boxed_slice()).type_name() == "list"`.

---

## Cross-Cutting Invariants

1. **No `unsafe_code`** in any `vb-core/src/` file.
2. **No `Arc<Mutex>`** or any `Mutex`/`RwLock` in `vb-core/src/`.
3. All new `pub` items are exported from `lib.rs`.
4. All files use `#[forbid(unsafe_code)]`.
5. All types that are `Serialize`/`Deserialize` derive both traits.

---

## Quality Gate

The following must pass before Phase 1 is considered complete:

```
cargo +nightly build -p vb_core 2>&1 | head -50
cargo +nightly clippy -p vb_core --all-targets --all-features -- -D warnings
cargo +nightly test -p vb_core --all-features
cargo +nightly fmt --all
```

Specifically:

- [ ] `vb_core::ids::SeqNo`, `SeqNo::ZERO`, `SeqNo::checked_add` are accessible.
- [ ] `vb_core::ids::StepIdx::ZERO`, `StepIdx::checked_add` are accessible.
- [ ] `vb_core::ids::SlotIdx::ZERO`, `SlotIdx::checked_add` are accessible.
- [ ] `vb_core::ids::RunId::ZERO` exists and `RunId` is `u64`-based.
- [ ] `vb_core::errors::CoreError`, `vb_core::errors::CoreResult<T>` are accessible.
- [ ] `vb_core::errors::EngineError` is a type alias for `CoreError`.
- [ ] `vb_core::limits::MAX_STEPS_PER_WORKFLOW == 65_535`.
- [ ] `vb_core::span::Span`, `vb_core::span::Located<T>`, `vb_core::span::Spanned<T>` are accessible.
- [ ] `vb_core::diagnostic::DiagnosticCode`, `vb_core::diagnostic::Severity`, `vb_core::diagnostic::Diagnostic` are accessible.
- [ ] `SlotValue::type_name` returns correct strings for all 7 variants.
- [ ] All `FromStr` implementations parse valid decimal strings.
- [ ] No `unsafe` anywhere in `vb-core/src/`.
- [ ] `cargo test -p vb_core` produces zero failures.
