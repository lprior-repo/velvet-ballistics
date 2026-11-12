# Proof Coverage Matrix — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10
**Phase**: State 4 — Proof Planning
**Date**: 2026-05-24

---

## 1. Seed → Obligation Traceability

| Proof Seed | Requirement | Domain Claim | Kani | proptest | fuzz | mutants | gauntlet |
|-----------|-------------|--------------|------|----------|------|---------|----------|
| PS-001 | C-SYM-2 | from_static validates against registry | PO-001 | PO-016 | — | PO-027 | PO-028 |
| PS-002 | C-REG-3 | Registry bijection (no duplicates) | PO-002 | PO-016, PO-023 | — | PO-027 | PO-028 |
| PS-003 | C-SYM-3 | SymbolicCode↔DiagnosticCode round-trip | PO-002 | PO-016, PO-023 | — | PO-027 | PO-028 |
| PS-004 | C-VE-2 | ValidationError::code() exhaustive, registered | PO-003 | — | — | PO-027 | PO-028 |
| PS-005 | C-VE-6 | 58 variants → 58 unique codes | — | PO-017 | — | PO-027 | PO-028 |
| PS-006 | C-DC-2 | is_supported_code() accepts all ranges | PO-004 | PO-018 | — | PO-027 | PO-028 |
| PS-007 | C-DIAG-2 | Diagnostic constructor consistency | PO-005 | PO-019 | — | PO-027 | PO-028 |
| PS-008 | C-YE-1 | YamlError::code() registered | PO-006 | (PO-025) | — | PO-027 | PO-028 |
| PS-009 | C-CE-2 | CompileError codes all registered | — | PO-020 | — | PO-027 | PO-028 |
| PS-010 | C-SYM-6 | Zero-allocation (perf invariant) | PO-007 | — | — | — | PO-028 |
| PS-011 | C-BC-1 | DiagnosticCode::from_str backward compat | PO-008 | — | — | PO-027 | PO-028 |
| PS-012 | C-SYM-5 | Serialization round-trip + reject unknown | PO-009 | PO-021 | PO-022 | PO-027 | PO-028 |
| PS-013 | C-REG-4 | All numeric codes non-zero | PO-010 | (PO-023) | — | PO-027 | PO-028 |
| PS-014 | C-REG-5 | Category matches numeric high byte | PO-011 | PO-023 | — | PO-027 | PO-028 |
| PS-015 | C-DC-3 | DiagnosticCode::symbolic_code() reverse-lookup | PO-012 | PO-023 | — | PO-027 | PO-028 |
| PS-016 | C-VE-3 | 36 Section 16 codes match master contract | — | PO-024 | — | PO-027 | PO-028 |
| PS-017 | C-TRAIT-3 | HasSymbolicCode pure, total, deterministic | PO-013 | — | — | PO-027 | PO-028 |
| PS-018 | C-FS-6 | Diagnostic no mismatched codes | PO-014 | — | — | PO-027 | PO-028 |
| PS-019 | C-OTH-1 | CoreError/RuntimeError/JournalError registered | PO-015 | PO-025 | — | PO-027 | PO-028 |
| PS-020 | GAP-6 | diag_codes.rs promoted to public | — | PO-026 | — | PO-027 | PO-028 |

**Legend**: — = not required for this seed. (PO-xxx) = covered by shared obligation, not primary.

---

## 2. Coverage Summary by Verifier

| Verifier | Obligations | Seeds Covered | Key Target |
|----------|------------|---------------|------------|
| **Kani** | PO-001–PO-015 (15) | PS-001–PS-004, PS-006–PS-008, PS-010–PS-015, PS-017–PS-019 | Bounded verification of registry invariants, code mappings, parser correctness, constructor invariants |
| **proptest** | PO-016–PO-021, PO-023–PO-026 (10) | PS-001–PS-003, PS-005, PS-006, PS-009, PS-012–PS-016, PS-019, PS-020 | Enumeration and property testing of error variants, registry consistency, master contract parity (PO-022 is cargo-fuzz, listed separately) |
| **cargo-fuzz** | PO-022 (1) | PS-012 | Hostile input deserialization boundary |
| **cargo-mutants** | PO-027 (1) | All 20 | Mutation resistance for test suite quality |
| **moon ci** | PO-028 (1) | All 20 | Aggregate CI gate |

**Total obligations**: 28 (15 Kani + 10 proptest + 1 fuzz + 1 mutation)

---

## 3. Requirement Coverage

| Requirement | Seeds | Obligations | Status |
|-------------|-------|------------|--------|
| REQ-1 (Section 16 public diagnostics) | PS-001, PS-004, PS-008, PS-016 | PO-001, PO-003, PO-006, PO-024 | Covered |
| REQ-2 (Numeric codes internal) | PS-011 | PO-008 | Covered |
| REQ-3 (Regression asserts symbolic strings) | PS-004, PS-005, PS-009 | PO-003, PO-017, PO-020 | Covered |
| GAP-1 (SymbolicCode type) | PS-001 | PO-001, PO-016 | Covered |
| GAP-2 (ValidationError code()) | PS-004 | PO-003, PO-017 | Covered |
| GAP-3 (YamlError code()) | PS-008 | PO-006 | Covered |
| GAP-4 (is_supported_code ranges) | PS-006 | PO-004, PO-018 | Covered |
| GAP-5 (Cross-crate registry) | PS-002, PS-016 | PO-002, PO-023, PO-024 | Covered |
| GAP-6 (diag_codes.rs promotion) | PS-020 | PO-026 | Covered |

---

## 4. Risk Tag Coverage

| Risk Tag | Seeds | Primary Lanes |
|----------|-------|--------------|
| `invariant` | PS-001–PS-007, PS-010, PS-012–PS-015, PS-017, PS-018 | Kani + proptest |
| `contract` | PS-001–PS-009, PS-011, PS-012, PS-014, PS-015, PS-017–PS-020 | Kani + proptest + fuzz |
| `domain` | PS-004, PS-005, PS-008, PS-009, PS-013, PS-014, PS-016, PS-017, PS-019 | proptest (enumeration) |
| `refinement` | PS-001 | Kani + proptest |
| `release` | PS-002, PS-011 | Kani |
| `parser` | PS-006, PS-011 | Kani + proptest |
| `hostile-input` | PS-012 | Kani + proptest + cargo-fuzz |
| `performance` | PS-010 | Kani (waiver candidate) |
| `public-api` | PS-011, PS-020 | Kani + proptest |
| `gap` | PS-008, PS-020 | Kani + proptest |

---

## 5. Non-Applicable Lane Summary

| Lane | Seeds Affected | Evidence |
|------|---------------|----------|
| TLA+ | All 20 | No temporal behavior. Pure functional diagnostic code system. hazard-analysis.md §5: No concurrency hazards. |
| Verus | All 20 | Invariants are const-validated data consistency. Kani + proptest provide adequate coverage for P1 infrastructure. |
| Flux | All 20 | Simple newtypes with const lookup validation. No complex refinement types or typestate predicates. |
| Loom | All 20 | No concurrency primitives. CODE_REGISTRY is const static. All code() methods are pure. |
| Miri | All 20 | No unsafe code. forbid(unsafe_code) in all diagnostic modules. |

---

## 6. Defense-in-Depth Gaps

| Gap | Mitigation | Priority |
|-----|-----------|----------|
| Zero-allocation proof may require waiver | WVR-PS010-ALLOC if Kani alloc stubs infeasible | Low (non-behavior) |
| Fuzz target coverage limited to deserialization | Only PS-012 has hostile input surface; no other codec boundaries | Acceptable |
| Mutation testing may flag surviving mutants in deeply nested error enums | PO-027 targets diagnostic modules only | Low (monitoring) |
