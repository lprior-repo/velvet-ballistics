# Bead vb-k8z9l: ABI and SSEI Evidence Artifacts

## Status: Complete for LOCAL5090 authoring; qi37 deferred

### Changes This Turn

**artifact_evidence_support/runtime_cases.rs:**

- Removed all NEW `panic!` branches introduced by prior worker; converted to Result-returning test style
- Replaced weak `assert_ne!(expected, submitted)` with exact `assert_eq` on `RuntimeError::AdmissionActionAbiDigestMismatch` variant including exact `expected` and `submitted` ABI digests
- Added `action_contract_abi_digest()` helper computing blake3 digest over postcard-serialized `ActionContract`, matching runtime's `vb_runtime::recovery::action_contract_abi_digest` computation
- Imported `vb_core::ids::ActionId` and `vb_core::action::ActionContract` to type ABI digest assertions
- Restored `changed_ir_rejected` function (prior worker accidentally orphaned it)
- Preserved all existing assertions, counter invariants, and no-allocation test properties
- Four `runtime_cases` tests now pass (were 0 passing after prior worker introduced panics)

**vb_ssei_verification_admission_acceptance.rs:**

- No source edits required; existing four tests already pass
- Positive certificate case (test_strict_verify_emits_certificate_when_workflow_is_safe) verified with actual test run, including println diagnostics confirming `granted == grants` and `idempotency_attested` production bug (empty, not populated by `prepare_compiled_artifact`)
- All four tests pass: accept-when-gates-pass, strict-verify-certificate, reject-when-capability-missing, reject-when-ir-digest-mismatches

**Contract correction (for record):**
- Bead contract referenced nonexistent `ObligationKind::TypedLedger` variant; actual enum has 10 variants: `Determinism`, `EffectSafety`, `CapabilityNonMinting`, `Boundedness`, `ReplayEffectSeparation`, `ArtifactValidity`, `IdempotencyClassification`, `RecoveryClassification`, `TaintHandling`, `PolicyBinding`
- `ArtifactEvidence` enum has `Verified`, `Unverified`, and `Packed` variants (not `Rejected`)
- No code change needed; test file uses correct variants

### Test Results

```
vb_9agmk_artifact_evidence: 25 passed; 0 failed
vb_ssei_verification_admission_acceptance: 4 passed; 0 failed
Total: 29 passed, 0 failed
```

Both suites pass under `RUSTFLAGS=-Dwarnings` strict scoped check.

### Formatting

Both edited files pass `cargo fmt --check`. No formatting issues remain in either target file.

### Deferred Work

**vb_qi37_4_2_strict_runtime_admission.rs still has 15 failing tests.** Untouched this turn; will be finished by DeepSeek author after independent mirror review of this batch. No attempt made to fix or work around qi37 failures in this scope.

### Artifacts Produced

- Updated runtime_cases.rs with exact ABI digest assertions (no panic branches, no inequality checks)
- Verified SSEI positive certificate case passes with actual run
- This handoff document
- No scaffolds, throwaway scripts, or temporary files left behind
- No commits, pushes, or bead close operations performed (independent review pending)
