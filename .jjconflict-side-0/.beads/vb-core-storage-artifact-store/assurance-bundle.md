# Assurance Bundle: vb-core-storage-artifact-store

**Bead**: vb-core-storage-artifact-store
**Date**: 2026-05-16
**Pipeline State**: 13 (Evidence Packaging)
**Workspace**: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-storage-artifact-store`

## Requirement-to-Evidence Mapping

### PRE-001: Strict/journaled runtime requires storage-backed artifact store
- **Contract clause**: PRE-001
- **Tests**: `pre_001_strict_constructor_requires_storage_artifact_store` (19-pass suite)
- **Proofs**: `STATIC-ADM-001` (static policy)
- **Evidence**: `crates/vb_runtime/src/admission.rs` lines 192, 274, 391; `crates/vb_runtime/src/runtime.rs` lines 49, 58
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### PRE-002: Run request names artifact digest
- **Contract clause**: PRE-002
- **Tests**: `strict_runtime_rejects_when_artifact_missing_before_allocation` (19-pass suite)
- **Proofs**: `INT-ADM-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### PRE-003: CLI persistence writes AcceptedArtifact envelope
- **Contract clause**: PRE-003
- **Tests**: `strict_runtime_rejects_malformed_stored_bytes`, `journaled_cli_uses_same_journal_for_persist_and_admit`
- **Proofs**: `CODEC-ADM-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### PRE-004: Gate count validation (15-gate schema)
- **Contract clause**: PRE-004
- **Tests**: `strict_runtime_rejects_gate_count_mismatch`, `strict_runtime_rejects_failed_gate`
- **Proofs**: `VERUS-ADM-001`, `GATE-ADM-001`
- **Evidence**: Verus 16 verified/0 errors; TLC 288 states/144 distinct/depth 6/0 errors
- **Waiver**: `WAIVER-GATE-REFINE-001` (deferred to vb-core-proof-15-gate)
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### PRE-005: Capability exactness
- **Contract clause**: PRE-005
- **Tests**: `strict_runtime_rejects_capability_mismatch` (19-pass suite)
- **Proofs**: `VERUS-ADM-002` (16 verified/0 errors)
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### POST-001: Missing artifact returns typed error before allocation
- **Contract clause**: POST-001
- **Tests**: `strict_runtime_rejects_when_artifact_missing_before_allocation` (19-pass suite)
- **Proofs**: `TLA-ADM-001`, `INT-ADM-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### POST-002: Malformed bytes return typed error with digest context
- **Contract clause**: POST-002
- **Tests**: `strict_runtime_rejects_malformed_stored_bytes`, `digest_mismatch_reports_requested_and_stored_digest` (19-pass suite)
- **Proofs**: `INT-ADM-002`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### POST-003: Validation failures return typed rejection
- **Contract clause**: POST-003
- **Tests**: `strict_runtime_rejects_gate_count_mismatch`, `strict_runtime_rejects_failed_gate`, `strict_runtime_rejects_capability_mismatch` (19-pass suite)
- **Proofs**: `TLA-ADM-001`, `VERUS-ADM-001`, `VERUS-ADM-002`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### POST-004: Valid artifact succeeds without AlwaysPresentArtifactStore
- **Contract clause**: POST-004
- **Tests**: `strict_runtime_admits_when_storage_contains_valid_accepted_artifact` (19-pass suite)
- **Proofs**: `INT-CLI-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### POST-005: CLI uses one coherent storage boundary
- **Contract clause**: POST-005
- **Tests**: `strict_and_journaled_cli_share_opened_storage_boundary_for_artifact_persist_and_admit` (19-pass suite)
- **Proofs**: `INT-CLI-001`, `STATIC-ADM-001`
- **Waiver**: POST-005 TLA+ four-boundary coherence waived
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### INV-001: AlwaysPresentArtifactStore cannot satisfy strict production
- **Contract clause**: INV-001
- **Tests**: `inv_001_dummy_store_not_used_by_production_strict_paths` (2-pass static suite)
- **Proofs**: `STATIC-ADM-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### INV-002: No RunAccepted before successful admission
- **Contract clause**: INV-002
- **Tests**: `inv_002_no_runaccepted_before_successful_admission` (19-pass suite)
- **Proofs**: `TLA-ADM-001`, `INT-ADM-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### INV-003: Gate schema mismatch rejects
- **Contract clause**: INV-003
- **Tests**: `inv_003_gate_schema_mismatch_rejects` (19-pass suite)
- **Proofs**: `VERUS-ADM-001`, `GATE-ADM-001`, `WAIVER-GATE-REFINE-001`, `WAIVER-TOOLING-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### INV-004: Digest identity preserved in diagnostics
- **Contract clause**: INV-004
- **Tests**: `post_002_malformed_artifact_preserves_digest`, `digest_mismatch_reports_requested_and_stored_digest` (19-pass suite)
- **Proofs**: `VERUS-ADM-002`, `INT-ADM-002`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### INV-005: Relaxed/test paths explicit
- **Contract clause**: INV-005
- **Tests**: `relaxed_runtime_can_use_dummy_store_only_when_explicit` (19-pass suite)
- **Proofs**: `STATIC-ADM-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

### INV-006: No YAML/JSON/HTTP fallback in runtime core
- **Contract clause**: INV-006
- **Tests**: `inv_006_runtime_core_has_no_yaml_json_http_fallback` (2-pass static suite)
- **Proofs**: `STATIC-CORE-001`
- **Review**: `contract-verification-review.md: STATUS: APPROVED`

## Formal Verification Results Summary

| id | layer | result | evidence |
|---|---|---|---|
| TLA-ADM-001 | tla-plus/tlc | PASS | 288 states, 144 distinct, depth 6, 0 errors |
| VERUS-ADM-001 | verus | PASS | 16 verified, 0 errors |
| VERUS-ADM-002 | verus | PASS | 16 verified, 0 errors (co-run) |
| GATE-ADM-001 | gauntlet-proof | PASS | `moon run :verify-proof` exit 0 |
| WAIVER-GATE-REFINE-001 | waiver | WAIVED | Owner: vb-core-proof-15-gate |
| WAIVER-TOOLING-001 | waiver | WAIVED | Tooling resolved |

## Unresolved Waiver/Deferred Debt Table

| id | classification | owner | reason |
|---|---|---|---|
| WAIVER-GATE-REFINE-001 | WAIVED | vb-core-proof-15-gate | 15-gate schema deferred upstream |
| WAIVER-TOOLING-001 | WAIVED | proof-lane-tooling | Tooling resolved |
| INT-ADM-001 | DEFERRED_GLOBAL | State 8 | Pre-existing global debt |
| INT-ADM-002 | DEFERRED_GLOBAL | State 8 | Pre-existing global debt |
| INT-CLI-001 | DEFERRED_GLOBAL | State 8 | Pre-existing global debt |
| STATIC-ADM-001 | DEFERRED_GLOBAL | State 8 | Pre-existing global debt |
| STATIC-CORE-001 | DEFERRED_GLOBAL | State 8 | Pre-existing global debt |
| CODEC-ADM-001 | DEFERRED_GLOBAL | State 8 | Pre-existing global debt |

## Test Evidence Summary

| Suite | Tests | Exit | Result |
|---|---|---|---|
| vb_runtime storage_artifact_store_contract | 19 | 0 | PASS |
| vb_storage accepted_artifact_codec_contract | 2 | 0 | PASS |
| static_storage_artifact_store_policy | 2 | 0 | PASS |
| **TOTAL** | **23** | **0** | **PASS** |

## Review Approval Chain

| Review | Status | Line |
|---|---|---|
| proof-review.md | APPROVED | line 3 |
| contract-verification-review.md | APPROVED | line 3 |
| test-plan-review.md | APPROVED | line 3 |
| test-suite-review.md | APPROVED | line 3 |
| formal-verification-report.md | APPROVED | line 3 |
| black-hat-review.md | APPROVED | line 3 |
