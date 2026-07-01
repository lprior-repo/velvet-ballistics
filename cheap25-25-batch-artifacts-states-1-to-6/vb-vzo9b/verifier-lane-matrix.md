# Verifier Lane Matrix: vb-vzo9b

## Lane Applicability Matrix

| Proof Seed | contract_clause | verus | kani | flux-rs | proptest (cargo-test) | loom | miri | cargo-fuzz |
|------------|-----------------|-------|------|---------|----------------------|------|------|------------|
| PS-vb-vzo9b-1 (Exactness of pin) | C-1 | not_applicable | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable |
| PS-vb-vzo9b-2 (Sentinel rejection) | C-2 | not_applicable | not_applicable | not_applicable | required (covered by PO-001) | not_applicable | not_applicable | not_applicable |
| PS-vb-vzo9b-3 (Empty-events path unchanged) | C-3 | not_applicable | not_applicable | not_applicable | required (covered by PO-001) | not_applicable | not_applicable | not_applicable |
| PS-vb-vzo9b-4 (No production change / source-lint) | C-5/C-6 | not_applicable | not_applicable | not_applicable | required (covered by PO-003) | not_applicable | not_applicable | not_applicable |
| PS-vb-vzo9b-5 (Forbidden patterns) | C-8 | not_applicable | not_applicable | not_applicable | required (covered by PO-003) | not_applicable | not_applicable | not_applicable |
| PS-vb-vzo9b-6 (Frame-seed call unchanged) | C-4 | not_applicable | not_applicable | not_applicable | required (covered by PO-002) | not_applicable | not_applicable | not_applicable |

## Lane Decision Summary

| Lane Decision ID | Verifier | Applicability | Bound Proof Obligation |
|------------------|----------|---------------|------------------------|
| VLD-001 | proptest (cargo-test) | required | PO-001 (summarize_recovery_events unit-test gate) |
| VLD-002 | proptest (cargo-test) | required | PO-002 (recover_runtime_frame_seed_from_events unit-test gate) |
| VLD-003 | proptest (cargo-build + source-lint) | required | PO-003 (fuzz binary compile + forbidden-pattern grep) |
| VLD-004 | verus | not_applicable | — |
| VLD-005 | kani | not_applicable | — |
| VLD-006 | flux-rs | not_applicable | — |
| VLD-007 | loom | not_applicable | — |
| VLD-008 | miri | not_applicable | — |
| VLD-009 | cargo-fuzz | not_applicable | — |

## Applicability Legend

- **required**: Mandatory verifier lane per the bead's restricted profile (`cargo-test`, `source-lint`); 2-3 obligations planned.
- **not_applicable**: Default-profile verifier provably does not apply to this test-only repair; concrete evidence provided in `verifier-lane-decisions.jsonl` `non_applicability_evidence_refs` (SHA-256 hashes of `contract.md`, `codebase-map.md`, `delivery-scope.jsonl`, and the fuzz harness source).
- **blocked_tooling**: Tool is unavailable; blocks proof closure. (None in this bead.)

## Non-Applicability Evidence Summary

| Lane | Limitation Kind | Reason | Evidence Refs |
|------|-----------------|--------|---------------|
| verus (all seeds) | `surface_absent` | The defect is in test code (`fuzz/src/journal_target/readback.rs:196`), not production. The fix replaces a `assert!(... \|\| ...)` with `assert_eq!` over an existing `RecoveryRuntimeSummary` that already derives `Debug, Clone, Copy, PartialEq, Eq` at `crates/vb_storage/src/recovery/types.rs:546`. No new Rust-local invariant to model; production behavior is unchanged. | `contract.md` SHA-256: `3e759af7624f332b6b3298e9a93de95bfd206422d2b820f804bfbb5a11cca5eb` (C-5: production read-only); `proof-seeds.jsonl` SHA-256: `346da60c2f2b4f078b70a3296d5493a2fbe552ba060ce3b48a076d1fa3fe6434` (PS-vb-vzo9b-1 notes). |
| kani (all seeds) | `surface_absent` | No new bounded symbolic claim. The fuzz payload shape (single `RunAccepted` event with `seq = EventSeq::new(1)`) has a single deterministic production path; the new `assert_eq!` is over a Copy + PartialEq + Eq struct and is exhaustive in the field set. | `proof-seeds.jsonl` SHA-256: `346da60c2f2b4f078b70a3296d5493a2fbe552ba060ce3b48a076d1fa3fe6434` (PS-vb-vzo9b-1: "cargo-test exact-pin is sufficient. No Verus/Kani/Flux refinement needed"). |
| flux-rs (all seeds) | `surface_absent` | No new refinement type introduction. `RecoveryRuntimeSummary` is unchanged; the fix is a plain `assert_eq!` over its existing fields. | `delivery-scope.jsonl` SHA-256: `92fa5762283d237fe8bfbb4e942ae9f55a4988df9710417d8b7ac9daecfad432` (row 30: `flux` required=false). |
| loom (all seeds) | `surface_absent` | No concurrency, atomics, channels, locks, async shutdown, or task ownership. The fuzz harness is a synchronous function reading stdin bytes; no threads, no `Send`/`Sync` boundary. | `fuzz/src/journal_target/readback.rs` SHA-256: `5b08c5c76662b28306416609b1b57c05a1044281d322e50f78633e41e8727423` (no `tokio`, no `crossbeam`, no `std::sync::*` in scope). |
| miri (all seeds) | `surface_absent` | No `unsafe` blocks, no FFI, no raw pointers, no MaybeUninit, no provenance-sensitive operations. The fuzz harness and the production recovery surface it calls all use safe Rust. | `fuzz/src/journal_target/readback.rs` SHA-256: `5b08c5c76662b28306416609b1b57c05a1044281d322e50f78633e41e8727423` (zero `unsafe`); `crates/vb_storage/src/recovery/types.rs` SHA-256: `ca189eebcfee4797a02524899dca76a94a09a219662e55d1c9b213c2f73f9d85` (zero `unsafe` in the `RecoveryRuntimeSummary` derive block). |
| cargo-fuzz (all seeds) | `superseded_by_other_lane_with_evidence` | The fuzz harness body is the target of the change. The repair tightens assertions inside the existing fuzz harness rather than introducing a new fuzz target. The closure gates are `cargo build -p fuzz --bin recovery_decode` (compile) + the two `cargo test` invocations (unit-test surface). A separate `cargo fuzz run` is not in the contract closure commands. | `contract.md` SHA-256: `3e759af7624f332b6b3298e9a93de95bfd206422d2b820f804bfbb5a11cca5eb` (C-7 closure commands); `delivery-scope.jsonl` SHA-256: `92fa5762283d237fe8bfbb4e942ae9f55a4988df9710417d8b7ac9daecfad432` (verifier_mode rows). |

## Cross-Lane Discipline

- `verus`, `kani`, `flux-rs` all surface-absent on the same root cause: production is unchanged and the new assertion is over an existing struct with no new invariant. Each row cites distinct evidence (PS-vb-vzo9b-1, delivery-scope row 30, contract C-5).
- `loom`, `miri` are surface-absent on independent concerns (no concurrency, no unsafe).
- `cargo-fuzz` is `superseded_by_other_lane_with_evidence` because the fuzz harness itself is the test target; the cargo-build lane (PO-003) covers the build/compile path that cargo-fuzz would otherwise exercise via libfuzzer's pre-build step.

No default-profile verifier is required; no verifier is silently omitted.

## Self-Audit Checklist

- [x] Every (requirement_id, contract_clause, proof_seed_id, verifier) tuple in the default profile has exactly one lane decision.
- [x] No default-profile verifier has `not_applicable` without `non_applicability_evidence_refs` containing at least one SHA-256 hash.
- [x] Every `required` lane decision has at least one paired `proof-obligation/v1` ID, and the obligation exists in `proof-obligations.planned.jsonl`.
- [x] No `blocked_tooling` rows.
- [x] All `decision_reason` strings cite concrete `risk_tags` and avoid the weak vocabulary.
- [x] All `not_applicable` rows have a typed `limitation_kind`.
- [x] No two rows duplicate `(requirement_id, contract_clause, proof_seed_id, verifier)` with conflicting `applicability`.