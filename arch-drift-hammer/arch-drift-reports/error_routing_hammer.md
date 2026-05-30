# Architectural Drift Report: `error_routing.rs`

**File**: `crates/vb_core/src/engine/error_routing.rs`
**Total Lines**: 485
**Line Limit**: 300
**Overflow**: 185 lines (155% of limit)
**Date**: 2026-05-29
**Enforcer**: architectural-drift

---

## Executive Summary

This file is a **CRITICAL architectural drift violation**. It exceeds the 300-line limit by 185 lines and exhibits severe primitive obsession throughout the error domain. The inline test module (320 lines) is itself larger than the entire allowed file budget and MUST be extracted.

---

## Violation #1: File Size (CRITICAL)

| Metric | Value |
|--------|-------|
| Actual Lines | 485 |
| Limit | 300 |
| Overflow | 185 lines |
| % of Limit | 155% |

### Breakdown

| Section | Lines | % of Total |
|---------|-------|------------|
| Module doc + imports | 1-21 | 4.3% |
| Public types (`ErrorHandlerOutcome`, `ErrorSlotData`) | 22-51 | 6.2% |
| `engine_error_static_code` | 54-106 | 10.9% |
| `error_code_string` | 108-113 | 1.2% |
| `route_error_handler` | 115-139 | 5.2% |
| `advance_to_handler` | 141-146 | 1.2% |
| `write_error_slot` | 148-156 | 1.9% |
| `has_error_handler` (test helper) | 158-162 | 1.0% |
| **Inline test module** | **165-485** | **66.1%** |

**The inline test module alone is 320 lines — 107% of the allowed budget.**

---

## Violation #2: Primitive Obsession in Error Domain

### Finding 2.1: `ErrorSlotData` uses raw `Box<str>` for domain values

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSlotData {
    pub code: Box<str>,      // ← PRIMITIVE OBSESSION
    pub message: Box<str>,   // ← PRIMITIVE OBSESSION
    pub failed_step: StepIdx,
}
```

**Problem**: `Box<str>` is a primitive. The domain concepts `code` and `message` should be **Value Objects**.

**Required Refactor**:
```rust
// NEW: crates/vb_core/src/engine/errors/error_code.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorCode(Box<str>);

impl ErrorCode {
    pub fn as_str(&self) -> &str { &self.0 }
}
```

```rust
// NEW: crates/vb_core/src/engine/errors/error_message.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorMessage(Box<str>);

impl ErrorMessage {
    pub fn as_str(&self) -> &str { &self.0 }
}
```

### Finding 2.2: `engine_error_static_code` returns raw `&'static str`

```rust
fn engine_error_static_code(error: &EngineError) -> &'static str {
    // ... returns raw string primitives
}
```

**Problem**: Returns `&'static str` instead of `ErrorCode`. Every call site must cast this to a `Box<str>` anyway (see `error_code_string`).

**Required Refactor**: Return `ErrorCode` directly.

### Finding 2.3: `error_code_string` returns `Box<str>`

```rust
fn error_code_string(error: &EngineError) -> Box<str> {
    error
        .runtime_code()
        .map(|code| code.into())
        .unwrap_or_else(|| engine_error_static_code(error).into())
}
```

**Problem**: Returns a primitive `Box<str>`. This function should return `ErrorCode`.

---

## Violation #3: Inline Test Module Exceeds Entire File Budget

**Lines 165-485 = 320 lines** of test code embedded in a production source file.

### Problems:
1. **Size**: 320 lines > 300 line limit for entire file
2. **Location**: Test code belongs in `crates/vb_core/tests/`, not inline
3. **Test helpers duplicated**: `test_parts_with_error_handler()` and `test_parts_without_error_handler()` create full `WorkflowParts` by hand
4. **No isolation**: Tests cannot run independently without the full module being compiled

### Required Action:
```
crates/vb_core/tests/
  └── error_routing_tests.rs  (extract all tests here)
```

---

## Violation #4: Flat Error Code Enumeration

`engine_error_static_code` (lines 54-106) is a 52-line match expression returning string literals. This is a **classic primitive obsession anti-pattern**: using strings where a typed enum or newtype should live.

**Preferred**: A derive macro on `CoreError` that auto-generates error codes, or an associated constant.

---

## Violation #5: Unused `_error` Parameter in `write_error_slot`

```rust
fn write_error_slot(
    run: &mut RunFrame,
    error_slot: SlotIdx,
    _error: &EngineError,  // ← UNUSED
    failed_step: StepIdx,
) -> Result<(), EngineError> {
    run.write_slot(error_slot, SlotValue::I64(i64::from(failed_step.get())))?;
    Ok(())
}
```

**Problem**: The `_error` parameter is never used. The comment in the module doc says "full diagnostic details (code, message, step) are captured" but only `failed_step` is actually written.

This is a **dead parameter** and a **semantic mismatch** — either use it or remove it.

---

## Structural Recommendations

### Target File Structure (after refactor)

```
crates/vb_core/src/engine/
├── mod.rs              (~20 lines - re-exports)
├── error_routing.rs    (target: ~150 lines)
│   ├── ErrorHandlerOutcome enum
│   ├── ErrorSlotData struct (with typed ErrorCode/ErrorMessage)
│   ├── route_error_handler()
│   └── advance_to_handler()
└── errors/
    ├── mod.rs
    ├── error_code.rs      (NEW - Value Object)
    └── error_message.rs   (NEW - Value Object)
```

### Move to tests/

```
crates/vb_core/tests/
└── error_routing_tests.rs  (320 lines - extracted inline tests)
```

---

## Action Items

| Priority | Item | Owner |
|----------|------|-------|
| P0 | Extract inline test module to `crates/vb_core/tests/error_routing_tests.rs` | bead |
| P0 | Create `ErrorCode` newtype and replace `Box<str>` in `ErrorSlotData` | bead |
| P0 | Create `ErrorMessage` newtype and replace `Box<str>` in `ErrorSlotData` | bead |
| P1 | Refactor `engine_error_static_code` to return `ErrorCode` | bead |
| P1 | Remove or use `_error` parameter in `write_error_slot` | bead |
| P2 | Generate error codes via derive macro (optional future) | bead |

---

## Verdict

**UNACCEPTABLE** — File requires immediate structural intervention before any feature work proceeds.

**Estimated Refactor Cost**: 4 beads (extract tests, create ErrorCode, create ErrorMessage, cleanup dead params)

---

*Report generated by architectural-drift agent*
