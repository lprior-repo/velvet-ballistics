# Waiver Candidates — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28
**State:** 4 (proof-planner)
**Date:** 2026-05-25
**Status:** PLANNED

---

## Candidate Summary

This bead has **one** non-behavior waiver candidate. No behavior-affecting waivers are proposed.

## WC-FE-01: Kani Tool Availability

| Field | Value |
|---|---|
| **ID** | WC-FE-01 |
| **Requirement** | All Kani obligations (PO-K-FE-01 through PO-K-FE-10) |
| **Clause** | N/A (tooling constraint) |
| **Reason** | `cargo kani` is not currently available in the runtime environment. Tool availability check at start of State 4 showed `kani not found`. While Kani can be installed via `cargo install --locked kani-verifier`, the CI environment may not support Kani's heavyweight model-checking (requires CBMC, SAT solver). |
| **Behavior-Affecting** | `false` |
| **Boundary Proof** | If Kani is unavailable, the 8 Kani obligations must be downgraded to `blocked_tooling` status with compensating evidence from proptest obligations. The proptest obligations (PO-P-FE-01 through PO-P-FE-08) provide statistical coverage for the same behavior claims. Kani provides exhaustive bounded proof; proptest provides statistical confidence. Together they form defense-in-depth; individually, proptest is the minimum viable coverage. |
| **Compensating Evidence** | (1) Proptest obligations PO-P-FE-01 through PO-P-FE-08 cover all P0 behavior claims statistically. (2) Rust compiler destructuring provides compile-time field exhaustiveness (INV-FE-01). (3) Existing determinism tests in `error_variant_tests.rs` provide regression baseline. (4) CI can still gate on `cargo test` and `moon ci` without Kani. |
| **Owner** | proof-planner |
| **Expiry** | 2026-06-25 (30-day timeout; Kani installation should be attempted during that window) |
| **Review Status** | `pending` |

## No Behavior-Affecting Waivers

All behavior-affecting contract clauses (AC-FE-01 through AC-FE-08, INV-FE-01, INV-FE-02) are fully covered by at least one required obligation (Kani or proptest). No behavior-affecting waivers are proposed.

## Out-of-Scope Primitives (Not Waivers)

The digest gaps for other primitives (Collect, Aggregate, Repeat, Together, Wait, Ask, Choose, Do, Save) are explicitly out of scope per DD-01. These are NOT waivers — they are deferred to future beads. The risk is documented but accepted at the bead scope level, not waived at the verification level.
