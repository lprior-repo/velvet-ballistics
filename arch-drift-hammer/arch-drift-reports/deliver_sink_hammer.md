# Architectural Drift Report: `deliver_sink.rs`

**File**: `crates/vb_cli/src/deliver_sink.rs`
**Line Count**: 387 lines
**Violation**: HARD LIMIT EXCEEDED — 300 line max, 387 found (+87 lines, +29%)

---

## Executive Summary

**VERDICT: GUILTY**

This file violates:
1. `<300 line rule` — 387 lines (87 over limit)
2. **Primitive Obsession** — Multiple `&str`/`PathBuf` used without NewType wrappers
3. **Single Responsibility Principle** — Mixes domain types, path validation, temp file lifecycle, I/O operations, and tests

---

## Responsibility Map

### Domain Types (Lines 14-51)
- `DeliverTarget` — enum with `Stdout` and `NewFile(PathBuf)` variants
- `DeliverSinkError` — error enum wrapping `io::ErrorKind`

### Parsing/Validation (Lines 53-112)
- `parse_deliver_target()` — parses `&str` into `DeliverTarget`
- `parse_file_target()` — extracts path from file scheme
- `validate_new_file_path()` — validates absolute, non-blocked, non-existing paths

### I/O Operations (Lines 118-190)
- `write_json_line()` — entry point for writing
- `write_json_line_to_new_file()` — validates then writes via temp file
- `write_json_line_to_temp_file()` — creates temp, writes, cleans up on failure
- `persist_temp_file()` — atomic-ish via `hard_link`
- `temporary_path()` — generates `.tmp` suffix path
- `create_new_file()` — `OpenOptions::new().create_new(true)`
- `write_json_line_to_writer()` — generic JSON line writer
- `to_io_error()` — IoErrorKind mapper

### Utility (Lines 114-116)
- `is_blocked_root()` — checks `/dev`, `/proc`, `/sys`

### Tests (Lines 196-387)
- 14 test functions, 191 lines — **50% of file is test code**

---

## Primitive Obsession Violations

### VIOLATION 1: Raw `&str` for Scheme Input
```rust
pub(crate) fn parse_deliver_target(raw: &str) -> Result<DeliverTarget, DeliverSinkError>
```
**Problem**: `&str` is unvalidated, unbounded input. The parsing logic must handle empty strings, missing colons, non-UTF-8 edge cases (though `split_once` handles UTF-8).

**Should Be**: NewType wrapper `DeliverTargetRaw<T: AsRef<str>>(T)` or at minimum a documented `RawTargetInput` type alias with a validator.

---

### VIOLATION 2: `PathBuf` in Domain Type
```rust
pub(crate) enum DeliverTarget {
    Stdout,
    NewFile(PathBuf),  // PRIMITIVE OBSESSION
}
```
**Problem**: `PathBuf` is a library type, not a domain concept. `NewFile` implies "new deliverable file" but the `PathBuf` leaks infrastructure.

**Should Be**: A NewType like `DeliverFilePath(PathBuf)` that encapsulates the validation domain logic.

---

### VIOLATION 3: `io::ErrorKind` Wrapped Inline
```rust
pub(crate) enum DeliverSinkError {
    // ...
    Io(io::ErrorKind),  // PRIMITIVE OBSESSION — io::ErrorKind is an infrastructure concern
}
```
**Problem**: `io::ErrorKind` is a standard library primitive. This leaks I/O concerns into the domain error model.

**Should Be**: A domain-specific error variant `Io(IoError)` where `IoError` is a NewType wrapper around `io::Error` or at minimum a bounded enum of domain-relevant I/O errors (e.g., `PermissionDenied`, `DiskFull`, `NotFound`).

---

### VIOLATION 4: `MAX_PATH_BYTES: usize = 4096` Constant
```rust
const MAX_PATH_BYTES: usize = 4096;
```
**Problem**: `usize` is primitive. This should be a typed constant `const MAX_PATH_BYTES: NonZeroUsize = ...`.

---

### VIOLATION 5: Raw `OsString` Manipulation in `temporary_path()`
```rust
fn temporary_path(path: &Path) -> Result<PathBuf, DeliverSinkError> {
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(".tmp");
    Ok(parent.join(temp_name))
}
```
**Problem**: Manual `OsString` manipulation instead of a typed `TempFileName` wrapper.

**Should Be**: A `TempFileName` NewType with a constructor that safely appends `.tmp`.

---

## File Size Violation — Root Cause Analysis

**387 lines total**:
- **Domain + I/O logic**: ~190 lines
- **Tests**: ~191 lines (49.4%)

**Recommendation**: Split into:
1. `deliver_sink.rs` — domain types, parsing, error model (~120 lines)
2. `deliver_sink_io.rs` — I/O operations, temp file lifecycle (~80 lines)
3. `deliver_sink/tests.rs` — tests moved to `tests/` directory (standard Rust layout)

---

## Scott Wlaschin DDD Violations

### 1. `parse_deliver_target` Mixes Parsing with Validation
```rust
pub(crate) fn parse_deliver_target(raw: &str) -> Result<DeliverTarget, DeliverSinkError> {
    if raw == STDOUT_TARGET {
        return Ok(DeliverTarget::Stdout);
    }
    let Some((scheme, value)) = raw.split_once(':') else {
        return Err(DeliverSinkError::MissingScheme);
    };
    match scheme {
        FILE_SCHEME => parse_file_target(value),
        WEBHOOK_SCHEME => Err(DeliverSinkError::UnsupportedWebhook),
        _ => Err(DeliverSinkError::UnknownScheme),
    }
}
```
**Problem**: This is "Validate, don't Parse" — it parses AND validates in one function. Scott Wlaschin advocates: **Parse, don't validate**. First convert to a structured form, THEN validate with a separate function.

**Should Be**:
1. `parse_target_string()` → `Result<TargetRaw, ParseError>` — only syntax parsing
2. `validate_target()` → `Result<DeliverTarget, ValidationError>` — semantic validation

---

### 2. State Machine Implicit in `write_json_line_to_new_file`
```rust
fn write_json_line_to_new_file(path: &Path, value: &Value) -> Result<(), DeliverSinkError> {
    validate_new_file_path(path)?;  // <-- validation happens HERE
    let temp_path = temporary_path(path)?;
    write_json_line_to_temp_file(&temp_path, value)?;
    persist_temp_file(&temp_path, path)  // <-- implicit state: temp→final
}
```
**Problem**: The temp-file-then-atomic-link workflow is implicit. This is an implicit state machine with states: `Validated → TempFileCreated → Persisted`. Not modeled as such.

**Should Be**: Explicit state enum:
```rust
enum DeliverWorkflow {
    Validated(PathBuf),
    TempFileCreated { temp: PathBuf, final_: PathBuf },
    Persisted(PathBuf),
}
```

---

### 3. Error Enum Leaks Infrastructure
`DeliverSinkError::Io(io::ErrorKind)` exposes that we use `io::Error`. If the implementation changes to `tokio::fs` or `async`, the error type changes. The error enum should be domain-centric.

---

## Refactoring Prescription

### Minimum Viable Fix (Line Count Only)

Move tests to `deliver_sink/tests.rs`:
- `tests/` directory is the standard Rust location for integration tests
- This alone saves 191 lines → file becomes **196 lines**

### Full Refactor (DDD Compliance)

| NewFile | Responsibility | Target Lines |
|---------|-----------------|--------------|
| `deliver_target.rs` | `DeliverTarget`, `DeliverSinkError`, parsing, validation | ~120 |
| `deliver_sink_io.rs` | I/O operations, temp file lifecycle | ~80 |
| `deliver_sink/tests.rs` | All 14 test functions | ~191 |

---

## Findings Summary

| # | Violation Type | Severity | Fix Complexity |
|---|----------------|----------|----------------|
| 1 | **File size: 387 > 300** | CRITICAL | MOVE TESTS (trivial) |
| 2 | `PathBuf` in `DeliverTarget::NewFile` | HIGH | NewType wrapper |
| 3 | `io::ErrorKind` in error enum | HIGH | Domain error wrapper |
| 4 | `parse_deliver_target` mixes parse+validate | MEDIUM | Separate parse/validate |
| 5 | Implicit temp→persist state machine | MEDIUM | Explicit state enum |
| 6 | Raw `&str` for scheme input | MEDIUM | NewType wrapper |
| 7 | `MAX_PATH_BYTES: usize` primitive | LOW | `NonZeroUsize` |
| 8 | Manual `OsString` in `temporary_path` | LOW | `TempFileName` NewType |

---

## Verdict

**STATUS: VIOLATION**

**Mandatory Actions**:
1. Move tests to `tests/deliver_sink.rs` (or `deliver_sink/tests.rs`) — reduces to 196 lines
2. Create `DeliverFilePath(PathBuf)` NewType wrapper
3. Create `Io(Error)` NewType instead of `io::ErrorKind` inline
4. Separate `parse_target_string()` from `validate_target()`

**Immediate Halt Until**: File is split and line count verified ≤300.

---

*Architectural Drift Enforcer — NO EXCEPTIONS*
