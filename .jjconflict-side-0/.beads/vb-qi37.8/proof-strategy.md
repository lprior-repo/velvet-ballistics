# Proof Strategy: vb-qi37.8 — Shared Validation Pipeline

## Overview

This proof strategy maps 36 proof obligations across 9 validation gates to appropriate
verifier lanes based on risk tags, obligation type, and contract clause criticality.

## Risk Tag Mapping

| Risk Level | Gates | Obligation Count | Primary Verifier | Fallback |
|------------|-------|------------------|------------------|----------|
| LOW | G7, G8, G9 | 10 | Kani bounded | Miri UB |
| MEDIUM | G10, G11, G12, G13, G14 | 17 | Kani + Miri dual | Proptest |
| HIGH | G15 | 4 | Kani + TLA+ + Lean | Proptest |
| PIPELINE | Pipeline | 3 | Kani + Proptest | Miri |
| INTEGRATION | Integration | 6 | Integration tests | Fuzz |

## Gate-by-Gate Strategy

### G7 — Expression Stack Depth (2 obligations)
- PO-001: Kani bounded model checking, induction on expression tree depth
- PO-002: Miri UB check, overflow detection
- **Verifier lane**: Kani (primary) + Miri (UB validation)
- **Risk tag**: LOW — bounded by max 64

### G8 — Accessor Path Segments (2 obligations)
- PO-003: Kani bounded traversal, symbols_count bound
- PO-004: Miri UB check, symbol resolution
- **Verifier lane**: Kani (primary) + Miri (UB validation)
- **Risk tag**: LOW — bounded traversal

### G9 — Slot References (3 obligations)
- PO-005: Kani bounded, slot_count bound
- PO-006: Kani bounded, slot_count bound
- PO-007: Miri UB check, slot index operations
- **Verifier lane**: Kani (primary) + Miri (UB validation)
- **Risk tag**: LOW — u16 bounded

### G10 — Node Kind Structural (5 obligations)
- PO-008: Kani bounded, ForEachStart→ForEachJoin matching
- PO-009: Kani bounded, TogetherStart→TogetherJoin matching
- PO-010: Kani bounded, ReduceStart→ReduceFinish matching
- PO-011: Kani bounded, CollectStart→CollectFinish matching
- PO-012: Miri UB check, kind matching
- **Verifier lane**: Kani (structural) + Miri (UB validation)
- **Risk tag**: MEDIUM — graph traversal complexity

### G11 — Loop Body Graph (3 obligations)
- PO-013: Kani bounded, ForEach body graph traversal
- PO-014: Kani bounded, Together body graph traversal
- PO-015: Miri UB check, graph operations
- **Verifier lane**: Kani (structural) + Miri (UB validation)
- **Risk tag**: MEDIUM — recursive graph traversal

### G12 — Action Contract Bijection (3 obligations)
- PO-016: Kani bounded, Do→ActionContract surjection
- PO-017: Kani bounded, ActionContract→Do injection
- PO-018: Proptest property testing, 1000 iterations
- **Verifier lane**: Kani (existence) + Proptest (bijection property)
- **Risk tag**: MEDIUM — cardinality relationship

### G13 — Slot Cycle Detection (3 obligations)
- PO-019: Kani bounded, slot_count iterations, cycle detection
- PO-020: TLA+ model checking, G13_NoCycle invariant
- PO-021: Miri UB check, cycle detection operations
- **Verifier lane**: Kani (bounded) + TLA+ (temporal) + Miri (UB)
- **Risk tag**: MEDIUM — algorithm termination

### G14 — Slot Type Consistency (2 obligations)
- PO-022: Kani bounded, pairwise type compatibility
- PO-023: Miri UB check, type checks
- **Verifier lane**: Kani (primary) + Miri (UB validation)
- **Risk tag**: MEDIUM — type system complexity

### G15 — Determinism Proof (4 obligations)
- PO-024: Kani bounded, suspension point separation
- PO-025: TLA+ model checking, G15_Separated temporal invariant
- PO-026: Lean theorem proving, NDNodesSeparated theorem
- PO-027: Miri UB check, graph operations
- **Verifier lane**: Kani + TLA+ + Lean + Miri (full stack)
- **Risk tag**: HIGH — temporal property + theorem proving

### Pipeline (3 obligations)
- PO-028: Proptest property testing, determinism
- PO-029: Miri UB check, no side effects
- PO-030: Kani bounded, composition soundness
- **Verifier lane**: Kani + Proptest + Miri
- **Risk tag**: MEDIUM — pipeline composition

### Integration (6 obligations)
- PO-031 to PO-036: Integration tests + fuzz
- **Verifier lane**: Integration tests + Fuzz
- **Risk tag**: LOW — call site verification

## Execution Order (Cheapest First)

1. **Miri** (UB validation, fast fail) — PO-002, PO-004, PO-007, PO-012, PO-015, PO-021, PO-023, PO-027, PO-029
2. **Proptest** (property testing) — PO-018, PO-028
3. **Kani** (bounded model checking) — PO-001, PO-003, PO-005, PO-006, PO-008, PO-009, PO-010, PO-011, PO-013, PO-014, PO-016, PO-017, PO-019, PO-022, PO-024, PO-030
4. **TLA+** (temporal model checking) — PO-020, PO-025
5. **Lean** (theorem proving) — PO-026
6. **Integration** (tests) — PO-031 to PO-036
7. **Fuzz** (continuous) — PO-036

## Verifier Configuration

| Verifier | Bound/Config | Timeout |
|----------|--------------|---------|
| Miri | --relaxed | 10min per PO |
| Kani | --unwind 64 | 15min per PO |
| Proptest | 1000 iterations | 5min per PO |
| TLA+ | TLC | 10min per spec |
| Lean | Theorem kernel | 20min per theorem |
| Integration | cargo test | 30min total |
| Fuzz | cargo fuzz run | continuous |

## Acceptance Criteria Mapping

| AC ID | Proof Obligation | Verifier Lane |
|-------|-----------------|---------------|
| AC1 | All gate unit tests | cargo test |
| AC2 | G7-G15 malformed input rejection | Kani + Miri |
| AC3 | G12 bijection | Kani + Proptest |
| AC6 | Pipeline determinism | Proptest |
| AC7 | No panic on any input | Miri + Kani |

## Evidence Recording

All proof runs record:
- PASS/FAIL_LOCAL/FAIL_REGRESSION/WAIVED/DEFERRED_GLOBAL per obligation
- Global debt tracked as ratchet (never decreases)
- Evidence artifacts stored in .beads/vb-qi37.8/evidence/
