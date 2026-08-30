# Proof Repair Guide — R3 (from Review R2)

**Reviewer Invocation**: prv-vb-xi2f10-20260525T140000Z
**Bead**: vb-xi2f.10
**Status**: REJECTED (R2)
**Date**: 2026-05-25

---

## 1. Summary of R2 vs R1

R1 was REJECTED because zero proof artifacts existed on disk. R2 shows good progress: all 27+ artifacts now exist on disk, 8/10 proptest files compile. However, R2 identifies 2 new CRITICAL findings not present in R1 because the files can now be reviewed for content quality.

### R1 → R2 Progress
- ✅ File existence: 0 files → 27+ files
- ✅ Proptest compilation: 0 compile → 8/10 compile
- ✅ STATE.md: State 1 → State 5

### R2 Blocking Issues (New)
- ❌ CRITICAL: Kani proofs are model-level, not connected to production
- ❌ CRITICAL: Zero execution evidence
- ❌ HIGH: Kani scaffolding missing
- ❌ HIGH: Section 16 parity incomplete
- ❌ HIGH: Gate/contract discovery tests always pass

---

## 2. CRITICAL Repairs

### Repair R3-001: Reconcile Kani Proofs with Production Types

**Resolves**: F-R2-001 (CRITICAL, all 15 Kani obligations)

The Kani harnesses prove properties about an inline model (SymbolicCode, CODE_REGISTRY, etc.) that does not exist in production. Two paths forward:

**Path A (Recommended)**: Rewrite Kani harnesses to prove properties of actual production types:
- PO-001: Prove `DiagnosticCode::from_str` returns Ok for all supported ranges, Err for others. Production already has `is_supported_code()` and `FromStr` impl.
- PO-002: Prove no duplicate codes across the supported ranges in `is_supported_code()`. Test uniqueness of the ranges themselves.
- PO-003: Prove `vb_validate::diagnostic::error_code()` returns DiagnosticCode in supported range for all ValidationError variants.
- PO-004: Prove all code constants in `vb_validate::diagnostic` are in `is_supported_code()` ranges.
- PO-005: Prove `Diagnostic::new(code, msg, sev, span)` preserves all fields.
- PO-006: Prove YamlError variants map to supported DiagnosticCode values.
- PO-008: Prove `DiagnosticCode::from_str` parses all previously supported codes + new codes.
- PO-009: Prove `DiagnosticCode` serde roundtrip (Serialize from Display, Deserialize from E-format).
- And so on.

**Path B**: Keep the inline models but provide a bridge warrant explaining exactly how each model construct maps to a production construct, with a commitment to complete the proof-to-implementation bridge at State 8.

**Path A is strongly preferred** because the production types already exist and are simple enough for Kani to handle.

### Repair R3-002: Execute Verifiers and Capture Evidence

**Resolves**: F-R2-002 (CRITICAL, all 28 obligations)

Minimum viable execution:
```bash
# Run the 8 compilable proptest tests
cargo test -p vb_core --test proptest_symbolic_code -- --nocapture 2>&1 | tee evidence/proptest-symbolic-code.log
cargo test -p vb_core --test proptest_supported_codes -- --nocapture 2>&1 | tee evidence/proptest-supported-codes.log
cargo test -p vb_core --test proptest_diagnostic_constructor -- --nocapture 2>&1 | tee evidence/proptest-diag-constructor.log
cargo test -p vb_core --test proptest_serde_roundtrip -- --nocapture 2>&1 | tee evidence/proptest-serde-roundtrip.log
cargo test -p vb_core --test proptest_registry_consistency -- --nocapture 2>&1 | tee evidence/proptest-registry-consistency.log
cargo test -p vb_core --test proptest_section16_parity -- --nocapture 2>&1 | tee evidence/proptest-section16-parity.log
cargo test -p vb_validate --test proptest_validation_error_codes -- --nocapture 2>&1 | tee evidence/proptest-validation-error-codes.log
cargo test -p vb_validate --test proptest_diag_codes_promotion -- --nocapture 2>&1 | tee evidence/proptest-diag-codes-promotion.log
```

Capture the raw output in `.evidence/` and reference from `proof-evidence.md`.

---

## 3. HIGH Repairs

### Repair R3-003: Add Kani Scaffolding

**Resolves**: F-R2-003 (HIGH, PO-001 through PO-014)

Add to each crate's `lib.rs`:
```rust
// In crates/vb_core/src/lib.rs (alongside existing kani_* declarations)
#[cfg(kani)]
pub mod kani;

// In crates/vb_validate/src/lib.rs
#[cfg(kani)]
pub mod kani;

// In crates/vb_yaml/src/lib.rs
#[cfg(kani)]
pub mod kani;
```

### Repair R3-004: Complete Section 16 Parity Test

**Resolves**: F-R2-004 (HIGH, PO-024)

Extend `SECTION16_CODES` in `proptest_section16_parity.rs` to include:
1. Gate verifier codes (E05xx): E0501 through E0513 (19 codes)
2. Contract discovery codes (E06xx): E0601 through E0603 (3 codes)
3. Fix `INVALID_LOOP` → `INVALID_WAIT` for code 0x0401
4. Add name verification: for each code, verify `vb_validate::diagnostic::error_code()` produces a DiagnosticCode matching the expected numeric code

### Repair R3-005: Strengthen Gate/Contract Discovery Tests

**Resolves**: F-R2-005 (HIGH, PO-026)

Option A (preferred): Add `#[test] fn gate_range_all_parseable()` and `#[test] fn contract_discovery_range_all_parseable()` with actual assertions. Update `is_supported_code()` in `vb_core::diagnostic.rs` to include E05xx and E06xx ranges.

Option B (if upstream blocker): Use explicit `assert!(!failures.is_empty(), ...)` that fails when the test becomes stale, or use conditional compilation with explicit blocker tracking comments.

---

## 4. MEDIUM Repairs

### Repair R3-006: Resolve workspace_tests Exclusion

**Resolves**: F-R2-006 (MEDIUM, PO-020, PO-025)

Option A: Re-add `crates/workspace_tests` to workspace members (requires resolving its dependency on deferred crates).
Option B: Move `proptest_compile_error_codes.rs` and `proptest_error_types_registration.rs` to another crate's test directory that has access to the required types.

### Repair R3-007: Strengthen Proptest Assertions

**Resolves**: F-R2-007 (MEDIUM, PO-016, PO-021, PO-023)

- PO-016: Rename file from `proptest_symbolic_code` to `proptest_diagnostic_code_parse` or extend to test actual symbolic→numeric mapping if such API exists.
- PO-021: Add `serde_json` dev-dependency to vb_core, add actual `serde_json::to_string`/`serde_json::from_str` roundtrip tests.
- PO-023: Rename from `proptest_registry_consistency` to `proptest_from_str_determinism` or extend to test the equivalent registry properties through public API.

### Repair R3-008: Address Missing Fuzz Target PO-022

**Resolves**: F-R2-008 (MEDIUM, PO-022)

**Finding**: `fuzz/fuzz_targets/fuzz_symbolic_code_deserialize.rs` does NOT exist. Not in `fuzz_targets/` directory and not in `fuzz/Cargo.toml` `[[bin]]` entries. This is a ledger inconsistency — multiple ledgers reference a non-existent target.

**Remediation options**:
1. Create the fuzz target file and `[[bin]]` entry in `fuzz/Cargo.toml` if hostile JSON testing is still required
2. Waive PO-022 and rely on compensating evidence from PO-021 (`proptest_serde_roundtrip`), which already covers JSON round-trip identity and unknown-code rejection via proptest
3. Update all ledgers to reflect the MISSING status (current repair)

**Current status**: Ledgers updated to reflect MISSING status. Compensating evidence: PO-021 proptest provides JSON round-trip and unknown-code rejection coverage.

---

## 5. Process Repairs

### Repair R3-009: Add Proof-Writer/Repair Invocation Row

Add an `agent-invocation/v1` row to `agent-invocation-ledger.jsonl` for the REPAIR-3 agent, including invocation_id, skill=proof-writer, state=5, input/output artifact hashes.

### Repair R3-010: Update proof-evidence.md

Replace `PENDING_FORMAL_EXECUTION` with actual execution results. For obligations that remain unexecutable, classify as `BLOCKED` with explicit blocker ID references.

---

## 6. Repair Priority Order

1. **R3-002** (Execute proptests — establishes baseline evidence)
2. **R3-003** (Kani scaffolding — unblocks Kani harness compilation)
3. **R3-001** (Kani production reconciliation — unblocks Kani meaningfulness)
4. **R3-004** (Section 16 parity — coverage completion)
5. **R3-005** (Gate/contract tests — assertion strength)
6. **R3-006** (workspace_tests — unblocks remaining 2 proptest files)
7. **R3-007** (Proptest assertion strength — naming and coverage)
8. **R3-008** (Fuzz dependencies)
9. **R3-009** (Ledger provenance)
10. **R3-010** (Evidence documentation)
