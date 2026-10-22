# Bead Contract: vb-k8z9l

**Migrate obsolete artifact-claim and capability fixtures to the accepted evidence contract.**

## Producer/Admission Contract

### Storage Producer (vb_storage::admission::prepare_compiled_artifact)
- Creates typed ArtifactEvidence with schema_version=3
- VerificationProof fields are display-only:
  - gate_count: always 0 (not verification authority)
  - durable: always false (determined by submit, not producer)
  - bounded_claimed: true
  - taint_safe_claimed: false
  - retry_safe_claimed: false
  - replayable_claimed: false
  - idempotency_verified_claimed: true
- Actual verification authority: typed ledger in ArtifactEvidence
- Ledger has 10 entries (not 15); mix of RuntimeValidated and ExternalAssumption closures

### Runtime Admission (vb_runtime::admission::admit_artifact_run)
- Relaxed: no store lookup, returns admission immediately
- Strict/Journaled: load artifact, validate envelope, call verify_stored_artifact
- verify_stored_artifact calls vb_storage::admission::verify_accepted_artifact
- All gate/flag validation happens in storage verification, NOT runtime admission
- Runtime admission only maps storage errors to AdmissionError variants
- Capability check: strict equality (cardinality + membership)

## Before → After Oracle Mapping

| Test File | Old Assertion | New Assertion |
|-----------|---------------|---------------|
| accepted_artifact_red_phase | gate_count==15, all flags true, durable true/false | gate_count==0, bounded+idempotency true, others false, durable always false |
| vb_core_yaml_e2e_chain_contract | gate_count==15, all flags true | gate_count==0, typed ledger assertions |
| vb_qi37_4_2_strict_runtime_admission | fake artifacts with gate_count 0/2/14/16 | real prepared artifacts; type-based rejection tests |
| vb_lp2v_admission_integration | gate_count=7 rejects with ArtifactInvalidGateCount | ArtifactInvalidProofFlag (flag="admission_policy") |
| vb_lp2v_admission_proptests | Relaxed accepts tampered digest | All policies reject tampered digest |
| vb_lp2v_proof_admission_bdd | grants 1 cap to 0-cap workflow | grants 0 caps (strict equality) |
| vb_9agmk_artifact_evidence* | legacy gate count assertions | typed ActionAbiDigestMismatch assertions |
| vb_ssei_verification_admission_acceptance | legacy assertions | inspect actual error |

## Verification Strategy
- Use real prepare_compiled_artifact for positive fixtures
- Mutate specific fields for targeted negative tests
- Never assert on display-only flags as verification authority
- Keep all negative/rejection test strength