# Assurance Bundle — vb-core-lower-values-actions-refs

**Bead**: vb-core-lower-values-actions-refs
**Workspace**: /tmp/vb-ws/vb-core-lower-values-actions-refs
**State**: 13
**Date**: 2026-05-15

---

## STATUS: COMPLETE

---

## Requirement-to-Evidence Mapping

| Requirement | Contract Clause | Proof/Test Obligation | Execution Evidence | Review Status |
|---|---|---|---|---|
| AC-1: YAML no longer requires low-level slots/actions | POST-001, POST-002 | KANI-SLOT-REF-001, KANI-ACCESSOR-REF-001 | 264 tests PASS | APPROVED |
| AC-2: Invalid references fail before runtime | ERR-* (11 variants) | ERR-TAXONOMY-001, KANI-SLOT-REF-001 | 264 tests PASS | APPROVED |
| AC-3: Lowered IR preserves semantics | POST-003, POST-008 | UNIT-EXPR-BYTESTACK-001, INV-TAINT-001 | 32 taint tests PASS | APPROVED |
| AC-4: Runtime receives numeric only | POST-004, POST-005 | KANI-EXPR-BYTECODE-001, KANI-CONSTANT-POOL-001 | 264 tests PASS | APPROVED |

---

## Artifact Inventory

| Artifact | Path | Status |
|---|---|---|
| STATE.md | `.beads/vb-core-lower-values-actions-refs/STATE.md` | EXISTS |
| baseline-report.md | `.beads/vb-core-lower-values-actions-refs/baseline-report.md` | EXISTS |
| codebase-map.md | `.beads/vb-core-lower-values-actions-refs/codebase-map.md` | EXISTS |
| delivery-scope.jsonl | `.beads/vb-core-lower-values-actions-refs/delivery-scope.jsonl` | EXISTS |
| contract.md | `.beads/vb-core-lower-values-actions-refs/contract/contract.md` | EXISTS |
| traceability-matrix.jsonl | `.beads/vb-core-lower-values-actions-refs/contract/traceability-matrix.jsonl` | EXISTS |
| proof-obligations.jsonl | `.beads/vb-core-lower-values-actions-refs/proof-obligations.jsonl` | EXISTS |
| proof-strategy.md | `.beads/vb-core-lower-values-actions-refs/proof-strategy.md` | EXISTS |
| proof-review.md | `.beads/vb-core-lower-values-actions-refs/proof-review.md` | EXISTS |
| contract-verification-review.md | `.beads/vb-core-lower-values-actions-refs/contract-verification-review.md` | EXISTS |
| test-plan.md | `.beads/vb-core-lower-values-actions-refs/test-plan.md` | EXISTS |
| test-suite-review.md | `.beads/vb-core-lower-values-actions-refs/test-suite-review.md` | EXISTS |
| implementation.md | `.beads/vb-core-lower-values-actions-refs/implementation.md` | EXISTS |
| formal-verification-report.md | `.beads/vb-core-lower-values-actions-refs/formal-verification-report.md` | EXISTS |
| machine-gate-report.md | `.beads/vb-core-lower-values-actions-refs/machine-gate-report.md` | EXISTS |
| verification-ledger.jsonl | `.beads/vb-core-lower-values-actions-refs/verification-ledger.jsonl` | EXISTS |
| black-hat-review.md | `.beads/vb-core-lower-values-actions-refs/black-hat-review.md` | EXISTS |
| `crates/vb_compile/src/kani/mod.rs` | workspace | EXISTS |
| `crates/vb_compile/src/lib.rs` | workspace | EXISTS |
| `scripts/rust-verification-gauntlet.sh` | workspace | EXISTS |

---

## Unresolved Waiver/Debt Table

| Obligation | Waiver | Owner | Reason | Expiry |
|---|---|---|---|---|
| VERUS-EXPR-STACK-001 | WAIVER-VERUS-EXPR-STACK | proof-planner | Verus not installed | Until Verus in CI |
| VERUS-SLOT-MAX-001 | WAIVER-VERUS-SLOT-MAX | proof-planner | Verus not installed | Until Verus in CI |
| INV-006-ORDER-001 | optional | proof-planner | Order-preserving invariant | N/A |
| INV-007-NODEDUP-001 | optional | proof-planner | Node deduplication invariant | N/A |

Compensating evidence: Kani + proptest covers the same bounded model checking scope as Verus would. 264 unit tests provide empirical coverage.

---

## Assurance Bundle: COMPLETE
