# Proof Coverage Matrix: vb-8mdp.7 State 5

## Verification Coverage by Obligation

| Obligation | Requirement | Verifier | Artifact | Evidence | Status |
|-----------|-------------|----------|----------|----------|--------|
| PO-001 | CC-DIGEST-001: Collect field coverage | TLA+ | `verification/tla/collect_body_model.tla` | `evidence/tlc-collect-body-model.log` | **PASS** (minimal model) |
| PO-002 | CC-DIGEST-001: Collect field coverage (Kani) | Kani | `verification/kani/collect_field_coverage.rs` | REWRITTEN, not executed | **PENDING_FORMAL_EXECUTION** |
| PO-003 | CC-DIGEST-001a: Variable field | Proptest | `crates/vb_compile/src/mod_compile_lowering/tests.rs:76` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-004 | CC-DIGEST-001a: Source field | Proptest | `tests.rs:96` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-005 | CC-DIGEST-001a: Pages field | Proptest | `tests.rs:116` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-006 | CC-DIGEST-001a: Items field | Proptest | `tests.rs:155` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-007 | CC-DIGEST-001a: Body recursive | Proptest | `tests.rs:194` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-008 | CC-DIGEST-001b: Step ID coverage | TLA+ | `verification/tla/collect_body_model.tla` | `evidence/tlc-collect-body-model.log` | **PASS** (TLC) |
| PO-008b | CC-DIGEST-001c: Trigger coverage | TLA+ | `verification/tla/collect_body_model.tla` | `evidence/tlc-collect-body-model.log` | **PASS** (TLC) |
| PO-009 | CC-DIGEST-002: Determinism | Proptest | `crates/vb_compile/src/tests/error_variant_tests.rs:853` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-010 | CC-DIGEST-003: Artifact dependency | Proptest | `error_variant_tests.rs:874` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-011 | CC-DIGEST-004: Lowering correctness | Verus | `verification/verus/collect_lowering.rs` | `evidence/verus-collect-lowering.log` | **PASS** (VACUUM — unbound to production) |
| PO-012 | CC-DIGEST-004: Lowering IR structure | TLA+ | `verification/tla/collect_body_model.tla` | `evidence/tlc-collect-body-model.log` | **PASS** (TLC) |
| PO-012b | CC-DIGEST-005: Digest mismatch detect | Integration | `workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-013 | CC-DIGEST-006: No panic on Collect | Kani | No specific harness | Harness missing | **GAP** |
| PO-014 | CC-DIGEST-007: Property equality | Proptest | `tests.rs:214` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-015 | H-2: ForEach field hashing | Kani | `verification/kani/foreach_field_coverage.rs` | REWRITTEN, not executed | **PENDING_FORMAL_EXECUTION** |
| PO-016 | H-2: Aggregate field hashing | Kani | `verification/kani/aggregate_field_coverage.rs` | REWRITTEN, not executed | **PENDING_FORMAL_EXECUTION** |
| PO-017 | H-4: Lowering determinism | TLA+ | `verification/tla/collect_body_model.tla` | `evidence/tlc-collect-body-model.log` | **PASS** (TLC) |
| PO-018 | H-5: Serialization determinism | Proptest | `error_variant_tests.rs:926` | Cannot compile | **BLOCKED_COMPILATION** |
| PO-020 | H-9: GOD RULE — no hardcoded | Kani | `verification/kani/collect_field_coverage.rs` | REWRITTEN, not executed | **PENDING_FORMAL_EXECUTION** |

## Summary

| Status | Count | Percentage |
|--------|-------|------------|
| PASS (TLC) | 5 | 23.8% |
| PASS (Verus — VACUUM) | 1 | 4.8% |
| PENDING_FORMAL_EXECUTION | 4 | 19.0% |
| BLOCKED_COMPILATION | 10 | 47.6% |
| GAP (harness missing) | 1 | 4.8% |
| **Total** | **21** | **100%** |

## Coverage by Verifier

| Verifier | Planned | Passed | Blocked | Pending |
|----------|---------|--------|---------|---------|
| TLA+ | 5 | 5 | 0 | 0 |
| Kani | 5 | 0 | 1 (no tool) | 4 |
| Proptest | 8 | 0 | 8 | 0 |
| Verus | 1 | 1 (vacuum) | 0 | 0 |
| Integration | 1 | 0 | 1 | 0 |
| **Total** | **21** | **6** | **10** | **4** |

## Production Source Coverage

| Source File | Lines | Obligations | Status |
|-------------|-------|-------------|--------|
| `vb_compile/src/mod_compile_lowering/part_05.rs:263-299` | 37 | PO-001 through PO-020 | FIXED in production |
| `vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | 54 | PO-011, PO-012, PO-017 | Not verified by Verus (vacuum) |

---
*Proof coverage matrix. State 5. Bead vb-8mdp.7. 2026-05-29.*
