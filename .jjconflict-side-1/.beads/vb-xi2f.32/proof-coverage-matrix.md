# Proof Coverage Matrix: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** proof-planner (State 4)
**Schema:** proof-coverage-matrix/v1

## 1. Requirements → Proof Layer Mapping

| Requirement | Clause | Kani | proptest | cargo-fuzz | Regression Tests |
|------------|--------|------|----------|------------|-----------------|
| C1 | Wait field hashing | PO-001, PO-013 | PO-002 | PO-003 | — |
| C2 | WaitUntil vs WaitEvent | PO-005 | PO-004 | — | — |
| C3 | Absent field sentinels | — | PO-006 | PO-007 | — |
| C4 | Digest determinism | — | PO-008 | — | existing stability tests |
| C5 | Dual implementation | PO-010 | PO-009, PO-016 | — | — |
| C6 | Stability regression | — | PO-014 | — | existing test suite |
| C1 (panic) | Panic-freedom | PO-015 | — | — | — |

## 2. Proof Seed → Obligation Traceability

| Proof Seed | Obligations | Layer Coverage |
|-----------|-------------|---------------|
| ps-wait-001 | PO-001, PO-002, PO-003 | Kani + proptest + fuzz |
| ps-wait-002 | PO-004, PO-005 | proptest + Kani |
| ps-wait-003 | PO-006, PO-007 | proptest + fuzz |
| ps-wait-004 | PO-008 | proptest |
| ps-wait-005 | PO-009, PO-010 | proptest + Kani |
| ps-wait-006 | PO-011, PO-012, PO-013 | proptest + fuzz + Kani |
| ps-wait-007 | PO-014 | proptest (regression) |
| ps-wait-008 | PO-015 | Kani |
| ps-wait-009 | PO-010, PO-016 | Kani + proptest |

## 3. Hazard Coverage

| Hazard ID | Severity | Proof Layer Coverage | Status |
|-----------|----------|---------------------|--------|
| RCIH-1: Digest Collision (THE BUG) | HIGH | PO-002, PO-003, PO-001, PO-013 | Planned — proptest + fuzz + Kani |
| RCIH-2: Post-Fix Collision | LOW | PO-004, PO-005 | Planned — proptest + Kani |
| RCIH-3: Hash Ordering Collision | NONE | PO-002, PO-011 | Covered implicitly by sensitivity tests |
| RCIH-4: Duplicate Code Divergence | HIGH | PO-009, PO-010, PO-016 | Planned — proptest + Kani |
| RCIH-5: Empty Steps Digest | NONE | — | Not applicable |
| TH-3: Post-Fix Persistence Break | MEDIUM | — | Out of scope; documented |
| RAH-1: Incompatible Digest Change | MEDIUM | — | Out of scope; documented |
| RAH-2: Proptest Regression | NONE | PO-008, PO-014 | Regression guard |
| RH-1: Discrimination Mismatch | LOW | PO-004, PO-005 | Planned — proptest + Kani |
| RH-2: Validation Bypass | LOW | — | Trusted base (validation gate) |
| HIH-1: Crafted Hash Collision | NONE | PO-003, PO-007, PO-012 | Fuzz covers adversarial inputs |

## 4. Implementation Files → Proof Traceability

| File | Fix Required | Proof Coverage |
|------|-------------|---------------|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140` | Add Wait match arm | PO-001, PO-002, PO-003, PO-004, PO-005, PO-006, PO-007, PO-015 |
| `crates/vb_compile/src/compile/mod.rs:243` | Add Wait match arm (same fix) | PO-009, PO-010, PO-015, PO-016 |
| `crates/vb_compile/tests/v1_primitive_lowering.rs` | Add sensitivity proptests | PO-002, PO-004, PO-006, PO-008, PO-011 |
| `crates/vb_compile/tests/` | Cross-path test | PO-009, PO-016 |
| `verification/kani/` | Kani harnesses | PO-001, PO-005, PO-010, PO-013, PO-015 |
| `fuzz/fuzz_targets/` | Fuzz targets | PO-003, PO-007, PO-012 |

## 5. Coverage Gaps and Waiver Candidates

| Gap | Risk | Mitigation | Waiver? |
|-----|------|-----------|---------|
| No Verus proof of digest correctness | NONE (P1 scope) | Kani + proptest + fuzz provide stronger direct coverage | Waiver candidate WC-001 |
| No TLA+ model of digest lifecycle | NONE (pure function) | Not applicable — no temporal behavior | n/a |
| No Flux refinement of Wait shape | NONE (validation-enforced) | Not applicable — shape is validated upstream | n/a |
| Validation bypass not proven | LOW | Trusted base: validation runs before digest | Trusted-base note TB-002 |
| blake3 collision resistance not proven | NONE (cryptographic assumption) | Industry-standard primitive; out of scope | Trusted-base note TB-001 |
| (None, None) shape not handled in digest | LOW (validated away) | Defensive: could hash sentinel but never reached | Acceptable gap per RH-2 |

## 6. Defense Depth Summary

```
┌─────────────────────────────────────────────────┐
│ Layer 4: Regression (existing tests)             │
│  PO-014: Run existing stability tests            │
├─────────────────────────────────────────────────┤
│ Layer 3: Fuzz (adversarial collision hunting)    │
│  PO-003, PO-007, PO-012                          │
├─────────────────────────────────────────────────┤
│ Layer 2: proptest (broad input-space coverage)   │
│  PO-002, PO-004, PO-006, PO-008, PO-009,         │
│  PO-011, PO-016                                  │
├─────────────────────────────────────────────────┤
│ Layer 1: Kani (bounded proof)                    │
│  PO-001, PO-005, PO-010, PO-013, PO-015          │
└─────────────────────────────────────────────────┘
```
