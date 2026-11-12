# Contract — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 3 — Rust Contract  
**Contract Version**: 1.0.0

---

## 1. Purpose

This contract defines the type-level, behavioral, and cross-crate contracts for exposing Section 16 symbolic diagnostic codes as the stable public API. It extends the existing numeric `DiagnosticCode` infrastructure with a symbolic-first contract while preserving backward compatibility.

---

## 2. Core Types

### 2.1 `SymbolicCode`

**Contract**: A `Copy`, zero-allocation, `repr(transparent)` newtype over `&'static str`. Every `SymbolicCode` value corresponds to exactly one registered diagnostic code. Construction is gated by the canonical code registry.

| Clause | Contract |
|--------|----------|
| **C-SYM-1: Encoding** | `SymbolicCode` is `#[repr(transparent)]` over `&'static str`. |
| **C-SYM-2: Validity** | `SymbolicCode::from_static(s)` returns `Some(code)` iff `s` is in `CODE_REGISTRY`. All other strings return `None`. |
| **C-SYM-3: Bijection** | For every code in the registry, `SymbolicCode::from_static(symbolic).unwrap().numeric_code()` equals the registry's numeric value. |
| **C-SYM-4: Display** | `Display` formats as the symbolic name (e.g., `"DUPLICATE_KEY"`), not the E-hex form. |
| **C-SYM-5: Serialization** | `Serialize` outputs the symbolic name as a string. `Deserialize` validates against the registry and rejects unknown names. |
| **C-SYM-6: Zero-allocation** | Constructing, copying, displaying, and serializing a `SymbolicCode` does not allocate on the heap. |
| **C-SYM-7: Send + Sync** | `SymbolicCode` is `Send` and `Sync`. |

### 2.2 `DiagnosticCode` (Evolved)

**Contract**: Retained internal numeric encoding. Extended with symbolic code lookup capability. The packed `u16` format is unchanged.

| Clause | Contract |
|--------|----------|
| **C-DC-1: Layout** | `DiagnosticCode` remains `#[repr(transparent)]` over `u16`. No breaking ABI change. |
| **C-DC-2: Parsing** | `FromStr` accepts `"E"` + 4 hex digits. `is_supported_code()` must return `true` for the parsed value. Updated to include E05xx, E06xx, and codes above 0x401B. |
| **C-DC-3: Symbolic lookup** | `DiagnosticCode::symbolic_code(self) -> Option<SymbolicCode>` returns the symbolic code if the numeric value is in the registry. |
| **C-DC-4: Display** | `Display` formats as `"E0101"` (existing behavior, unchanged). |
| **C-DC-5: Backward-compatible** | All existing `DiagnosticCode::new(code)` callsites continue to compile and function correctly. |

### 2.3 `Diagnostic` (Evolved)

**Contract**: User-facing diagnostic record. Symbolic code is the primary identifier.

| Clause | Contract |
|--------|----------|
| **C-DIAG-1: Symbolic primary** | `Diagnostic.code` is of type `SymbolicCode`. |
| **C-DIAG-2: Numeric derived** | `Diagnostic.numeric_code` is of type `DiagnosticCode`, derived from `self.code`. Invariant: `self.numeric_code.symbolic_code() == Some(self.code)`. |
| **C-DIAG-3: Constructor** | `Diagnostic::new(code: SymbolicCode, message: Box<str>, severity: Severity, span: Span)` derives `numeric_code` from `code`. Never panics. |
| **C-DIAG-4: Severity** | Every `ValidationError`-derived diagnostic has `Severity::Error`. |

### 2.4 `CodeRegistry`

**Contract**: The single source of truth for all known diagnostic codes. Defined in `vb_core`. Immutable at runtime.

| Clause | Contract |
|--------|----------|
| **C-REG-1: Location** | The registry is defined in `vb_core::diagnostic` and exported as a public `const`. |
| **C-REG-2: Completeness** | Every numeric code constant defined in `vb_validate/src/diagnostic.rs`, `vb_storage/src/error/codes.rs`, and all other error-code locations has a corresponding entry. |
| **C-REG-3: Uniqueness** | No duplicate symbolic names. No duplicate numeric codes. |
| **C-REG-4: Non-zero** | All registered numeric codes are non-zero. |
| **C-REG-5: Category consistency** | For each entry, `(numeric >> 8) & 0xFF` matches the expected high byte for `category`. |
| **C-REG-6: Append-only** | Once a code is in the registry and released, it cannot be removed. Deprecation is done via a `deprecated: bool` flag or separate deprecated list. |

---

## 3. Error Type Contracts

### 3.1 `ValidationError`

| Clause | Contract |
|--------|----------|
| **C-VE-1: code()** | `ValidationError` has a public `code() -> SymbolicCode` method. Every variant maps to exactly one `SymbolicCode`. |
| **C-VE-2: Exhaustive** | `code()` uses an exhaustive `match` without wildcard arms. Adding a variant forces a compile error until `code()` is updated. |
| **C-VE-3: Section 16 parity** | All 36 Section 16 codes map correctly. The symbolic name matches the master contract exactly (e.g., `"DUPLICATE_KEY"` not `"DUPLICATE_KEY "`). |
| **C-VE-4: Gate verifier codes** | The 19 gate verifier variants (E05xx) are included in `code()`. |
| **C-VE-5: Contract discovery codes** | The 3 contract discovery variants (E06xx) are included in `code()`. |
| **C-VE-6: Unique codes** | Every `ValidationError` variant produces a unique `SymbolicCode`. No two variants share the same code. |
| **C-VE-7: diagnostic_from_error()** | Updated to produce `Diagnostic` with `SymbolicCode` as primary. Backward compatible: existing consumers that called `diagnostic.code.code()` must migrate. |

### 3.2 `CompileError`

| Clause | Contract |
|--------|----------|
| **C-CE-1: code()** | `CompileError::code()` return type updated from `&'static str` to `SymbolicCode`. |
| **C-CE-2: Symbolic parity** | All symbolic string codes currently returned by `code()` (including compilation-specific ones like `"INVALID_EXPRESSION"`) are registered in the registry. |
| **C-CE-3: diagnostic_code()** | `diagnostic_code()` continues to alias `code()`. |

### 3.3 `YamlError`

| Clause | Contract |
|--------|----------|
| **C-YE-1: code()** | `YamlError` gains a public `code() -> SymbolicCode` method. |
| **C-YE-2: Mapping** | Each of the 20 `YamlError` variants maps to a Section 16 `SymbolicCode` (see error-taxonomy §2.3). |
| **C-YE-3: Exhaustive** | `code()` uses exhaustive match without wildcard. |

### 3.4 `CoreError`, `RuntimeError`, `JournalError`

| Clause | Contract |
|--------|----------|
| **C-OTH-1: symbolic_code()** | Each error type gains a `symbolic_code() -> SymbolicCode` method (or equivalent). |
| **C-OTH-2: Existing methods preserved** | `diagnostic_code() -> DiagnosticCode` methods are retained. |
| **C-OTH-3: Section 17 codes** | `RuntimeError` Section 17 codes map to `SymbolicCode` values. |
| **C-OTH-4: Storage codes** | `JournalError` 28 codes map to `SymbolicCode` values. |

---

## 4. Trait: `HasSymbolicCode`

| Clause | Contract |
|--------|----------|
| **C-TRAIT-1: Definition** | `pub trait HasSymbolicCode { fn symbolic_code(&self) -> SymbolicCode; }` defined in `vb_core`. |
| **C-TRAIT-2: Implementors** | `ValidationError`, `CompileError`, `YamlError`, `CoreError`, `RuntimeError`, `JournalError`. |
| **C-TRAIT-3: Purity** | All implementations are pure functions: no I/O, no allocation, no side effects. |

---

## 5. Backward Compatibility

| Clause | Contract |
|--------|----------|
| **C-BC-1: DiagnosticCode::from_str** | `DiagnosticCode::from_str("E0101")` continues to return `Ok(DiagnosticCode(0x0101))`. |
| **C-BC-2: New codes parsable** | `DiagnosticCode::from_str("E0501")` now returns `Ok(...)` instead of `Err(UnsupportedCode)`. |
| **C-BC-3: Numeric API preserved** | `DiagnosticCode::new(code)`, `.code() -> u16`, `Display` as `"E0101"` all unchanged. |
| **C-BC-4: Diagnostic struct migration** | Consumers accessing `diagnostic.code.code()` (numeric) must migrate. This is a documented breaking change. |

---

## 6. Forbidden States (Made Unrepresentable)

| Clause | Forbidden State |
|--------|----------------|
| **C-FS-1** | A `SymbolicCode` containing a string not in the registry. |
| **C-FS-2** | A `DiagnosticCode` parseable from string but without a corresponding symbolic entry. |
| **C-FS-3** | An error variant without a `code()` entry. |
| **C-FS-4** | A duplicate symbolic code in the registry. |
| **C-FS-5** | A duplicate numeric code in the registry. |
| **C-FS-6** | A `Diagnostic` record with mismatched symbolic and numeric codes. |

---

## 7. Acceptance Criteria

| # | Criterion | Source |
|---|-----------|--------|
| AC-1 | `ValidationError` has `code() -> SymbolicCode` method covering all 58 variants. | REQ-1 |
| AC-2 | `CompileError::code()` returns `SymbolicCode` instead of bare `&'static str`. | REQ-1 |
| AC-3 | `YamlError` has `code() -> SymbolicCode` method covering all 20 variants. | GAP-3 |
| AC-4 | `vb_core::CODE_REGISTRY` contains all 90+ known codes with bijective symbolic↔numeric mapping. | GAP-5 |
| AC-5 | `is_supported_code()` accepts E05xx and E06xx ranges and codes above 0x401B. | GAP-4 |
| AC-6 | `SymbolicCode::from_static("DUPLICATE_KEY")` returns `Some`; `from_static("BOGUS")` returns `None`. | Domain model |
| AC-7 | `CompileError` symbolic code behavior test passes with updated `SymbolicCode` assertions. | REQ-3 |
| AC-8 | `ValidationError` diagnostic tests pass with all 58 variants producing valid `SymbolicCode`. | REQ-3 |
| AC-9 | `DiagnosticCode::from_str("E0501")` succeeds (previously failed). | GAP-4 |
| AC-10 | `DiagnosticCode::from_str("E0101")` still succeeds (backward compat). | C-BC-1 |
| AC-11 | No existing numeric code constants are duplicated across crates after migration. | GAP-5 |
| AC-12 | `Diagnostic.code` is `SymbolicCode`. Diagnostic record always has consistent symbolic and numeric codes. | C-DIAG-3 |

---

## 8. Contract Versioning

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-05-24 | Initial contract for Section 16 symbolic diagnostic codes. Defines `SymbolicCode`, `CodeRegistry`, error type contracts, backward compatibility requirements. |
