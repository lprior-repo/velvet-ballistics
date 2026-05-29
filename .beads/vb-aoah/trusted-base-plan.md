# Trusted Base Plan (Reduced Scope) — vb-aoah

## Planned trusted surfaces and reductions

| Surface | Applies to | Planned treatment | Behavior-affecting | Closure requirement |
|---|---|---|---|---|
| Fjall persistence semantics | Kani/proptest/fuzz obligations | Treat Fjall read/write/delete/flush API behavior as external dependency; verify our call ordering and typed outcomes around it. | true | State 12 ledger must cite integration/proptest/fuzz evidence around actual Fjall operations. |
| Postcard/envelope codec | Kani/proptest/fuzz obligations | Trust existing codec implementation only at public boundary; hostile bytes must be covered by fuzz when migration parses records (seeds 001, 004, 006, 007). | true | Fuzz obligations PO-R15–PO-R18 must execute against production codec paths. |
| Bounded model constants | Kani/proptest/fuzz | Bound versions to u16, records/bytes to explicit named maxima (u64), and include overflow/error transitions. | true | Bounds documented in every obligation row; no unbounded Nat assumptions. Kani uses kani::Arbitrary per GOD RULE. |
| Kani harness shape policy | All Kani obligations (PO-R01–PO-R07) | Use `kani::Arbitrary` or bounded generators; never hardcoded storage shapes. | true | Proof-reviewer (State 6) rejects hardcoded harness shapes. |
| Performance benchmark omission | Non-behavior evidence scope | No benchmark proof lane because no speed claim is made and migration is cold-path. | false | Waiver candidate WC-001 requires reviewer disposition and expires 2026-08-31. |

## Excluded verifier trust

| Surface | Treatment | Reason |
|---|---|---|
| TLA+ models | Not planned | Test-first bead; no production temporal behavior to model |
| Verus specs | Not planned | Test-first bead; no production Rust to bind specs to (GOD RULE) |
| Flux refinements | Not planned | Test-first bead; no refinement type-level enforcement needed |
| Loom models | Not planned | No concurrency scope (boundary-map.md, hazard-analysis.md) |
| Miri checks | Not planned | No unsafe/FFI/raw-pointer scope (boundary-map.md, hazard-analysis.md) |

## Planner stance

Pending trust is not approval. State 5/6/12 must either remove, ledger, or formally waive non-behavior trust. Behavior-affecting waivers are forbidden. WC-001 (non-behavior performance evidence) is the only waiver candidate; it is pending reviewer disposition and expires 2026-08-31.

This plan is identical in substance to the prior trusted-base-plan.md; only lane references are updated for the reduced scope (TLA+/Verus/Flux excluded lanes added, Kani/Arbitrary constraint made explicit).
