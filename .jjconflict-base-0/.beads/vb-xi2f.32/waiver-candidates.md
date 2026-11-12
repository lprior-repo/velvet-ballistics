# Waiver Candidates: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** proof-planner (State 4)
**Schema:** waiver-candidates/v1

## Important: No Behavior-Affecting Waivers

Per the proof-planner operating rules, behavior-affecting waiver candidates are **invalid**. All proof seeds tagged `behavior_affecting: true` must be covered by proof obligations. The waiver candidates below cover only non-behavior, operational, or tooling-gap concerns.

---

## WC-001: Verus Proof of Digest Correctness

| Field | Value |
|-------|-------|
| **Candidate ID** | WC-001 |
| **Clause** | C1, C2, C3 (Wait digest correctness) |
| **Verifier** | Verus |
| **Reason** | P1 scope limitation. Verus would require rewriting both copies of `digest_step_primitive` and `canonical_digest` in Verus-subset Rust, then proving the full state-transformation. This is disproportionate for a pure-function fix where Kani + proptest + fuzz provide stronger direct coverage (collision detection, panic-freedom, broad input-space). |
| **Boundary proof** | PO-001 through PO-016 cover the real risks: panic-freedom (Kani), collision detection (proptest + fuzz + Kani), cross-path equivalence (proptest + Kani). |
| **Compensating evidence** | Kani bounded proofs + proptest wide-coverage + fuzz adversarial coverage. |
| **Behavior affecting?** | **No.** Verus is a proof method, not a behavior requirement. The behavior properties (digest sensitivity, determinism) are covered by other lanes. |
| **Owner** | Proof-plan reviewer |
| **Expiry** | When bead scope promotes to P0 or Verus Rust subset matures to handle blake3 Hasher natively |
| **Follow-up** | Consider Verus proof in a future bead when Verus-compatible blake3 binding exists and bead priority justifies the effort. |
| **Status** | `candidate` |

## WC-002: TLA+ Model of Digest Lifecycle

| Field | Value |
|-------|-------|
| **Candidate ID** | WC-002 |
| **Clause** | C4 (determinism), C5 (dual implementation consistency) |
| **Verifier** | TLA+ |
| **Reason** | Digest computation is a pure function with no temporal, retry, lease, queue, or interleaving semantics. TLA+ models temporal behavior over state spaces — there is no state space to model for a pure function. |
| **Boundary proof** | boundary-map.md sections 5, 7 confirm no temporal boundary. hazard-analysis.md CH-1, CH-2 rated NONE. workflow-model.md section 5 confirms pure-function concurrency model. |
| **Compensating evidence** | Determinism is covered by proptest PO-008, PO-014. Cross-path equivalence by PO-009, PO-010, PO-016. |
| **Behavior affecting?** | **No.** TLA+ is a verifier choice, not a behavior requirement. |
| **Owner** | Proof-plan reviewer |
| **Expiry** | Never — digest remains a pure function |
| **Follow-up** | None |
| **Status** | `candidate` |

## WC-003: Loom Model of Digest Computation

| Field | Value |
|-------|-------|
| **Candidate ID** | WC-003 |
| **Clause** | All |
| **Verifier** | Loom |
| **Reason** | Zero threads, atomics, channels, or concurrent interleavings in digest computation. `canonical_digest` is a pure function. Loom models concurrent schedule exploration — there are no schedules to explore. |
| **Boundary proof** | hazard-analysis.md CH-1, CH-2 rated NONE. boundary-map.md section 3 classifies digest as "Pure Core". |
| **Compensating evidence** | Not applicable — no concurrency risk exists. |
| **Behavior affecting?** | **No.** |
| **Owner** | Proof-plan reviewer |
| **Expiry** | Never — digest remains a pure function |
| **Follow-up** | None |
| **Status** | `candidate` |

## WC-004: Miri Check of Digest Path

| Field | Value |
|-------|-------|
| **Candidate ID** | WC-004 |
| **Clause** | All |
| **Verifier** | Miri |
| **Reason** | Zero `unsafe` code in the digest computation path. `#![forbid(unsafe_code)]` is enforced project-wide. Miri detects undefined behavior in unsafe code — with no unsafe code, there is nothing to detect. |
| **Boundary proof** | hazard-analysis.md UPH-1 rated NONE. invariants.yaml `no_unsafe_in_first_party` covers vb_compile. |
| **Compensating evidence** | `cargo miri test` pass on `vb_compile` is not planned because there is no unsafe to verify. If unsafe is ever added, Miri becomes required. |
| **Behavior affecting?** | **No.** |
| **Owner** | Proof-plan reviewer |
| **Expiry** | Until unsafe code is introduced in vb_compile |
| **Follow-up** | If unsafe is ever added to vb_compile, withdraw this waiver and add Miri obligations |
| **Status** | `candidate` |

## WC-005: Flux Refinement of Wait Shape

| Field | Value |
|-------|-------|
| **Candidate ID** | WC-005 |
| **Clause** | C1 (Wait field type safety) |
| **Verifier** | Flux |
| **Reason** | No refinement-type predicates exist in the digest path. The Wait shape validation (`validate_wait_shape`) is handled by pattern matching, not by refinement types. Flux would add annotations with no additional safety benefit. |
| **Boundary proof** | type-contracts.md section 7 identifies no refinement gaps. hazard-analysis.md RH-1 rated NONE. |
| **Compensating evidence** | Pattern matching in match arm ensures exhaustiveness. Validation gate ensures shape correctness. |
| **Behavior affecting?** | **No.** |
| **Owner** | Proof-plan reviewer |
| **Expiry** | Never — no refinement predicates needed |
| **Follow-up** | None |
| **Status** | `candidate` |

---

## Summary

| Candidate | Verifier | Behavior Affecting | Status |
|-----------|----------|--------------------|--------|
| WC-001 | Verus | No | candidate |
| WC-002 | TLA+ | No | candidate |
| WC-003 | Loom | No | candidate |
| WC-004 | Miri | No | candidate |
| WC-005 | Flux | No | candidate |

**Zero behavior-affecting waivers proposed.** All behavior requirements (C1-C6) are covered by Kani, proptest, and cargo-fuzz obligations (PO-001 through PO-016).
