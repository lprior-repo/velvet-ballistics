# Proof-to-Implementation Bridge Input — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10
**Phase**: State 4 — Proof Planning (bridge input)
**Date**: 2026-05-24

**Purpose**: Provide `proof-to-implementation` agent with sufficient context to map approved proof claims to Rust source refs, behavior tests, refinement harness refs, and exact evidence commands. This document is NOT proof artifacts — it is bridge input.

---

## 1. Bridge Mapping Summary

| Proof Domain | Proof Claims | Rust Source Targets | Behavior Tests | Refinement Harness Refs |
|-------------|-------------|-------------------|----------------|------------------------|
| SymbolicCode type | PO-001, PO-016 | `crates/vb_core/src/diagnostic.rs` (SymbolicCode, from_static) | `crates/vb_core/tests/proptest_symbolic_code.rs` | `crates/vb_core/kani/kani_symbolic_code_validation.rs` |
| CODE_REGISTRY bijection | PO-002, PO-010, PO-011, PO-023 | `crates/vb_core/src/diagnostic.rs` (CODE_REGISTRY, const assertions) | `crates/vb_core/tests/proptest_registry_consistency.rs` | `crates/vb_core/kani/kani_registry_bijection.rs`, `kani_registry_category.rs` |
| ValidationError::code() | PO-003, PO-017 | `crates/vb_validate/src/lib.rs` (ValidationError enum, code() method) | `crates/vb_validate/tests/proptest_validation_error_codes.rs` | `crates/vb_validate/kani/kani_validation_error_code.rs` |
| is_supported_code() | PO-004, PO-018 | `crates/vb_core/src/diagnostic.rs` (is_supported_code) | `crates/vb_core/tests/proptest_supported_codes.rs` | `crates/vb_core/kani/kani_is_supported_code.rs` |
| Diagnostic constructor | PO-005, PO-014, PO-019 | `crates/vb_core/src/diagnostic.rs` (Diagnostic::new) | `crates/vb_core/tests/proptest_diagnostic_constructor.rs` | `crates/vb_core/kani/kani_diagnostic_constructor.rs` |
| YamlError::code() | PO-006 | `crates/vb_yaml/src/error.rs` (YamlError enum, code() method) | Covered by PO-025 cross-crate test | `crates/vb_yaml/kani/kani_yaml_error_code.rs` |
| Zero-allocation | PO-007 (waiver candidate) | `crates/vb_core/src/diagnostic.rs` (SymbolicCode, DiagnosticCode) | — | `crates/vb_core/kani/kani_zero_alloc.rs` |
| Backward compat | PO-008 | `crates/vb_core/src/diagnostic.rs` (DiagnosticCode::from_str) | `crates/vb_core/tests/proptest_supported_codes.rs` | `crates/vb_core/kani/kani_from_str_compat.rs` |
| Serialization | PO-009, PO-021, PO-022 | `crates/vb_core/src/diagnostic.rs` (Serialize/Deserialize impls) | `crates/vb_core/tests/proptest_serde_roundtrip.rs` | `crates/vb_core/kani/kani_serde_roundtrip.rs`, `fuzz/fuzz_targets/fuzz_symbolic_code_deserialize.rs` (MISSING — not in fuzz_targets/ or Cargo.toml) |
| Reverse lookup | PO-012 | `crates/vb_core/src/diagnostic.rs` (DiagnosticCode::symbolic_code) | Covered by PO-023 | `crates/vb_core/kani/kani_reverse_lookup.rs` |
| Section 16 parity | PO-024 | `crates/vb_core/src/diagnostic.rs` (CODE_REGISTRY, public constants) | `crates/vb_core/tests/proptest_section16_parity.rs` | — |
| HasSymbolicCode determinism | PO-013 | `crates/vb_core/src/diagnostic.rs` (HasSymbolicCode trait, implementations) | — | `crates/vb_core/kani/kani_determinism.rs` |
| CompileError codes | PO-020 | `crates/vb_compile/src/mod_compile_errors/collection.rs` (CompileError::code) | `crates/workspace_tests/tests/proptest_compile_error_codes.rs` | — |
| Error type registration | PO-015, PO-025 | `crates/vb_core/src/errors.rs`, `crates/vb_runtime/src/error/diagnostics.rs`, `crates/vb_storage/src/error/codes.rs` | `crates/workspace_tests/tests/proptest_error_types_registration.rs` | `crates/workspace_tests/kani/kani_error_types_code.rs` |
| diag_codes.rs promotion | PO-026 | `crates/vb_validate/src/diag_codes.rs` (promoted constants) | `crates/vb_validate/tests/proptest_diag_codes_promotion.rs` | — |
| Mutation resistance | PO-027 | All diagnostic modules across crates | — | — |
| CI Gauntlet | PO-028 | — | — | `moon-rust-verification.yml` |

---

## 2. Cross-Crate Impacts

### 2.1 vb_core (diagnostic types)

**Source files**:
- `crates/vb_core/src/diagnostic.rs` — SymbolicCode, DiagnosticCode, Diagnostic, CODE_REGISTRY, is_supported_code, HasSymbolicCode, Serialize/Deserialize
- `crates/vb_core/src/errors.rs` — CoreError::symbolic_code()
- `crates/vb_core/src/lib.rs` — public re-exports

**Proof obligations mapping to vb_core**: PO-001, PO-002, PO-004, PO-005, PO-007–PO-016, PO-018, PO-019, PO-021, PO-023, PO-024

### 2.2 vb_validate (validation errors)

**Source files**:
- `crates/vb_validate/src/lib.rs` — ValidationError enum, new code() method
- `crates/vb_validate/src/diagnostic.rs` — error-to-diagnostic bridge (updated for SymbolicCode)
- `crates/vb_validate/src/diag_codes.rs` — constants promotion from #[cfg(test)]
- `crates/vb_validate/src/schema.rs`, `control_flow.rs`, `references.rs`, `type_taint.rs`, `gates.rs` — error variant emission

**Proof obligations mapping to vb_validate**: PO-003, PO-017, PO-026

### 2.3 vb_compile (compilation errors)

**Source files**:
- `crates/vb_compile/src/mod_compile_errors/collection.rs` — CompileError::code() return type change &'static str → SymbolicCode
- `crates/vb_compile/src/mod_compile_errors/kind.rs` — CompileError enum

**Proof obligations mapping to vb_compile**: PO-020

### 2.4 vb_yaml (YAML errors)

**Source files**:
- `crates/vb_yaml/src/error.rs` — YamlError enum, new code() method

**Proof obligations mapping to vb_yaml**: PO-006

### 2.5 vb_runtime (runtime errors)

**Source files**:
- `crates/vb_runtime/src/error/mod.rs` — RuntimeError enum
- `crates/vb_runtime/src/error/diagnostics.rs` — new symbolic_code() method

**Proof obligations mapping to vb_runtime**: PO-015, PO-025

### 2.6 vb_storage (storage errors)

**Source files**:
- `crates/vb_storage/src/error/mod.rs` — JournalError enum
- `crates/vb_storage/src/error/codes.rs` — new symbolic_code() method

**Proof obligations mapping to vb_storage**: PO-015, PO-025

---

## 3. Behavior Test Obligations

These are the *independent behavior tests* that proof-to-implementation should plan mapping lines for:

| Test | Crate | Target Proofs | What It Validates |
|------|-------|--------------|-------------------|
| `proptest_symbolic_code` | vb_core | PO-001, PO-016 | from_static only accepts registered strings |
| `proptest_registry_consistency` | vb_core | PO-023 | Registry uniqueness, non-zero, category matching |
| `proptest_validation_error_codes` | vb_validate | PO-017 | 58 variants → 58 unique SymbolicCodes |
| `proptest_supported_codes` | vb_core | PO-018 | is_supported_code() + from_str correctness |
| `proptest_diagnostic_constructor` | vb_core | PO-019 | Diagnostic constructor consistency |
| `proptest_compile_error_codes` | workspace_tests | PO-020 | All CompileError codes registered |
| `proptest_serde_roundtrip` | vb_core | PO-021 | Serialization round-trip |
| `proptest_section16_parity` | vb_core | PO-024 | 36 codes match master doc |
| `proptest_error_types_registration` | workspace_tests | PO-025 | CoreError/RuntimeError/JournalError codes registered |
| `proptest_diag_codes_promotion` | vb_validate | PO-026 | 58 constants match CODE_REGISTRY |

---

## 4. Evidence Command Registry

Proof-to-implementation should map each of these exact commands to its bridge lines:

```bash
# Kani harnesses
cargo kani --harness kani_from_static_validation --crate vb_core
cargo kani --harness kani_registry_bijection --crate vb_core
cargo kani --harness kani_validation_error_code_registered --crate vb_validate
cargo kani --harness kani_is_supported_code_all_constants --crate vb_core
cargo kani --harness kani_diagnostic_constructor_consistency --crate vb_core
cargo kani --harness kani_yaml_error_code_registered --crate vb_yaml
cargo kani --harness kani_zero_alloc_hot_path --crate vb_core --enable-stubbing
cargo kani --harness kani_from_str_backward_compat --crate vb_core
cargo kani --harness kani_serde_roundtrip --crate vb_core
cargo kani --harness kani_registry_nonzero --crate vb_core
cargo kani --harness kani_registry_category_match --crate vb_core
cargo kani --harness kani_reverse_lookup --crate vb_core
cargo kani --harness kani_symbolic_code_determinism --crate vb_core
cargo kani --harness kani_diagnostic_no_mismatch --crate vb_core
cargo kani --harness kani_error_types_symbolic_code --crate workspace_tests

# Proptest suites
cargo test --test proptest_symbolic_code -- --nocapture
cargo test --test proptest_validation_error_codes -- --nocapture
cargo test --test proptest_supported_codes -- --nocapture
cargo test --test proptest_diagnostic_constructor -- --nocapture
cargo test --test proptest_compile_error_codes -- --nocapture
cargo test --test proptest_serde_roundtrip -- --nocapture
cargo test --test proptest_registry_consistency -- --nocapture
cargo test --test proptest_section16_parity -- --nocapture
cargo test --test proptest_error_types_registration -- --nocapture
cargo test --test proptest_diag_codes_promotion -- --nocapture

# Fuzz (BLOCKED — target MISSING)
# cargo fuzz run fuzz_symbolic_code_deserialize -- -max_len=4096 -runs=100000
# NOTE: fuzz/fuzz_targets/fuzz_symbolic_code_deserialize.rs does NOT exist. Not in fuzz_targets/ or fuzz/Cargo.toml.

# Mutation
cargo mutants --in-package vb_core --in-package vb_validate --in-package vb_compile --in-package vb_yaml -- --test-dir tests/

# CI Gauntlet
moon run :rust-verification-gauntlet
```

---

## 5. Source-to-Proof Traceability

| Rust Source Location | Proof Obligation IDs | What Is Proved |
|---------------------|---------------------|----------------|
| `SymbolicCode` (struct definition) | PO-001, PO-010, PO-016 | Zero-allocation Copy type, valid static lifetime |
| `SymbolicCode::from_static()` | PO-001, PO-013, PO-016 | Only accepts registered strings; deterministic |
| `CODE_REGISTRY` (const data) | PO-002, PO-010, PO-011, PO-023 | Bijection, non-zero, category matching |
| `DiagnosticCode::from_str()` | PO-008, PO-018 | Backward compat + new code parsing |
| `DiagnosticCode::symbolic_code()` | PO-012, PO-023 | Reverse lookup correctness |
| `Diagnostic::new()` | PO-005, PO-014, PO-019 | Constructor invariant: no mismatched codes |
| `is_supported_code()` | PO-004, PO-018 | Accepts all in-use code constants |
| `ValidationError::code()` | PO-003, PO-017 | Every variant → registered SymbolicCode, unique |
| `YamlError::code()` | PO-006 | Every variant → registered SymbolicCode |
| `CompileError::code()` | PO-020 | Every variant → registered SymbolicCode |
| `CoreError::symbolic_code()` | PO-015, PO-025 | Every variant → registered SymbolicCode |
| `RuntimeError::symbolic_code()` | PO-015, PO-025 | Every variant → registered SymbolicCode |
| `JournalError::symbolic_code()` | PO-015, PO-025 | Every variant → registered SymbolicCode |
| `HasSymbolicCode` trait | PO-013 | All impls pure, total, deterministic |
| `SymbolicCode` Serialize/Deserialize | PO-009, PO-021, PO-022 | Round-trip; reject unknown; hostile input safe |
| `diag_codes.rs` constants | PO-026 | Synchronized with CODE_REGISTRY |

---

## 6. Acceptance Criteria Cross-Reference

| AC | Contract Clause | Proof Obligations | Implementation Target |
|----|----------------|-------------------|----------------------|
| AC-1 | C-VE-1, C-VE-2 | PO-003, PO-017 | `vb_validate/src/lib.rs` ValidationError::code() |
| AC-2 | C-CE-1 | PO-020 | `vb_compile/src/mod_compile_errors/collection.rs` CompileError::code() |
| AC-3 | C-YE-1 | PO-006 | `vb_yaml/src/error.rs` YamlError::code() |
| AC-4 | C-REG-2, C-REG-3 | PO-002, PO-023, PO-024 | `vb_core/src/diagnostic.rs` CODE_REGISTRY |
| AC-5 | C-DC-2 | PO-004, PO-018 | `vb_core/src/diagnostic.rs` is_supported_code() |
| AC-6 | C-SYM-2 | PO-001, PO-016 | `vb_core/src/diagnostic.rs` SymbolicCode::from_static() |
| AC-7 | C-CE-2 | PO-020 | `vb_compile/src/mod_compile_errors/collection.rs` |
| AC-8 | C-VE-3 | PO-024 | `vb_core/src/diagnostic.rs` + proptest_section16_parity.rs |
| AC-9 | C-DC-2 | PO-008, PO-018 | `vb_core/src/diagnostic.rs` from_str |
| AC-10 | C-BC-1 | PO-008 | `vb_core/src/diagnostic.rs` from_str backward compat |
| AC-11 | C-REG-3 | PO-002, PO-023 | `vb_core/src/diagnostic.rs` CODE_REGISTRY |
| AC-12 | C-DIAG-2, C-DIAG-3 | PO-005, PO-014, PO-019 | `vb_core/src/diagnostic.rs` Diagnostic::new() |

---

## 7. Bridge Readiness Checklist

- [x] All 20 proof seeds mapped to proof obligations
- [x] All obligation IDs referenceable from bridge mapping lines
- [x] All Rust source locations identified (crate, file, target symbol)
- [x] All behavior test files identified
- [x] All refinement harness files identified
- [x] Exact evidence commands documented
- [x] Cross-crate impacts enumerated
- [x] Acceptance criteria ↔ proof obligation traceability
- [ ] Awaiting proof-plan-reviewer approval before bridge finalization
- [ ] Awaiting proof-writer materialization of harness files before bridge mapping
