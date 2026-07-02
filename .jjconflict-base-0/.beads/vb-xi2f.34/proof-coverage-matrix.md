# Proof Coverage Matrix — vb-xi2f.34: Finish Digest Coverage

**Bead**: vb-xi2f.34  
**Phase**: p4-proof-planner  
**Date**: 2026-05-24  

---

## Coverage by Contract Clause

| Clause | Description | Kani | Proptest | Integration | Static | Status |
|---|---|---|---|---|---|---|
| **C1** | Finish result value sensitivity | PO-KANI-001 (String), PO-KANI-002 (Integer) | PO-PROPTEST-002 | PO-INT-001 | — | Full |
| **C2** | Finish step ID sensitivity | — | (via C3 coverage) | PO-INT-002 | — | Full |
| **C3** | Finish step position sensitivity | — | PO-PROPTEST-003 | — | — | Full |
| **C4** | Canonical digest determinism | — | PO-PROPTEST-001 | — | — | Full |
| **C5** | Hash discrimination (variant) | PO-KANI-003 | — | PO-INT-003 | — | Full |
| **C6** | Digest survives compilation | — | — | PO-INT-001 | — | Full |
| **C7** | Single implementation | — | — | PO-INT-004 | — | Full |
| **C8** | Forward compatibility | — | — | — | PO-STATIC-001 | Full |
| **C9** | Pre-validation digest scope | — | PO-PROPTEST-004 | — | — | Full |
| **C10** | Exclusion of runtime | — | — | — | PO-STATIC-002 | Full |

---

## Coverage by Proof Seed

| Seed ID | Obligations | Gaps Closed |
|---|---|---|
| PS-FINISH-DIGEST-001 | PO-KANI-001, PO-PROPTEST-002, PO-INT-001 | GAP-2 |
| PS-FINISH-DIGEST-002 | PO-KANI-003, PO-INT-003 | GAP-4, GAP-5 |
| PS-FINISH-DIGEST-003 | PO-PROPTEST-001, PO-INT-002 | GAP-1, GAP-3 |
| PS-FINISH-DIGEST-004 | PO-INT-004 | — |
| PS-FINISH-DIGEST-005 | PO-STATIC-001 | GAP-5 |
| PS-FINISH-DIGEST-006 | PO-INT-001 | GAP-2, GAP-6 |
| PS-FINISH-DIGEST-007 | PO-PROPTEST-004 | — |
| PS-FINISH-DIGEST-008 | PO-STATIC-002 | — |
| PS-FINISH-DIGEST-009 | PO-KANI-002 | GAP-2 |
| PS-FINISH-DIGEST-010 | PO-PROPTEST-003 | GAP-3 |

---

## Hazard Coverage

| Hazard | Severity | Covered By | Mitigation |
|---|---|---|---|
| HAZ-1: Duplicate divergence | HIGH | PO-INT-004 (equivalence test) | Test verifies both paths produce same output |
| HAZ-2: Silent hash collapse | MEDIUM | PO-STATIC-001 (exhaustiveness) | Test asserts _ arm unreachable for current variants |
| HAZ-3: No cross-validation | MEDIUM | PO-INT-001 (round-trip) | Integration test verifies digest survives compile |
| HAZ-4: Integer encoding | LOW | PO-KANI-002 (injectivity) | Kani proves i64.to_le_bytes() is injective |
| HAZ-5: String/Integer collision | LOW | PO-KANI-003 (discrimination) | Kani proves byte sequences differ |
| HAZ-6: Digest before validation | LOW | Documented design intent | — |
| HAZ-7: Trigger _ arm | LOW | Out of scope (separate bead) | Not a Finish concern |
| HAZ-8: Empty step ID | LOW | Parser concern (out of scope) | — |
| HAZ-9: canonical_primitive_name | LOW | WC-001 waiver | Not a Finish concern; waived for this bead |

---

## Gap Closure Summary

| Gap | Description | Closed By | Status |
|---|---|---|---|
| GAP-1 | No canonical_digest() unit tests | PO-PROPTEST-001 (determinism proptest) | Planned |
| GAP-2 | No test that changing finish result value changes digest | PO-KANI-001, PO-KANI-002, PO-PROPTEST-002, PO-INT-001 | Planned |
| GAP-3 | No test that changing finish step ID changes digest | PO-INT-002 | Planned |
| GAP-4 | No test that changing result type changes digest | PO-INT-003, PO-KANI-003 | Planned |
| GAP-5 | No test for _ fallback arm | PO-STATIC-001 (exhaustiveness assertion) | Planned |
| GAP-6 | No integration test for finish semantic change → compiled digest | PO-INT-001 | Planned |
