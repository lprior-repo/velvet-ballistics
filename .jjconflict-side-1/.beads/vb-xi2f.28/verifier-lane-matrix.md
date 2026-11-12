# Verifier Lane Matrix — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28
**State:** 4 (proof-planner)
**Date:** 2026-05-25
**Status:** PLANNED

---

## Lane Assignment Matrix

Each row maps a proof seed to a per-verifier applicability decision.

Legend:
- **R** = Required (produces a proof obligation)
- **N/A** = Not Applicable (with evidence reference)
- **B** = Blocked (tooling unavailable; listed as non-behavior waiver)

| Proof Seed | Domain Claim | TLA+ | Verus | Kani | Flux | Loom | Miri | proptest | fuzz |
|---|---|---|---|---|---|---|---|---|---|
| PS-FE-01 | input change → digest change | N/A | N/A | **R** | N/A | N/A | N/A | **R** | N/A |
| PS-FE-02 | at_once change → digest change | N/A | N/A | **R** | N/A | N/A | N/A | **R** | N/A |
| PS-FE-03 | variable change → digest change | N/A | N/A | **R** | N/A | N/A | N/A | **R** | N/A |
| PS-FE-04 | body change → digest change | N/A | N/A | **R** | N/A | N/A | N/A | **R** | N/A |
| PS-FE-05 | determinism preserved | N/A | N/A | **R** | N/A | N/A | N/A | **R** | N/A |
| PS-FE-06 | dual-path equivalence | N/A | N/A | N/A | N/A | N/A | N/A | **R** | N/A |
| PS-FE-07 | None/Some(1) equivalence | N/A | N/A | **R** | N/A | N/A | N/A | N/A | N/A |
| PS-FE-08 | non-regression Set/Finish | N/A | N/A | N/A | N/A | N/A | N/A | **R** | N/A |
| PS-FE-09 | exhaustive field coverage | N/A | N/A | **R** | N/A | N/A | N/A | N/A | N/A |
| PS-FE-10 | delimiter collision resistance | N/A | N/A | **R** | N/A | N/A | N/A | N/A | N/A |

## Non-Applicable Evidence References

| Verifier | All Seeds | Evidence Reference |
|---|---|---|
| **tla-plus** | All | `boundary-map.md` §1, §5: `canonical_digest` is a pure function with no temporal state, no queues, no retries, no leases, no distributed protocols. The verifier-trigger-matrix requires TLA+ for "temporal workflow, queue ordering, retry, lease, lifecycle, cancellation, distributed protocol" — none of which apply. |
| **verus** | All | `workflow-model.md` §2: digest computation is an infallible pure pipeline. No deep arithmetic invariants (only field hashing), no complex typestate transitions. The properties are behavioral (equality, determinism), not type-theoretic. Bounded model checking (Kani) and property-based testing (proptest) are cheaper and sufficient. |
| **flux-rs** | All | `type-contracts.md` §4, `hazard-analysis.md` §4 HZ-I01: The fix uses Rust destructured match patterns (`ForEach { variable, input, at_once, body }`), which forces the compiler to check field exhaustiveness. Flux would need to model `blake3::Hasher` internal state to prove field absorption, which is infeasible. Kani covers the behavioral exhaustiveness proof. |
| **loom** | All | `hazard-analysis.md` §5 HZ-C01, HZ-C02: No threads, atomics, channels, locks, or async shutdown. `canonical_digest` creates a local `blake3::Hasher` and operates on immutable references. The verifier-trigger-matrix requires Loom for "threads, atomics, channels, locks, async shutdown, scheduler races" — none of which apply. |
| **miri** | All | `boundary-map.md` §4: No unsafe code, FFI, raw pointers, aliasing, provenance, or interior mutability. `blake3::Hasher` is pure safe Rust. `WorkflowDigest` is `#[repr(transparent)]` over `[u8; 32]` with no unsafe transmutes. The verifier-trigger-matrix requires Miri for "unsafe, FFI, raw pointers, aliasing, provenance, interior-mutability UB risk" — none of which apply. |
| **cargo-fuzz** | All | `domain-model.md` §5 DD-02: `canonical_digest` consumes a fully-parsed `WorkflowSource` AST, not raw bytes. There is no parser boundary at the digest computation level. Fuzzing the YAML parser is out of scope for this bead. Structured random input (proptest) covers adversarial input space by generating diverse AST values. |

## Lane Count Summary

| Status | Count |
|---|---|
| Required | 15 (8 Kani + 7 proptest) |
| Not Applicable | 64 (8 seeds × 8 verifiers - 16 required = 48... wait, let me recount) |

Accurate count: 10 seeds × 8 verifiers = 80 lane positions.
- Required: 15 (8 Kani + 7 proptest)
- Not Applicable: 65

| Verifier | Required | Not Applicable |
|---|---|---|
| tla-plus | 0 | 10 |
| verus | 0 | 10 |
| kani | 8 | 2 |
| flux-rs | 0 | 10 |
| loom | 0 | 10 |
| miri | 0 | 10 |
| proptest | 7 | 3 |
| cargo-fuzz | 0 | 10 |
| **Total** | **15** | **65** |
