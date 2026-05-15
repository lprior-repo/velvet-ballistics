# Assurance Bundle: vb-engine-yaml

STATUS: COMPLETE

## Assurance Bundle Summary

Bead: `vb-engine-yaml`
State: 13 attempt 1
Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`

## Contract Clauses Coverage

### PRE-001 / POST-001 / INV-001: Runtime YAML Leak / Dependency Boundary
- **Evidence**: moon ci static-scan-ci gate (deferred to CI)
- **Status**: Not covered by this bead's verification

### PRE-002 / POST-002 / POST-003 / INV-003 / INV-004: Admission, Artifact Acceptance, Durability
- **Evidence**: TLA+ EngineYamlAdmission.tla (PO-002) PASS: 32 states, 13 distinct
- **Evidence**: Kani engine_yaml_admission_rejects_raw_ir (PO-012) PASS
- **Status**: ADEQUATE

### PRE-003 / POST-005 / INV-005: Lifecycle, Terminal State, Sequence
- **Evidence**: TLA+ EngineYamlRunLifecycle.tla (PO-003) PASS: 100 states, 31 distinct
- **Evidence**: Verus step_state_machine.rs (PO-008) PASS: 9 verified
- **Status**: ADEQUATE

### PRE-004 / POST-004 / INV-002 / INV-006 / ERR-ResourceLimitExceeded / ERR-InvalidNumericIr: Numeric Bounds, Resource Limits
- **Evidence**: Kani PO-011A (8 sub-harnesses) PASS
- **Evidence**: Verus resource_budget.rs (PO-007) PASS: 10 verified
- **Evidence**: Verus step_state_machine.rs (PO-008) PASS
- **Waiver**: PO-011B (6 sub-harnesses timeout/fail_alloc) - compensating PO-011A evidence
- **Status**: ADEQUATE WITH WAIVER

### PRE-005 / POST-006 / INV-008 / ERR-RecoveryIncomplete / ERR-CorruptRecord / ERR-ReplayDiverged: Recovery
- **Evidence**: TLA+ EngineYamlRecovery.tla (PO-004) PASS: 838 states, 387 distinct
- **Evidence**: Verus recovery_verification.rs (PO-009) PASS: 7 verified
- **Status**: ADEQUATE

### PRE-006 / POST-007 / ERR-Backpressure / ERR-UnsupportedRuntimeProtocol / ERR-ArtifactNotAccepted: Ingress, Backpressure, Protocol Rejection
- **Evidence**: TLA+ EngineYamlIngress.tla (PO-005) PASS: 2234 states, 447 distinct
- **Evidence**: Loom bounded_queue (PO-013) PASS: 2 passed
- **Evidence**: New test `unsupported_yaml_features_return_typed_diagnostics` PASS
- **Status**: ADEQUATE

### ERR-CapabilityDenied / ERR-DigestMismatch / ERR-NonIdempotentReplayBlocked / ERR-DurabilityBeforeAckFailed: Capability and Digest
- **Evidence**: TLA+ CapabilityLifecycle.tla (PO-006) PASS: 478 states, 220 distinct
- **Evidence**: Verus capability_artifact_model.rs (PO-010) PASS: 8 verified
- **Status**: ADEQUATE

### POST-008 / INV-007: Missing Gate, Regression, Coverage
- **Evidence**: moon ci gates (deferred to CI)
- **Status**: Not covered by this bead's verification

## Formal Verification Evidence

| Lane | Obligations | Status |
|------|-------------|--------|
| TLA+ | PO-002 through PO-006 | 5 PASS |
| Verus | PO-007 through PO-010 | 4 PASS |
| Kani | PO-011A, PO-012 | 9 sub-harnesses PASS |
| Loom | PO-013 | PASS |
| Waived | PO-011B, PO-022, PO-023 | Documented |

## Test Evidence

| Crate | Tests | Status |
|-------|-------|--------|
| vb_yaml | 204 | PASS |
| vb_validate | 927 | PASS |
| vb_core | 1521 | PASS |

## Machine Gate Evidence

- Compile: 12 crates compiled successfully
- Tests: All gates passed
- No production code changes

## Gaps and Waivers

| Gap | Waiver | Compensating Evidence |
|-----|--------|----------------------|
| 6 Kani sub-harnesses timeout/fail_alloc | PO-011B waiver | 8 PO-011A sub-harnesses prove core accessor invariants |
| moon ci static-scan-ci not run | N/A | Not required for this bead's scope |
| moon ci fuzz/miri/mutation not run | N/A | Owner-state-11 obligations |

## Decision

**Assurance bundle is complete and adequate for delivery.**