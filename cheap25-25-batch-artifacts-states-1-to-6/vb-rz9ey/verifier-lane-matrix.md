# Verifier Lane Matrix — vb-rz9ey

- bead_id: vb-rz9ey
- state: 4 (proof-planner)
- contract_sha256: e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66
- codebase_map_sha256: 7336795bdf60f345ae7d2af2641b16388e36fc79d27e653cf00db31affd66697
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
- authored_by: proof-planner

This matrix enumerates every (proof-seed, verifier) tuple required by the bead's
risk profile, plus every default-profile verifier for which this bead has no
risk. Rows marked `not_applicable` carry a typed `limitation_kind` and concrete
`non_applicability_evidence_refs` per
`/home/lewis/.opencode/skill/proof-planner/references/lane-decision-guide.md`.

## 1. Profile Inventory

| Lane | In default profile for this bead? | Reason |
|------|----------------------------------|--------|
| verus | NO | No Verus spec exists; verification/verus/ is absent for vb_compile. |
| kani | NO | No Kani harness executes against WorkflowSourceParts in this bead's scope. Kani harnesses in `src/kani_digest_ask_*.rs` are pre-existing and out-of-scope (OI-1). |
| flux-rs | NO | No Flux refinement targets WorkflowSourceParts; verification/flux/ is absent for vb_compile. |
| loom | NO | No concurrency surface; the change is a manifest edit, no threads/Send/Sync. |
| miri | NO | vb_compile sets no `#![forbid(unsafe_code)]` override; the bead does not touch unsafe. |
| cargo-fuzz | NO | No parser/codec hostile-input boundary in this bead. |
| proptest | YES | The test build IS the evidence surface; proptest harnesses in `tests/proptest_*` are compiled as part of `cargo build -p vb_compile --tests`. |

The lane "source-lint" (`moon run :lint-src`) is a governance gate, not a
formal-verifier lane, and is therefore not modeled as a
`verifier-lane-decision/v1` row. It is, however, listed as a mandatory
sub-evidence in the obligation `expected_evidence` text.

## 2. Lane Decisions Per Obligation

### PO-001 — Manifest obligation (PS-001)

| verifier | applicability | decision_reason | limitation_kind | paired_obligation |
|----------|---------------|-----------------|------------------|--------------------|
| proptest | required | `risk:build` + `risk:public_api` + `risk:test_only`: the test build must compile after the dev-dep activates `test-util`. Cargo's test build invokes the proptest harnesses in `tests/proptest_digest_foreach.rs`, `tests/proptest_digest_determinism.rs`, `tests/proptest_digest_ask_*.rs` as part of the same compilation unit. The proptest category is the closest schema-aligned verifier whose evidence surface (cargo test invocation) matches the actual evidence command. | n/a | PO-001 |
| verus | not_applicable | No Verus spec references `WorkflowSourceParts`; `verification/verus/` is absent for vb_compile. The visibility invariant is enforced statically by rustc, not by a proof obligation. | surface_absent | n/a |
| kani | not_applicable | The 6 Kani harnesses at `src/kani_digest_ask_*.rs` and `src/kani_digest_step_primitive_no_panic.rs` import `WorkflowSource` from `crate::ast` (line marked OI-1 latent defect in `codebase-map.md` Q1). They are gated by `#[cfg(all(kani, any(test, feature = "test-util")))]` and do NOT participate in `cargo build --tests`. This bead does not exercise Kani. | surface_absent | n/a |
| flux-rs | not_applicable | No Flux refinement targets `WorkflowSourceParts`; `verification/flux/` is absent for vb_compile. The visibility invariant has no refinement shape. | surface_absent | n/a |
| loom | not_applicable | The Cargo manifest change introduces no concurrency surface. There are no `Send`/`Sync` boundaries, channels, or threads touched by this bead. `boundary-map.md` SHA-256 confirms no async/thread surface for vb_compile. | surface_absent | n/a |
| miri | not_applicable | The bead is a manifest edit, not an unsafe-code change. vb_compile does not export FFI. The fix does not introduce raw pointers, `MaybeUninit`, or any unsafe primitive. `boundary-map.md` SHA-256 confirms no FFI boundary. | surface_absent | n/a |
| cargo-fuzz | not_applicable | No parser/codec hostile-input boundary in this bead. The fix is a build-time manifest change; there is no byte-level external input surface affected. | surface_absent | n/a |

### PO-002 — Downstream preservation obligation (PS-002)

| verifier | applicability | decision_reason | limitation_kind | paired_obligation |
|----------|---------------|-----------------|------------------|--------------------|
| proptest | required | `risk:public_api` + `risk:downstream`: the dev-dep must NOT propagate `test-util` into downstream production builds. The cargo build of vb_cli and workspace_tests is the property under test — cargo's per-build-graph feature unification is verified by the build succeeding with the feature NOT visible to those crates. The proptest category is the closest schema-aligned verifier whose evidence surface (cargo build invocation) matches the actual evidence command. | n/a | PO-002 |
| verus | not_applicable | No Verus spec references the downstream-build-graph property. The visibility invariant is enforced by rustc and by cargo's feature unification; no formal proof obligation applies. | surface_absent | n/a |
| kani | not_applicable | No Kani harness targets vb_cli or workspace_tests in this bead's scope. The downstream build-graph property is verified by `cargo build` exit code, not by symbolic execution. | surface_absent | n/a |
| flux-rs | not_applicable | No Flux refinement targets the feature-unification property. The build-graph property is a static cargo property, not a refinement type. | surface_absent | n/a |
| loom | not_applicable | The downstream builds introduce no concurrency surface beyond what already exists in vb_cli and workspace_tests; this bead does not touch that surface. | surface_absent | n/a |
| miri | not_applicable | vb_cli and workspace_tests do not introduce unsafe as part of this bead; Miri does not model feature unification. | surface_absent | n/a |
| cargo-fuzz | not_applicable | The downstream builds are not parser/codec surfaces; cargo-fuzz is irrelevant for verifying feature isolation. | surface_absent | n/a |

## 3. Counting

- Total `verifier-lane-decision/v1` rows: 14 (7 verifiers × 2 obligations).
- Required rows: 2 (1 per obligation; proptest).
- Not_applicable rows: 12 (6 per obligation × 2 obligations).
- Blocked_tooling rows: 0.
- Waivers: 0.

## 4. Pairing Invariants (per lane-decision-guide.md §"Self-Audit Checklist")

- Every `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple
  appears exactly once across the 14 rows. No duplicates.
- Every `required` row's `required_obligation_ids` names PO-001 or PO-002,
  which exist in `proof-obligations.planned.jsonl`.
- Every `not_applicable` row's `non_applicability_evidence_refs` lists at
  least one SHA-256 hash from `contract.md`, `codebase-map.md`,
  `proof-seeds.jsonl`, `traceability-matrix.jsonl`, or
  `delivery-scope.jsonl`.
- Every `not_applicable` row carries a typed `limitation_kind` (only
  `surface_absent` is used here).
- No row uses weak vocabulary ("not needed", "too hard", "covered by other
  lane", "low risk", "we'll add this later").

## 5. Cross-Reference

- See `proof-obligations.planned.jsonl` for the paired obligation definitions.
- See `proof-coverage-matrix.md` for the (requirement, contract clause,
  proof-seed, obligation) mapping.
- See `proof-strategy.md` §3 for the risk-tag inventory that drives this
  matrix.
- See `proof-to-implementation-input.md` for the bridge to the
  `holzman-rust` and `black-hat-reviewer` lanes.