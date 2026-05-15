# Proof Plan Review Input — vb-core-proof-gate-inputs

## Review Trigger
State 4 (Proof Planning) completion. Proof-writer should begin obligation execution.

---

## Scope Summary

**Primary artifacts**:
- `crates/vb_storage/src/admission.rs` — VerificationProof, VerificationWarning, submit_artifact_with_contracts, gate 2 checksum
- `crates/vb_core/src/compiled_workflow.rs` — CompiledWorkflow::try_from_parts, gate 1 structure
- `crates/vb_core/src/validation.rs` — validate_budget, BoundednessPolicy

**Key contracts**:
- POST-001: VerificationProof::new field defaults
- POST-002: Relaxed → gate_count=0, durable=false
- POST-003: Journaled → gate_count=2, durable=false
- POST-004: Strict → gate_count=2, durable=true
- INV-001/INV-002: VerificationProof well-formedness + VerificationWarning::is_valid gate range

---

## Obligation Summary (16 rows)

| ID | Clause | Verifier | Required | Status |
|----|--------|----------|----------|--------|
| V-PF-001 | POST-001 | Verus | yes | planned |
| V-PF-002 | INV-002 | Verus | yes | planned |
| V-G1-001 | POST-002/003/004 | Verus | yes | planned |
| V-G1-002 | bounded | Verus | yes | planned |
| V-G2-001 | POST-002/003/004 | Verus | yes | planned |
| V-POL-001 | POST-002/003/004 | Verus | yes | planned |
| K-G2-001 | ERR-ArtifactChecksumMismatch | Kani | yes | planned |
| K-G1-001 | POST-002/003/004 | Kani | no | planned |
| TEST-POL-001 | POST-002 | cargo test | yes | planned |
| TEST-POL-002 | POST-003 | cargo test | yes | planned |
| TEST-POL-003 | POST-004 | cargo test | yes | planned |
| TEST-WARN-001 | INV-002 | cargo test | yes | planned |
| TEST-BDD-001 | POST-002/003/004 | cargo test | yes | planned |
| MIRI-001 | POST-001/004 | Miri | no | planned |
| PROP-G1-001 | POST-002/003/004 | proptest | no | planned |
| WAIVER-FLAG-DERIV | bounded/taint_safe/retry_safe/replayable | waiver | no | waived |

---

## Gate Derivation Review Checklist

- [ ] Gate 1: CompiledWorkflow::try_from_parts — 9 pure sub-checks + validate_budget
- [ ] Gate 2: blake3 hash of postcard-serialized parts (digest zeroed) = workflow.digest()
- [ ] Policy dispatch: Relaxed (gate_count=0), Journaled (gate_count=2, durable=false), Strict (gate_count=2, durable=true)
- [ ] VerificationProof::new defaults: all 6 flags=true, idempotency lists empty, warnings empty
- [ ] VerificationWarning::is_valid: gate ∈ [1, 2]

---

## Flag Derivation Status

| Flag | Source | Status |
|------|--------|--------|
| bounded | validate_budget success → BoundednessPolicy::DEFAULT.validate | Default true; V-G1-002 proves |
| taint_safe | ActionContract taint propagation (future) | WAIVER-FLAG-DERIV |
| retry_safe | ActionContract retry_safety (future) | WAIVER-FLAG-DERIV |
| replayable | ActionContract idempotency replay rules (future) | WAIVER-FLAG-DERIV |
| idempotency_keyed | Actions with Idempotency≠DeterministicPure (future) | WAIVER-FLAG-DERIV |
| idempotency_attested | Actions with explicit idempotency key (future) | WAIVER-FLAG-DERIV |

---

## Proof Coverage Gaps

1. **Flag derivation**: 6 proof flags are not yet derived from ActionContract. Waiver covers this; no proof gap for current defaults.
2. **Checksum blake3+postcard trust boundary**: These externals are assumed correct. Kani stubs them in K-G2-001 harness.
3. **validate_budget boundedness**: V-G1-002 proves policy validation implies boundedness; does not prove full budget arithmetic soundness.

---

## Anti-Hallucination Attestation

- No verifier availability assumed beyond what is in toolchain/proof-obligations.jsonl
- No pass/fail results claimed; all obligations are `planned`
- Waiver WAIVER-FLAG-DERIV has owner, reason, expiry, and compensating evidence
- Commands match actual crate names (vb_storage, vb_core) and test file names
- `admission.rs` confirmed `#![forbid(unsafe_code)]`
