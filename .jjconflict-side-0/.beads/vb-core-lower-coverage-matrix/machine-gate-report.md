# Machine Gate Report — vb-core-lower-coverage-matrix

## Gate Execution Summary
- **Bead**: vb-core-lower-coverage-matrix
- **Execution Date**: 2026-05-17
- **Gate**: State 11 — Formal Verification

## Command Evidence

### Cargo Test Gate
```bash
cd /home/lewis/src/velvet-ballistics && rtk cargo test -p vb_compile
```
**Result**: 294 tests PASSED (5 suites, 12.26s)

### Verus Verification Gate
```bash
cd /home/lewis/src/velvet-ballistics && verus verification/verus/v1_primitive_lowering.rs
```
**Result**: 15 verified, 0 errors

## Verification Ledger Summary
| Obligation | Mode | Result |
|------------|------|--------|
| PO-001 (INV-001 Node Density) | verify-standard | PASS |
| PO-002 (INV-002 Slot Bounds) | verify-proof | PASS |
| PO-003 (INV-003 Target Range) | verify-proof | PASS |
| PO-004 (INV-004 Determinism) | verify-standard | PASS |
| PO-005 (POST-001 Primitives) | verify-standard | PASS |
| PO-006 (POST-002 Unsupported) | verify-standard | PASS |
| PO-007 (POST-003 Error Variants) | verify-standard | PASS |
| PO-008 (POST-003 API Parity) | verify-standard | PASS |
| PO-GAP-001 (vars) | waiver | WAIVED |
| PO-GAP-002 (secrets) | waiver | WAIVED |
| PO-GAP-003 (examples) | waiver | WAIVED |

## Regression Analysis
No regression detected. All 294 tests pass.

## Blocker Classification
- **BLOCK_LOCAL**: ATTACK-001 (incomplete construct coverage) - scope limitation
- **BLOCK_RELEASE**: None
- **WAIVED**: vars, secrets, examples gaps

**STATUS**: PASS