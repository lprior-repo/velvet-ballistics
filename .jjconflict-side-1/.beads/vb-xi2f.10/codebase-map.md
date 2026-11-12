# Codebase Map — vb-xi2f.10: Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 2 — Explore  
**Date**: 2026-05-24  
**Source checkout**: /home/lewis/src/velvet-ballistics

---

## 1. Executive Summary

The codebase has **three parallel diagnostic representation systems** for validation errors:

| System | Type | Public API | Section 16 Parity |
|--------|------|-----------|-------------------|
| `CompileError::code()` | `&'static str` (symbolic) | ✅ Yes | ✅ Full (strings like `"DUPLICATE_KEY"`) |
| `ValidationError` (thiserror display) | `String` via `#[error("DUPLICATE_KEY")]` | ❌ No `code()` method | ⚠️ Display-only, not typed |
| `DiagnosticCode` (vb_core) | Packed `u16` (E-style numeric) | ✅ Yes (stable type) | ❌ Numeric only, no symbolic mapping |

**Primary Gap**: The stable public diagnostic type `vb_core::DiagnosticCode` is purely numeric (E0101-style), while Section 16 requires **symbolic** codes as the public contract. There is **no unified symbolic diagnostic code type** across the crate boundary.

---

## 2. Section 16 Requirements (verbatim from master doc)

From `velvet-ballistics-MASTER.md` line 669, Section 16 lists **36 symbolic validation codes**:

```text
DUPLICATE_KEY, FORBIDDEN_YAML_FEATURE, UNKNOWN_TOP_LEVEL_FIELD, UNKNOWN_STEP_FIELD,
MISSING_REQUIRED_FIELD, INVALID_VERSION, INVALID_ID, RESERVED_ID, DUPLICATE_ID,
MULTIPLE_STEP_PRIMITIVES, MISSING_STEP_PRIMITIVE, UNKNOWN_REFERENCE, FUTURE_REFERENCE,
SECRET_NOT_DECLARED, DIRECT_RUNTIME_REFERENCE, INVALID_THEN_TARGET, CONTROL_FLOW_CYCLE,
UNREACHABLE_STEP, INVALID_CHOOSE, INVALID_FOR_EACH, INVALID_TOGETHER, INVALID_COLLECT,
INVALID_REDUCE, INVALID_REPEAT, INVALID_WAIT, INVALID_ASK, INVALID_FINISH, INVALID_RETRY,
INVALID_ON_ERROR, SECRET_RESULT_LEAK, TYPE_MISMATCH, PAYLOAD_TOO_LARGE, LIMIT_REQUIRED,
LIMIT_EXCEEDED, UNSUPPORTED_TRIGGER, HTTP_TRIGGER_OUT_OF_CORE
```

Master doc line 563: *"All errors must be typed (no stringly errors), must carry diagnostic codes (Section 16), and must never require heap allocation in the hot path."*

---

## 3. Key Files Map

### 3.1 Core Diagnostic Infrastructure (vb_core)

| File | Purpose | Gap Notes |
|------|---------|-----------|
| `crates/vb_core/src/diagnostic.rs` | `DiagnosticCode(u16)` — packed E-style numeric code. `Diagnostic` struct with code+message+severity+span. `DiagnosticCodeParseError`. `is_supported_code()` validates ranges. | **GAP: No symbolic string field.** Only numeric. Ranges supported: E01xx, E02xx, E03xx, E04xx, E10xx, E11xx, E12xx, E13xx, E14xx, E20xx, E30xx, E40xx. **Missing: E05xx (gate verifier), E06xx (contract discovery), new codes above 0x401B.** |
| `crates/vb_core/src/lib.rs` (line 89) | Re-exports `Diagnostic`, `DiagnosticCode`, `DiagnosticCodeParseError`, `Severity` | Public stable API |
| `crates/vb_core/src/errors.rs` | `CoreError` — 40+ variants with `diagnostic_code() -> DiagnosticCode` (numeric) and `runtime_code() -> Option<&'static str>` (symbolic Section 17 codes) | Has symbolic constants like `QUEUE_FULL_RUNTIME_CODE`, `INPUT_TYPE_MISMATCH_RUNTIME_CODE` — **only for runtime boundary, not validation** |

### 3.2 Validation Error Types (vb_validate)

| File | Purpose | Gap Notes |
|------|---------|-----------|
| `crates/vb_validate/src/lib.rs` | `ValidationError` enum — 50+ variants. Uses `#[error("DUPLICATE_KEY")]` for display. All 36 Section 16 codes present as variants. Plus 20+ gate-verifier and contract-discovery variants. | **GAP: No `code()` method** returning `&'static str`. Symbolic codes are only in thiserror display strings, not extractable as typed values. |
| `crates/vb_validate/src/diagnostic.rs` | `diagnostic_from_error()` and `error_code()` — maps `ValidationError` → `Diagnostic` / `DiagnosticCode` (numeric E-style). Internal mapping table. Has tests covering all variants. | Maps symbolic→numeric but the public API is numeric only |
| `crates/vb_validate/src/diag_codes.rs` | All numeric code constants (E01xx–E06xx) — test-only (`#[cfg(test)]`). 58 constants total. | **GAP: Test-only.** These should be public stable constants or the basis for a symbolic code type. |
| `crates/vb_validate/src/schema.rs` | Schema validation — emits `ValidationError` variants | Covered by Section 16 codes |
| `crates/vb_validate/src/control_flow.rs` | Control-flow validation | Covered by Section 16 codes |
| `crates/vb_validate/src/references.rs` | Reference validation | Covered by Section 16 codes |
| `crates/vb_validate/src/type_taint.rs` | Type/taint validation | Covered by Section 16 codes |
| `crates/vb_validate/src/gates.rs` | Gate verifier (Gates 7-15) | Gate-specific variants (E05xx) not in Section 16 |

### 3.3 Compilation Error Types (vb_compile)

| File | Purpose | Gap Notes |
|------|---------|-----------|
| `crates/vb_compile/src/mod_compile_errors/kind.rs` | `CompileError` enum — 60+ variants for YAML parse/validation/compilation errors | — |
| `crates/vb_compile/src/mod_compile_errors/collection.rs` | `CompileError::code()` → `&'static str` — **the only public API that returns Section 16 symbolic codes**. `CompileErrors::diagnostic_codes()` iterates codes. | This already meets the bead requirement for compilation errors. Symbolic codes like `"DUPLICATE_KEY"`, `"FORBIDDEN_YAML_FEATURE"` etc. |

### 3.4 YAML Error Types (vb_yaml)

| File | Purpose | Gap Notes |
|------|---------|-----------|
| `crates/vb_yaml/src/error.rs` | `YamlError` enum — 20 variants. Uses thiserror display strings (not Section 16 codes). | **GAP: No `code()` method.** No diagnostic code mapping. Errors are stringly. |

### 3.5 Runtime Error Types (vb_runtime)

| File | Purpose | Gap Notes |
|------|---------|-----------|
| `crates/vb_runtime/src/error/mod.rs` | `RuntimeError` enum — 25+ variants | — |
| `crates/vb_runtime/src/error/diagnostics.rs` | `RuntimeError::diagnostic_code() -> DiagnosticCode` (numeric), `runtime_code() -> Option<&'static str>` (symbolic Section 17 codes) | Section 17 codes, not Section 16 validation |

### 3.6 Storage Error Types (vb_storage)

| File | Purpose | Gap Notes |
|------|---------|-----------|
| `crates/vb_storage/src/error/mod.rs` | `JournalError` enum | — |
| `crates/vb_storage/src/error/codes.rs` | `JournalError::diagnostic_code() -> DiagnosticCode` (numeric constants). 28 code constants. | Numeric E-style only |

### 3.7 CLI Integration

| File | Purpose | Notes |
|------|---------|-------|
| `crates/vb_cli/src/app_impl.rs` | `explain_error()` — human-readable formatting for each `CompileError` variant. Exhaustive match on all variants. | Displays user-friendly messages, not symbolic codes |
| `crates/vb_cli/src/app_impl.rs` | `explain_compile_repair_hint()` — repair hints for each variant | — |

### 3.8 Existing Tests and Verification

| File | Purpose |
|------|---------|
| `crates/workspace_tests/tests/vb_test_compile_error_quality_behavior.rs` | 834-line behavior test. Tests that every `CompileError::code()` variant returns the correct Section 16 symbolic string. |
| `crates/vb_validate/src/diagnostic.rs` (tests) | `all_variants_produce_valid_diagnostic()`, `all_variants_have_unique_diagnostic_codes()` — covers all 50+ ValidationError variants |
| `crates/vb_validate/src/diag_codes.rs` (tests) | Asserts 58 diagnostic codes are unique, non-zero, and in correct ranges |
| `crates/vb_validate/src/schema.rs` (tests) | Extensive schema validation tests covering all Section 16 error paths |
| `crates/vb_storage/src/error_tests.rs` | Tests `JournalError::diagnostic_code()` for correct numeric codes |
| `crates/vb_core/src/errors.rs` (tests) | Tests `CoreError::diagnostic_code()` and `runtime_code()` |
| `crates/vb_runtime/src/error/tests_diagnostics.rs` | Tests `RuntimeError::diagnostic_code()` and `runtime_code()` |
| `crates/workspace_tests/tests/error_variant_completeness_test.rs` | Cross-crate error variant exhaustive-match audit |
| `docs/error-variant-completeness.md` | Documented public diagnostic error coverage policy |
| `crates/vb_validate/src/kani_gate_08_accessor.rs` | Kani proof harnesses for gate verifier |
| `crates/vb_validate/src/kani_gate_08_structural.rs` | Kani structural harnesses for gate 8 |

---

## 4. Gap Analysis: Symbolic Code Infrastructure

### Gap 1: Missing Unified Symbolic Diagnostic Code Type

**Problem**: `vb_core::DiagnosticCode` is the only stable, public, cross-crate diagnostic code type. It is purely numeric (`u16`). There is no symbolic equivalent.

**Impact**: Consumers of the diagnostic API only see `E0101`, not `DUPLICATE_KEY`. The bead requires symbolic codes as the stable contract.

**Files affected**:
- `crates/vb_core/src/diagnostic.rs` — needs symbolic code support or replacement
- `crates/vb_core/src/lib.rs` — public API surface

### Gap 2: ValidationError Has No `code()` Method

**Problem**: `ValidationError` uses `#[error("DUPLICATE_KEY")]` in thiserror derive for display but has no method to extract the symbolic code as a typed value.

**Impact**: Downstream code cannot programmatically get `"DUPLICATE_KEY"` from a `ValidationError` without parsing the display string.

**Files affected**:
- `crates/vb_validate/src/lib.rs` — needs `code()` method returning symbolic string
- `crates/vb_validate/src/diagnostic.rs` — currently maps to numeric only

### Gap 3: YamlError Has No Diagnostic Code Support

**Problem**: `YamlError` has no `code()` method and no diagnostic code mapping. Errors are stringly.

**Impact**: YAML parsing errors cannot participate in the symbolic code system.

**Files affected**:
- `crates/vb_yaml/src/error.rs`

### Gap 4: Numeric Code Range Incompleteness

**Problem**: `is_supported_code()` in `diagnostic.rs` does NOT include E05xx (gate verifier codes) or E06xx (contract-discovery codes). New codes above 0x401B are also unsupported.

**Impact**: `DiagnosticCode::from_str("E0501")` returns `Err(UnsupportedCode)` — gate verifier diagnostics cannot round-trip through the public `DiagnosticCode` type.

**Files affected**:
- `crates/vb_core/src/diagnostic.rs` — `is_supported_code()` function

### Gap 5: No Cross-Crate Symbolic Code Registry

**Problem**: Each error type defines its own diagnostic code mapping independently:
- `CompileError` → symbolic `code()` method (string match)
- `ValidationError` → numeric mapping in `diagnostic.rs` (internal)
- `CoreError` → numeric constants in `errors.rs` + `runtime_code()` (symbolic for Section 17)
- `JournalError` → numeric constants in `codes.rs`
- `RuntimeError` → numeric constants in `diagnostics.rs` + `runtime_code()`

There is no single registry, enum, or type that enumerates all Section 16 symbolic codes.

**Impact**: Adding a new error code requires changes in multiple files. Consistency between symbolic and numeric codes is not enforced at compile time.

---

## 5. Dependencies Between Crates

```
vb_yaml ──→ (no diagnostic dep)
vb_validate ──→ vb_core (DiagnosticCode, Diagnostic, Severity, Span)
vb_compile ──→ vb_validate (ValidationError), vb_core (WorkflowError)
vb_core ──→ (self-contained diagnostic types)
vb_runtime ──→ vb_core (DiagnosticCode)
vb_storage ──→ vb_core (DiagnosticCode)
vb_cli ──→ vb_compile (CompileError, CompileErrors)
```

Public diagnostic API flow:
```
vb_core::DiagnosticCode (numeric, stable) ← vb_validate::diagnostic.rs (conversion)
vb_compile::CompileError::code() → &'static str (symbolic, bypasses DiagnosticCode)
vb_compile::CompileError::diagnostic_code() → alias for code()
```

---

## 6. Risk Tags

| Tag | Scope | Detail |
|-----|-------|--------|
| **public API** | `vb_core::DiagnosticCode` | Changing from numeric to symbolic would break the public stable API |
| **migration** | All diagnostic code constants | ~100 numeric code constants across 4 crates would need symbolic equivalents |
| **parser/codec** | `is_supported_code()` | Numeric code validation function must be updated for new ranges |
| **user-visible behavior** | CLI error output | Symbolic codes would change user-facing error format |
| **contract** | Section 16 + Section 17 | Two sections define 36 + 33 = 69 required codes; must stay in sync |

---

## 7. Open Questions

1. Should `DiagnosticCode` be replaced entirely with a symbolic type, or should symbolic codes be added alongside numeric ones?
2. Should `ValidationError` get a `code() -> &'static str` method (matching `CompileError` pattern), or should symbolic codes go through the `DiagnosticCode` type?
3. How should the E05xx gate-verifier codes be incorporated — as Section 16 extensions or a separate section?
4. Should `YamlError` also get symbolic diagnostic codes, or is it acceptable to leave YAML errors as display-only?

---

## 8. Recommended Downstream Owners

| Artifact | Recommended Owner |
|----------|-------------------|
| Contract/type design | `rust-contract` skill |
| Proof planning | `proof-planner` skill (Kani harnesses for code mapping) |
| Implementation | `holzman-rust` skill |
| Test planning | `test-planner` skill (full Section 16 matrix) |
| Review | `black-hat-reviewer` skill |

---

## 9. Evidence Provenance

All facts backed by file reads at specific lines. Key evidence sources:
- `velvet-ballistics-MASTER.md` §16 (lines 669–710)
- `crates/vb_core/src/diagnostic.rs` (lines 1–292)
- `crates/vb_validate/src/lib.rs` (lines 100–362)
- `crates/vb_validate/src/diagnostic.rs` (lines 1–859)
- `crates/vb_validate/src/diag_codes.rs` (lines 1–336)
- `crates/vb_compile/src/mod_compile_errors/kind.rs` (lines 1–167)
- `crates/vb_compile/src/mod_compile_errors/collection.rs` (lines 1–286)
- `crates/vb_runtime/src/error/diagnostics.rs` (lines 1–150)
- `crates/vb_storage/src/error/codes.rs` (lines 1–121)
- `crates/vb_yaml/src/error.rs` (lines 1–79)
- `crates/vb_core/src/errors.rs` (lines 500–716)
- `crates/workspace_tests/tests/vb_test_compile_error_quality_behavior.rs` (lines 1–834)
