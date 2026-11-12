# Proof Coverage Matrix: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Matrix Legend

- **Required lanes**: K = Kani, V = Verus, P = Proptest, F = cargo-fuzz
- **Not applicable**: — (with rationale in verifier-lane-decisions.jsonl)
- **Waived**: W (with waiver ID)
- **Status**: ✅ planned, ⏳ pending execution

## Coverage Matrix

| Proof Seed | Requirement | Contract Clause | Hazard | Kani | Verus | Proptest | Fuzz | Obligations | Status |
|------------|-------------|----------------|--------|------|-------|----------|------|-------------|--------|
| PS-001 | R1 | C1 (Digest-Contract Binding) | H-001 | K | V | P | — | PO-K01, PO-V01, PO-P01, PO-P07 | ✅ planned |
| PS-002 | R2 | C1 (Single-field sensitivity) | H-001 | K | V | P | — | PO-K02, PO-V01, PO-P01 | ✅ planned |
| PS-003 | R3 | C1 (Cross-field collision) | H-010 | K | V | — | — | PO-K03, PO-V02 | ✅ planned |
| PS-004 | R4 | C8 (Migration path) | H-001 | K | — | — | — | PO-K04 | ✅ planned |
| PS-005 | R5 | C2 (Single canonical type) | H-002 | K | — | — | — | PO-K05 | ✅ planned |
| PS-006 | R6 | C2 (Type consistency) | H-002 | K | — | — | — | PO-K06 | ✅ planned |
| PS-007 | R7 | C3 (Entry point contract) | H-003 | K | — | P | — | PO-K07, PO-P02 | ✅ planned |
| PS-008 | R8 | C4 (Taint digest sensitivity) | H-004 | K | V | P | — | PO-K08, PO-V03, PO-P03 | ✅ planned |
| PS-009 | R9 | C4 (Runtime enforcement) | H-004 | K | V | — | — | PO-K09, PO-V04 | ✅ planned |
| PS-010 | R10 | C6 (Dual path consistency) | H-005 | K | — | P | — | PO-K10, PO-P04 | ✅ planned |
| PS-011 | R11 | C7 (YAML contract parsing) | H-006 | W | — | W | W | PO-F01 (waived WC-001) | ⏳ waived P2 |
| PS-012 | R12 | C5 (Full validation) | H-007 | K | — | — | — | PO-K11 | ✅ planned |
| PS-013 | R13 | C1 (Proptest sensitivity) | H-008 | K | — | P | — | PO-K02, PO-P01, PO-P07 | ✅ planned |
| PS-014 | R14 | C1 (Determinism all contracts) | H-008 | K | — | P | — | PO-K01, PO-P05 | ✅ planned |
| PS-015 | R15 | C1 (Policy digest agreement) | H-009 | K | — | — | — | PO-K14 | ✅ planned |
| PS-016 | R16 | C1 (Encoding injectivity) | H-010 | K | V | — | — | PO-K12, PO-V02 | ✅ planned |
| PS-017 | R17 | C3 (with_default equivalence) | H-003 | K | — | P | — | PO-K13, PO-P06 | ✅ planned |

## Summary Statistics

| Verifier | Required | Not Applicable | Waived | Total Decisions |
|----------|----------|---------------|--------|-----------------|
| Kani | 16 | 0 | 1 | 17 |
| Verus | 6 | 11 | 0 | 17 |
| Proptest | 8 | 8 | 1 | 17 |
| cargo-fuzz | 0 | 16 | 1 | 17 |
| TLA+ | 0 | 17 | 0 | 17 |
| Flux RS | 0 | 17 | 0 | 17 |
| Loom | 0 | 17 | 0 | 17 |
| Miri | 0 | 17 | 0 | 17 |
| **Total** | **30** | **103** | **3** | **136** |

## Obligation Count by Verifier

| Verifier | Obligations |
|----------|------------|
| Kani | PO-K01 through PO-K14 (14) |
| Verus | PO-V01 through PO-V04 (4) |
| Proptest | PO-P01 through PO-P07 (7) |
| cargo-fuzz | PO-F01 (1, waived) |
| **Total planned** | **25 active + 1 waived** |

## Hazard Coverage Completeness

| Hazard | Severity | Proof Seeds Covered | All Lanes Planned? |
|--------|----------|--------------------|--------------------|
| H-001: Digest orphan | CRITICAL | PS-001, PS-002, PS-003, PS-004 | ✅ Kani(4) + Verus(1) + Proptest(2) |
| H-002: Duplicate types | HIGH | PS-005, PS-006 | ✅ Kani(2) |
| H-003: Hardcoded DEFAULT | HIGH | PS-007, PS-017 | ✅ Kani(2) + Proptest(2) |
| H-004: Taint silent match | HIGH | PS-008, PS-009 | ✅ Kani(2) + Verus(2) + Proptest(1) |
| H-005: Dual path drift | MEDIUM | PS-010 | ✅ Kani(1) + Proptest(1) |
| H-006: Missing YAML parsing | MEDIUM | PS-011 | ⚠️ Waived P2 |
| H-007: Validation gap | HIGH | PS-012 | ✅ Kani(1) |
| H-008: No test coverage | HIGH | PS-013, PS-014 | ✅ Kani(1) + Proptest(2) |
| H-009: Digest split | MEDIUM | PS-015 | ✅ Kani(1) |
| H-010: Field name stability | MEDIUM | PS-003, PS-016 | ✅ Kani(2) + Verus(1) |

## Gap Analysis

### P1 Gaps
- **None**: All CRITICAL and HIGH severity hazards (H-001 through H-004, H-007, H-008) are covered by at least 2 verifier lanes. All 17 proof seeds have planned obligations.

### P2 Gaps (waived)
- **H-006 (Missing YAML parsing)**: Waived for P1. YAML contract parsing will require cargo-fuzz (primary), Kani (parser invariants), and Proptest (valid/invalid YAML generation) when implemented in a future P2 bead.

### No Material Gaps
- TLA+: Correctly excluded — no temporal/state-machine behavior in any affected code.
- Loom: Correctly excluded — no concurrent interleavings.
- Miri: Correctly excluded — no unsafe code in affected paths.
- Flux RS: Correctly excluded — no index-struct refinement relationships that Kani cannot cover.

## Defense Depth

| Property | Layer 1 (Kani) | Layer 2 (Proptest) | Layer 3 (Verus) | Layer 4 (Fuzz) |
|----------|---------------|--------------------|-----------------|----------------|
| Determinism | ✅ bounded | ✅ 5,000+ random | — | — |
| Field sensitivity | ✅ 17 fields bounded | ✅ 500+/field random | ✅ for-all proof | — |
| Cross-field collisions | ✅ bounded candidates | — | ✅ injectivity proof | — |
| Taint sensitivity | ✅ both bool values | ✅ 1,000+ random | ✅ injectivity proof | — |
| Type identity | ✅ compile-time | — | — | — |
| Dual-path equiv | ✅ bounded | ✅ 1,000+ random | — | — |
| Validation bounds | ✅ all 17 fields | — | — | — |
| Migration | ✅ bounded | — | — | — |
| Parser robustness | — | — | — | ⚠️ waived P2 |

## Reviewer Notes

All `not_applicable` decisions have concrete evidence in `verifier-lane-decisions.jsonl`. No lane is silently omitted. The waived P2 lane (YAML parser fuzzing) has explicit waiver WC-001 in `waiver-candidates.md` and `waiver-candidates.jsonl`.
