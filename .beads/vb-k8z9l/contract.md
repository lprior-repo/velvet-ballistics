# vb-k8z9l Artifact Fixture Tests - Contract and Oracle Map

## Scope

Fix compilation and runtime failures in 5 existing test files only.
No production code changes. No test file additions or deletions.

## Files

1. `crates/vb_storage/tests/accepted_artifact_red_phase.rs`
2. `crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs`
3. `crates/workspace_tests/tests/vb_lp2v_admission_integration.rs`
4. `crates/workspace_tests/tests/vb_lp2v_admission_proptests.rs`
5. `crates/workspace_tests/tests/vb_lp2v_proof_admission_bdd.rs`

## Known Compile Errors (vb_core_yaml_e2e_chain_contract.rs)

1. `ObligationClosure` imported from `vb_core` root; actual location: `vb_core::artifact`
2. `ActionName("http.fetch".into())` tuple struct constructor not visible; use `ActionName::from_static_infallible("http.fetch")`

## Evidence Requirements

For accepted-artifact verification, the test must assert:
- `verify_accepted_artifact(artifact)` returns `Ok(CompiledWorkflow)` (positive typed proof)
- `verification.evidence` is `ArtifactEvidence::Verified(Box<ArtifactVerificationEvidence>)` (not Unverified/Packed)
- `ArtifactVerificationEvidence.ledger` has 10 entries (`VerificationLedgerEntry`)
- At least one entry has `obligation` field set to an actual `ObligationKind` variant (e.g., `Determinism`, `Boundedness`, `ArtifactValidity`)
- Binding predicates: `verification.digest == artifact.digest`
- For tampered digests: `verify_accepted_artifact` returns `Err(JournalError::ArtifactChecksumMismatch)`

## Oracle Map (actual APIs)

| Assertion | Actual API |
|-----------|------------|
| Producer creates valid artifact | `prepare_compiled_artifact(workflow, policy, contracts)` |
| Runtime verifies artifact | `verify_accepted_artifact(artifact)` |
| Typed evidence structure | `artifact.verification.evidence` -> `ArtifactEvidence::Verified` -> `ArtifactVerificationEvidence` |
| Typed ledger | `evidence.ledger: [VerificationLedgerEntry; 10]` |
| Obligation kind | `VerificationLedgerEntry::obligation` -> `ObligationKind` enum (Determinism, EffectSafety, CapabilityNonMinting, Boundedness, ReplayEffectSeparation, ArtifactValidity, IdempotencyClassification, RecoveryClassification, TaintHandling, PolicyBinding) |
| Binding | `artifact.digest == artifact.verification.digest` |

## Verification Strategy

- Build positive cases with `prepare_compiled_artifact` then targeted mutation
- No weak generic `is_err` replacing exact typed diagnostic
- Relaxed MUST reject tampered digest
- No-action workflow requires 0 caps not 1
- Do not turn desired successful strict admission into expected failure

## Author Errata (vb-k8z9l)

Original contract contained incorrect enum variant name `ObligationKind::TypedLedger` and field name `VerificationLedgerEntry::kind`. Both are now corrected to match the actual production code in `crates/vb_core/src/artifact.rs`:
- `ObligationKind` is a 10-variant enum: Determinism, EffectSafety, CapabilityNonMinting, Boundedness, ReplayEffectSeparation, ArtifactValidity, IdempotencyClassification, RecoveryClassification, TaintHandling, PolicyBinding
- `VerificationLedgerEntry` has field `obligation` (not `kind`) of type `ObligationKind`
- Field `closure` (type `ObligationClosure`) and `policy_digest` (type `WorkflowDigest`) are also present

No production source changes required for this correction; the contract document now accurately reflects the existing production API.
