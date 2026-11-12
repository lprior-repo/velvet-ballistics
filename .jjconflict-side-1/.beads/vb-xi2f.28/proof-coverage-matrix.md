# Proof Coverage Matrix — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28
**State:** 4 (proof-planner)
**Date:** 2026-05-25
**Status:** PLANNED

---

## Coverage Mapping: Contract Clause → Obligation → Verifier

| Contract Clause | Domain Claim | Proof Seed | Kani Obligation | Proptest Obligation | Coverage |
|---|---|---|---|---|---|
| AC-FE-01 | ForEach.input change → digest change | PS-FE-01 | PO-K-FE-01 | PO-P-FE-01 | Full |
| AC-FE-02 | ForEach.at_once change → digest change | PS-FE-02 | PO-K-FE-02 | PO-P-FE-02 | Full |
| AC-FE-03 | ForEach.variable change → digest change | PS-FE-03 | PO-K-FE-03 | PO-P-FE-03 | Full |
| AC-FE-04 | ForEach.body change → digest change | PS-FE-04 | PO-K-FE-04 | PO-P-FE-04 | Full |
| AC-FE-05 | Determinism preserved | PS-FE-05 | PO-K-FE-05 | PO-P-FE-05 | Full |
| AC-FE-06 | Dual-path equivalence | PS-FE-06 | — (N/A) | PO-P-FE-06 | Full |
| AC-FE-07 | None/Some(1) equivalence | PS-FE-07 | PO-K-FE-07 | — (redundant) | Full |
| AC-FE-08 | Non-regression Set/Finish | PS-FE-08 | — (N/A) | PO-P-FE-08 | Full |
| INV-FE-01 | Exhaustive field coverage | PS-FE-09 | PO-K-FE-09 | — (redundant) | Full |
| INV-FE-02 | Delimiter collision resistance | PS-FE-10 | PO-K-FE-10 | — (redundant) | Full |

## Defense-in-Depth Layers

| Layer | Role | Obligations | Coverage |
|---|---|---|---|
| **Kani** (bounded proof) | Prove field inclusion, determinism, semantic equivalence, delimiter safety | 8 | All P0/P1 behavior claims; bounded proof of field hashing |
| **proptest** (statistical) | Validate sensitivity across broad random input space, dual-path equivalence | 7 | All P0 behavior claims; broad input coverage; cross-path regression |
| **Rust compiler** (compile-time) | Enforce field exhaustiveness via destructuring match | — (language guarantee) | INV-FE-01 (field exhaustiveness) |

## Coverage Gap Analysis

| Contract Clause | Covered? | Gap |
|---|---|---|
| AC-FE-01 | Yes | Kani + proptest |
| AC-FE-02 | Yes | Kani + proptest |
| AC-FE-03 | Yes | Kani + proptest |
| AC-FE-04 | Yes | Kani + proptest |
| AC-FE-05 | Yes | Kani + proptest |
| AC-FE-06 | Yes | Proptest only (Kani N/A for cross-path) |
| AC-FE-07 | Yes | Kani only (specific bounded claim) |
| AC-FE-08 | Yes | Proptest only (regression across versions) |
| INV-FE-01 | Yes | Kani (behavioral) + Rust compiler (structural) |
| INV-FE-02 | Yes | Kani only (bounded byte-level proof) |

**All 10 contract clauses are covered with at least one proof obligation. No coverage gaps.**

## Out-of-Scope Clause Coverage

| Out-of-Scope Primitive | Current Digest Gap | Bead Coverage |
|---|---|---|
| Collect | Name only | N/A (out of scope) |
| Aggregate (reduce) | Name only | N/A (out of scope) |
| Repeat | Name only | N/A (out of scope) |
| Together (parallel) | Name only | N/A (out of scope) |
| Wait | Name only | N/A (out of scope) |
| Ask | Name only | N/A (out of scope) |
| Choose | Name only | N/A (out of scope) |
| Do | Name only | N/A (out of scope) |
| Save | Name only | N/A (out of scope) |

These gaps exist but are explicitly out of scope per DD-01. They remain representable after this bead's fix. Future beads should address them using the same pattern established for ForEach.
