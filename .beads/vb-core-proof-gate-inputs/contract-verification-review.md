# Contract Verification Review — vb-core-proof-gate-inputs

**Bead**: vb-core-proof-gate-inputs
**Workspace**: /tmp/vb-ws/vb-core-proof-gate-inputs
**Reviewer**: contract-verification-reviewer
**Date**: 2026-05-15

---

## Files Reviewed

| File | Present | Size |
|------|---------|------|
| `contract.md` | ✓ | 138 lines |
| `proof-obligations.planned.jsonl` | ✓ | 16 records |
| `proof-writer-report.md` | ✓ | 199 lines |

### Not Present (Informational — Out of Scope for This Bead)

| File | Note |
|------|------|
| `tla-spec.md` | Not required: admission gate inputs are stateless/checklist items (gate_count, durable), not temporal workflows. |
| `lean-contract.md` | Not required: no theorem kernel beyond Verus in scope. |
| `traceability-matrix.jsonl` | Not produced by this bead; coverage mapping is in proof-obligations.jsonl and proof-writer-report. |

---

## Command Evidence

No `jq` validation run — proof-obligations.planned.jsonl was read directly and confirmed as valid JSONL with 16 records.

---

## Findings

### Severity: LETHAL — Missing Traceability to Contract Clauses

**Clause**: layer_completeness (skill rule)
**Problem**: `proof-obligations.planned.jsonl` does not include `traceability-matrix.jsonl`. While proof-obligations.jsonl maps each obligation to a `contract_clause` field (e.g., POST-001, POST-002, INV-002), the skill requires a separate `traceability-matrix.jsonl` to prove complete clause coverage. Three contract clauses appear in contract.md but are not individually traced in obligations:
- Error taxonomy entries (`ERR-001-ArtifactChecksumMismatch`, `ERR-ArtifactMalformed`, `ERR-InvalidGateCount`) — mapped only via K-G2-001 which is a stub
- Open questions (idempotency_keyed/attested derivation, bounded flag derivation) — covered by waiver
**Required fix**: Produce `traceability-matrix.jsonl` mapping every contract clause ID to its proof obligation(s) and status (verified/waived/planned).

---

### Severity: MAJOR — Proof Flag Derivation Not Verified

**Clause**: POST-001 (bounded, taint_safe, retry_safe, replayable)
**Problem**: POST-001 requires `bounded == true` (derived from `validate_budget`), `taint_safe == true`, `retry_safe == true`, `replayable == true`. The proof-obligations show these as `default to true` but there is no Verus spec proving that `bounded` is derived from `validate_budget` success. V-G1-002 (validate_budget) has a trivially-true proof fn `validate_budget_bounded_flag` that proves nothing. The action-contract-based flag derivation is entirely unwired (waived).
**Required fix**: Either (a) strengthen V-G1-002 to prove bounded flag derivation, or (b) expand the waiver to cover the bounded flag specifically, with compensating evidence.

---

### Severity: MAJOR — Kani Obligations Are Non-Executable

**Clause**: K-G2-001 (ERR-001-ArtifactChecksumMismatch)
**Problem**: The Kani harness `verification/kani/vb_storage_checksum_kani.rs` is a stub with `kani::assume(true)`. ERR-001 (checksum mismatch error) is only verified by this non-functional Kani harness. There is no cargo test that exercises the checksum mismatch path (the existing tests all use correctly-constructed workflows that pass checksum validation).
**Required fix**: Replace Kani stub with a real harness, or add a cargo test that corrupts the serialized bytes to trigger checksum mismatch, or obtain a waiver for K-G2-001.

---

### Severity: MINOR — Waiver Table Misrepresents V-PF-001

**Clause**: WAIVER-FLAG-DERIV
**Problem**: Waiver record lists V-PF-001 as WAIVED. V-PF-001 Verus spec covers ALL of POST-001 including non-waived fields (digest, gate_count, durable). Only flag fields are waived. This creates a misleading record.
**Required fix**: Clarify that V-PF-001 is partially waived: core fields (digest/gate_count/durable) remain verified by the Verus spec and tests.

---

## Coverage Decision

### Contract clauses traced:

| Clause ID | Obligation(s) | Status |
|-----------|---------------|--------|
| POST-001 | V-PF-001 (Verus), PROP-G1-001 (proptest) | Partially verified — flags waived |
| POST-002 | V-POL-001 (Verus spec), TEST-POL-001 (cargo test) | Verified by tests |
| POST-003 | V-POL-001 (Verus spec), TEST-POL-002 (cargo test) | Verified by tests |
| POST-004 | V-POL-001 (Verus spec), TEST-POL-003 (cargo test) | Verified by tests |
| INV-001 | V-PF-001 (gate_count ∈ {0,2}) | Verified by tests |
| INV-002 | V-PF-002 (Verus), TEST-WARN-001 (cargo test) | Verified |
| ERR-001 | K-G2-001 (Kani — stub) | NOT verified |
| bounded (POST-001) | V-G1-002 (trivially true proof) | NOT verified |

### TLA+-owned clauses: None (out of scope — stateless gate inputs)

### Verus-owned clauses: V-PF-001, V-PF-002, V-G1-001, V-G1-002, V-G2-001, V-POL-001
- **Scope valid**: Rust-local pure validation and constructor logic
- **Issue**: Self-referential specs; V-G1-002 trivially true; Err branch weak

### Theorem-owned clauses: None

### Proof obligations traced: 16 obligations in proof-obligations.planned.jsonl
- Verus ×6: SPEC ONLY (no execution evidence)
- Kani ×2: FAIL (stubs)
- cargo test ×5: PASS (existing tests verified)
- Miri ×1: CONDITIONAL (trivial pass — no unsafe)
- proptest ×1: FAIL (todo!() stubs)
- waiver ×1: VALID with minor table error

### Waivers valid: Yes (WAIVER-FLAG-DERIV has reason, expiry, compensation, owner)
- Minor issue: V-PF-001 listed as fully waived when only flag fields are waived

---

## Verdict

**STATUS: REJECTED**

### Blockers

1. **Missing `traceability-matrix.jsonl`** — contract completeness not proven without clause-level traceability
2. **K-G2-001 non-executable** — ERR-001 (checksum mismatch) has no working verification
3. **V-G1-002 trivially true proof** — bounded flag derivation not actually verified
4. **PROP-G1-001 non-executable** — proptest helpers contain `todo!()`

### Required Repairs

1. Produce `traceability-matrix.jsonl` mapping every contract clause to obligation(s)
2. Replace K-G2-001 Kani stub with real harness OR add cargo test for checksum mismatch path OR obtain waiver for K-G2-001
3. Strengthen V-G1-002 `validate_budget_bounded_flag` to actually prove bounded=true OR expand waiver for bounded flag
4. Replace proptest `todo!()` helpers with working implementations OR move property tests to existing proptest infrastructure

### Compensating Evidence (Keep Ship Holding)

- All 5 cargo test obligations are verified by real, existing tests in admission.rs
- V-PF-002 (INV-002) is verified by Verus specs and boundary tests
- BDD tests cover policy-level behavior
- Waiver for flag derivation is valid with compensating evidence (gate_count, durable are primary signals)
- No unsafe code in admission.rs (`#![forbid(unsafe_code)]`)

---

*contract-verification-reviewer: vb-core-proof-gate-inputs rejected at proof-gate-inputs stage — repairs required before test planning*
