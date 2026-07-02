# Test Plan: vb-engine-yaml

## Overview

Bead: `vb-engine-yaml`
State: 7 test-planning
Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`
Test plan based on: `contract.md`, `traceability-matrix.jsonl`, `proof-obligations.planned.jsonl`, `proof-review.md`, existing test artifacts.

## Contract Clause Coverage

### PRE-002 / POST-002 / INV-003 / INV-007: Admission, Artifact Acceptance, Capability Gate

**Existing tests:**
- `crates/vb_yaml/src/profile_tests.rs`: `single_document_accepted`, `empty_source_rejected`, `multiple_documents_rejected`, `anchor_rejected`, `duplicate_keys_rejected`, `flow_collection_rejected`
- `crates/vb_yaml/src/profile_tests_adversarial.rs`: adversarial YAML inputs
- `crates/vb_validate/tests/capability_contract_schema.rs`: capability schema validation
- `crates/velvet_ballistics/tests/ir_artifact_admission.rs`: IR artifact admission tests

**Gaps:** None identified. Coverage adequate for admission gate behavior.

### PRE-003 / POST-001 / POST-006 / INV-008 / ERR-CorruptRecord: Durability, Recovery, Corrupt Records

**Existing tests:**
- `crates/vb_storage/tests/recovery_integration.rs`: recovery integration
- `crates/vb_storage/tests/replay_resume.rs`: replay/resume
- `crates/vb_runtime/tests/durable_resume_red_phase.rs`: durable resume
- `crates/vb_runtime/tests/durable_retry_red_phase.rs`: durable retry
- `crates/vb_runtime/tests/durability_matrix_integration.rs`: durability matrix

**Gaps:** None identified. Recovery and durability covered by existing integration tests.

### PRE-004 / INV-002 / INV-006 / ERR-ResourceLimitExceeded / ERR-InvalidNumericIr: Numeric Bounds, Resource Limits

**Existing tests:**
- `crates/vb_yaml/src/profile_tests.rs`: `depth_limit_rejected`, `map_size_limit_rejected`, `total_length_limit_rejected`
- `crates/vb_core/tests/proptest_core_types.rs`: property-based tests for core types
- `crates/vb_core/tests/aggregate_resource_budget_red.rs`: resource budget tests
- `crates/vb_validate/tests/gate_08_accessor_parity.rs`: accessor parity tests

**Gaps:** None identified. Numeric bounds and resource limits covered.

### PRE-005 / POST-005 / INV-005: Lifecycle, Terminal State, Sequence

**Existing tests:**
- `crates/vb_runtime/tests/vb_jggy_lifecycle_tests.rs`: lifecycle tests
- `crates/vb_core/src/engine/tests/integration_workflow.rs`: workflow integration

**Gaps:** Terminal state absorption sequence coverage - may need augmentation if not fully covered by existing tests.

### PRE-006 / POST-007 / ERR-Backpressure / ERR-UnsupportedRuntimeProtocol / ERR-ArtifactNotAccepted: Ingress, Backpressure, Protocol Rejection

**Existing tests:**
- `crates/vb_yaml/src/profile_tests_adversarial.rs`: adversarial YAML including unsupported constructs
- `crates/velvet_ballistics/tests/admission_evidence_integration.rs`: admission evidence integration
- IPC/backpressure scenarios: not covered by unit tests; covered by TLA+ model (PO-005) and Loom (PO-013)

**Gaps:** IPC backpressure scenarios are covered by formal verification (TLA+ PO-005, Loom PO-013). Unit test gaps for direct IPC submit rejection are not critical given formal proof coverage.

### ERR-DigestMismatch / ERR-CapabilityDenied: Digest and Capability

**Existing tests:**
- `crates/vb_validate/tests/idempotency_contract_red.rs`: capability and idempotency tests
- `crates/vb_core/tests/aggregate_resource_budget_kani_red.rs`: Kani-verified budget tests

**Gaps:** None identified.

## Test Gaps Requiring New Tests

### Gap 1: Typed Operator Diagnostic Coverage
- **Clause**: POST-007, OP-DIAG-001
- **Gap**: No unit test explicitly verifies typed diagnostic outcomes for unsupported YAML/JSON/HTTP/text protocol attempts
- **Proposed test**: `crates/vb_yaml/src/profile_tests.rs` or new integration test verifying `ERR-UnsupportedRuntimeProtocol` diagnostic class is returned for YAML/JSON protocol attempts

### Gap 2: Bounded Ingress Backpressure
- **Clause**: PRE-006, ERR-Backpressure
- **Gap**: No unit test verifies bounded queue behavior under backpressure
- **Proposed test**: `crates/vb_runtime/tests/ingress_backpressure_tests.rs` - integration test verifying full queue rejects without growth
- **Note**: Covered by TLA+ (PO-005) and Loom (PO-013); unit test gap is acceptable

## Verification Approach

- **Unit tests**: Run `cargo test -p vb_yaml` and `cargo test -p vb_runtime` to verify existing tests pass
- **Integration tests**: Run `cargo test -p velvet_ballistics --test ir_artifact_admission` for admission scenarios
- **Formal verification**: TLA+ (PO-002 through PO-006) and Loom (PO-013) already provide coverage for temporal/concurrent behavior
- **Property tests**: Proptest already covers in `crates/vb_core/tests/proptest_core_types.rs`

## Recommendation

1. Run existing test suite to confirm all tests pass
2. Add `test_typed_diagnostic_unsupported_yaml` to `crates/vb_yaml/src/profile_tests.rs` for Gap 1
3. Skip Gap 2 unit test (formal verification provides adequate coverage)
4. Proceed to State 8 (test execution) with existing tests plus one new test