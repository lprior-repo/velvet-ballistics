# Proof Coverage Matrix: vb-xi2f.38

## Coverage by Requirement

| Requirement | Clause Description | Verifiers | Obligation Coverage | Gaps |
|-------------|-------------------|-----------|---------------------|------|
| CC-DIGEST-001 | Digest content-addressing for Collect | TLA+, Kani, Proptest | PO-001, PO-002, PO-003..007 | None |
| CC-DIGEST-001a | Collect variable field hashing | Proptest | PO-003 | None |
| CC-DIGEST-001a | Collect source field hashing | Proptest | PO-004 | None |
| CC-DIGEST-001a | Collect pages field hashing | Proptest | PO-005 | None |
| CC-DIGEST-001a | Collect items field hashing | Proptest | PO-006 | None |
| CC-DIGEST-001a | Collect body recursive hashing | Kani, Proptest, TLA+ | PO-002, PO-007, PO-001 | None |
| CC-DIGEST-001b | Step ID coverage | TLA+, Proptest | PO-008, PS-007 | None |
| CC-DIGEST-001c | Trigger coverage | TLA+, Proptest | PO-008b, PS-008 | None |
| CC-DIGEST-002 | Digest determinism | Proptest, TLA+ | PO-009, PS-009 | None |
| CC-DIGEST-003 | Artifact digest depends on source | Proptest, TLA+ | PO-010, PS-010 | None |
| CC-DIGEST-004 | Collect lowering preserves semantics | Verus, TLA+ | PO-011, PO-012 | None |
| CC-DIGEST-005 | Digest mismatch detection | Integration test | PO-012b | None |
| CC-DIGEST-006 | No panic on Collect digest | Kani | PO-013 | None |
| CC-DIGEST-007 | Property-based digest equality | Kani, Proptest | PO-014, PS-014 | None |

## Coverage by Proof Seed

| Proof Seed ID | Description | Verifiers | Obligations | Status |
|---------------|-------------|-----------|-------------|--------|
| vb-xi2f.38-ps-001 | Digest content-addressing for Collect | Kani, Proptest, TLA+ | PO-001, PO-002, PO-003..007 | Covered |
| vb-xi2f.38-ps-002 | Collect variable field hashing | Proptest | PO-003 | Covered |
| vb-xi2f.38-ps-003 | Collect source field hashing | Proptest | PO-004 | Covered |
| vb-xi2f.38-ps-004 | Collect pages field hashing | Proptest | PO-005 | Covered |
| vb-xi2f.38-ps-005 | Collect items field hashing | Proptest | PO-006 | Covered |
| vb-xi2f.38-ps-006 | Collect body recursive hashing | Kani, Proptest, TLA+ | PO-002, PO-007, PO-001 | Covered |
| vb-xi2f.38-ps-007 | Step ID hashing | Proptest, TLA+ | PO-008, PS-007 | Covered |
| vb-xi2f.38-ps-008 | Trigger coverage | Proptest, TLA+ | PO-008b, PS-008 | Covered |
| vb-xi2f.38-ps-009 | Digest determinism | Proptest, TLA+ | PO-009, PS-009 | Covered |
| vb-xi2f.38-ps-010 | Artifact digest depends on source | Proptest, TLA+ | PO-010, PS-010 | Covered |
| vb-xi2f.38-ps-011 | Collect lowering correctness | Verus, TLA+ | PO-011, PO-012 | Covered |
| vb-xi2f.38-ps-012 | Digest mismatch detection | Integration test | PO-012b | Covered |
| vb-xi2f.38-ps-013 | No panic on Collect digest | Kani | PO-013 | Covered |
| vb-xi2f.38-ps-014 | Property-based digest equality | Kani, Proptest | PO-014, PS-014 | Covered |
| vb-xi2f.38-ps-015 | ForEach field hashing | Kani | PO-015 | Covered |
| vb-xi2f.38-ps-016 | Aggregate field hashing | Kani | PO-016 | Covered |
| vb-xi2f.38-ps-017 | Lowering determinism | TLA+ | PO-017 | Covered |
| vb-xi2f.38-ps-018 | Serialization determinism | Proptest | PO-018 | Covered |
| vb-xi2f.38-ps-019 | Pagination state integrity | Proptest, Integration | PS-019, PO-019 | Covered |
| vb-xi2f.38-ps-020 | GOD RULE: no hardcoded Collect | Kani | PO-020 | Covered |

**Total Proof Seeds**: 20  
**Fully Covered**: 20  
**Partially Covered**: 0  
**Not Covered**: 0

---

## Coverage by Hazard

| Hazard | Severity | Verifiers | Obligations | Status |
|--------|----------|-----------|-------------|--------|
| H-1: Collect fields not hashed | CRITICAL | Kani, Proptest, TLA+ | PO-001, PO-002, PO-003..007, PO-020 | Covered |
| H-2: ForEach/Aggregate same bug | HIGH | Kani, Proptest | PO-015, PO-016, PS-015, PS-016 | Covered |
| H-3: Digest collision | LOW | TLA+ | PS-010 (artifact digest model) | Covered (theoretical) |
| H-4: Lowering non-determinism | MEDIUM | TLA+, Proptest | PO-017, PS-017 | Covered |
| H-5: Serialization non-determinism | MEDIUM | Proptest | PO-018, PS-018 | Covered |
| H-6: Pagination state corruption | MEDIUM | Proptest, Integration | PS-019, PO-019 | Covered |
| H-7: Duplicate step IDs | MEDIUM | Validation | Mitigation (validation rejects duplicates) | Mitigated |
| H-8: Identical body steps, different parent | LOW | N/A | Not a bug (correct behavior) | N/A |
| H-9: Hardcoded harness data | CRITICAL | Kani | PO-013, PO-020 | Covered |

---

## Verifier-by-Verifier Coverage

### TLA+ (Model Checking)
- **Obligations**: PO-001, PO-008, PO-008b, PO-012, PO-017
- **Artifacts**: `verification/tla/collect_body_model.tla`
- **Invariants**: `CollectDigestCoverage`, `StepIdCoverage`, `TriggerCoverage`, `LoweringDeterminism`
- **Coverage**: CC-DIGEST-001 (field coverage), CC-DIGEST-001b (step ID), CC-DIGEST-001c (trigger), CC-DIGEST-004 (lowering), H-4 (lowering determinism)

### Kani (Bounded Model Checking)
- **Obligations**: PO-002, PO-013, PO-015, PO-016, PO-020
- **Artifacts**: `verification/kani/collect_field_coverage.rs`, `verification/kani/collect_try_from_parts.rs`, `verification/kani/foreach_field_coverage.rs`, `verification/kani/aggregate_field_coverage.rs`
- **Coverage**: H-1 (Collect fields), H-2 (ForEach/Aggregate), CC-DIGEST-006 (no panic), H-9 (GOD RULE)

### Proptest (Property-Based Testing)
- **Obligations**: PO-003, PO-004, PO-005, PO-006, PO-007, PO-009, PO-010, PO-014, PO-018
- **Artifacts**: `crates/vb_compile/src/tests/digest_collect_tests.rs`, `crates/vb_compile/src/tests/error_variant_tests.rs`
- **Coverage**: CC-DIGEST-001a (all 5 fields), CC-DIGEST-002 (determinism), CC-DIGEST-003 (artifact digest), CC-DIGEST-007 (equality property), H-5 (serialization)

### Verus (Formal Proof)
- **Obligations**: PO-011
- **Artifacts**: `verification/verus/collect_lowering.rs`
- **Coverage**: CC-DIGEST-004 (lowering correctness)

### Integration Test
- **Obligations**: PO-012b
- **Artifacts**: `crates/vb_storage/tests/vb_core_atomic_admission_red.rs`
- **Coverage**: CC-DIGEST-005 (mismatch detection), H-6 (pagination state)

---

## Uncovered Areas

**NONE** — All 20 proof seeds are fully covered by at least one verifier lane.

---

## Overlapping Coverage (Defense in Depth)

| Property | Primary | Secondary | Tertiary |
|----------|---------|-----------|----------|
| Collect field hashing | Kani | Proptest | TLA+ |
| Lowering correctness | Verus | TLA+ | — |
| Determinism | Proptest | TLA+ | — |
| No panic | Kani | — | — |
| GOD RULE compliance | Kani | proof-reviewer | — |

---

## Obligation Status Summary

| Status | Count |
|--------|-------|
| Planned | 19 |
| Blocked tooling | 0 |
| Waived | 0 |
| Not applicable | 4 (Flux, Loom, Miri, Fuzz) |
| Total lanes evaluated | 23 |
