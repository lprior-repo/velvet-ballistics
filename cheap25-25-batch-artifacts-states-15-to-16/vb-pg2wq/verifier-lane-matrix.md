# Verifier Lane Matrix — vb-pg2wq duplicate-event test exact-contract repair

STATUS: PLANNED. No verifier, test, fuzz, CI, or proof success is claimed here.

## Lane universe (default profile)

| Verifier | Role in default profile | Trigger tags |
|----------|------------------------|--------------|
| `verus` | Rust-local pure/core invariant | `rust_local`, `pure_core`, `arithmetic` |
| `kani` | Bounded state machine / rejection / panic-freedom | `bounded_state`, `rejection`, `panic_freedom` |
| `flux-rs` | Refinement / index / ownership | `refinement`, `index`, `ownership` |
| `loom` | Concurrency / interleaving / cancellation / shutdown | `concurrency`, `interleaving`, `cancellation`, `shutdown`, `channel`, `lock`, `task_ownership` |
| `miri` | UB / unsafe / FFI / raw_pointer / provenance | `ub`, `unsafe`, `ffi`, `raw_pointer`, `provenance`, `aliasing`, `layout` |
| `cargo-fuzz` | Parser / codec / hostile input | `parser`, `codec`, `hostile_input`, `persisted_bytes`, `ipc_decode`, `fuzzable_canonicalization` |
| `proptest` | Property / equality / ordering / field_sensitivity | `property`, `field_sensitivity` |

## Seed × verifier matrix

The full `verifier-lane-decisions.jsonl` is 56 rows = 8 proof seeds × 7 default-profile verifiers. The matrix below is the rolled-up view per seed.

### vb-pg2wq-seed-ps001 (`ps001_duplicate_rejected`)

| Verifier | Applicability | Reason kind | Required obligation IDs |
|----------|---------------|-------------|--------------------------|
| `proptest` | required | audit-regression-resistance + field_sensitivity | `PO-vb-pg2wq-001` |
| `verus` | not_applicable | no-production-bound-seam | — |
| `kani` | not_applicable | superseded_by_other_lane_with_evidence | — |
| `flux-rs` | not_applicable | trigger-not-present | — |
| `loom` | not_applicable | trigger-not-present | — |
| `miri` | not_applicable | trigger-not-present | — |
| `cargo-fuzz` | not_applicable | trigger-not-present | — |

### vb-pg2wq-seed-ps003 (`ps003_dup_fields`)

| Verifier | Applicability | Reason kind | Required obligation IDs |
|----------|---------------|-------------|--------------------------|
| `proptest` | required | audit-regression-resistance + field_sensitivity | `PO-vb-pg2wq-001` |
| `verus` | not_applicable | no-production-bound-seam | — |
| `kani` | not_applicable | superseded_by_other_lane_with_evidence | — |
| `flux-rs` | not_applicable | trigger-not-present | — |
| `loom` | not_applicable | trigger-not-present | — |
| `miri` | not_applicable | trigger-not-present | — |
| `cargo-fuzz` | not_applicable | trigger-not-present | — |

### vb-pg2wq-seed-ps004a (`ps004_no_persist`)

| Verifier | Applicability | Reason kind | Required obligation IDs |
|----------|---------------|-------------|--------------------------|
| `proptest` | required | audit-regression-resistance + secondary-invariant-preservation + field_sensitivity | `PO-vb-pg2wq-002` |
| `verus` | not_applicable | no-production-bound-seam | — |
| `kani` | not_applicable | superseded_by_other_lane_with_evidence | — |
| `flux-rs` | not_applicable | trigger-not-present | — |
| `loom` | not_applicable | trigger-not-present | — |
| `miri` | not_applicable | trigger-not-present | — |
| `cargo-fuzz` | not_applicable | trigger-not-present | — |

### vb-pg2wq-seed-ps004b (`ps004_empty_commit_after_rej`)

| Verifier | Applicability | Reason kind | Required obligation IDs |
|----------|---------------|-------------|--------------------------|
| `proptest` | required | audit-regression-resistance + secondary-invariant-preservation + field_sensitivity | `PO-vb-pg2wq-002` |
| `verus` | not_applicable | no-production-bound-seam | — |
| `kani` | not_applicable | superseded_by_other_lane_with_evidence | — |
| `flux-rs` | not_applicable | trigger-not-present | — |
| `loom` | not_applicable | trigger-not-present | — |
| `miri` | not_applicable | trigger-not-present | — |
| `cargo-fuzz` | not_applicable | trigger-not-present | — |

### vb-pg2wq-seed-ps008 (`ps008_dup_before_queue`)

| Verifier | Applicability | Reason kind | Required obligation IDs |
|----------|---------------|-------------|--------------------------|
| `proptest` | required | audit-regression-resistance + field_sensitivity | `PO-vb-pg2wq-001` |
| `verus` | not_applicable | no-production-bound-seam | — |
| `kani` | not_applicable | superseded_by_other_lane_with_evidence | — |
| `flux-rs` | not_applicable | trigger-not-present | — |
| `loom` | not_applicable | trigger-not-present | — |
| `miri` | not_applicable | trigger-not-present | — |
| `cargo-fuzz` | not_applicable | trigger-not-present | — |

### vb-pg2wq-seed-ps009 (`ps009_dup_rejected`)

| Verifier | Applicability | Reason kind | Required obligation IDs |
|----------|---------------|-------------|--------------------------|
| `proptest` | required | audit-regression-resistance + field_sensitivity | `PO-vb-pg2wq-001` |
| `verus` | not_applicable | no-production-bound-seam | — |
| `kani` | not_applicable | superseded_by_other_lane_with_evidence | — |
| `flux-rs` | not_applicable | trigger-not-present | — |
| `loom` | not_applicable | trigger-not-present | — |
| `miri` | not_applicable | trigger-not-present | — |
| `cargo-fuzz` | not_applicable | trigger-not-present | — |

### vb-pg2wq-seed-class-no-regression (PS_00x series pattern discipline)

| Verifier | Applicability | Reason kind | Required obligation IDs |
|----------|---------------|-------------|--------------------------|
| `proptest` | required | audit-regression-resistance + pattern-discipline (cross-cutting source-lint scan) | `PO-vb-pg2wq-003` |
| `verus` | not_applicable | no-production-bound-seam | — |
| `kani` | not_applicable | superseded_by_other_lane_with_evidence | — |
| `flux-rs` | not_applicable | trigger-not-present | — |
| `loom` | not_applicable | trigger-not-present | — |
| `miri` | not_applicable | trigger-not-present | — |
| `cargo-fuzz` | not_applicable | trigger-not-present | — |

### vb-pg2wq-seed-kani-binding-strengthened (runtime↔Kani alignment)

| Verifier | Applicability | Reason kind | Required obligation IDs |
|----------|---------------|-------------|--------------------------|
| `proptest` | not_applicable | superseded_by_other_lane_with_evidence (PO-vb-pg2wq-001/002 cover this seed's surface) | — |
| `verus` | not_applicable | no-production-bound-seam | — |
| `kani` | not_applicable | superseded_by_other_lane_with_evidence (existing harness at crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59 unchanged) | — |
| `flux-rs` | not_applicable | trigger-not-present | — |
| `loom` | not_applicable | trigger-not-present | — |
| `miri` | not_applicable | trigger-not-present | — |
| `cargo-fuzz` | not_applicable | trigger-not-present | — |

## Aggregate counts

| Applicability | Count |
|---------------|-------|
| `required`    | 7 |
| `not_applicable` | 49 |
| `blocked_tooling` | 0 |
| **Total**     | **56** |

## Evidence reference discipline

Every `not_applicable` row carries at least two SHA-256 evidence references:

- `contract.md` SHA-256: `dd4d338812807d0031826d05ce822d6fa342c0e7e89466de09ef08b5657daf05` (primary source of O6-no-production-change evidence)
- `codebase-map.md` SHA-256: `9f900a1816564661a06d03b749ba4dcd62846d66cbfb72f6b7b16503bc982008` (concrete codebase-level evidence: lines 102-106 for fuzz out-of-scope, lines 263-278 for production API surface, lines 301-306 for concurrency surface, lines 318-324 for Kani harness binding)
- `traceability-matrix.jsonl` SHA-256: `99df14c409b3b9f6e61014fa6cbd8f6cb363dfc19a689cd485dfe4ab1b3ec942` (per-row production contract references)

The `not_applicable` reason vocabulary is restricted to the schema-allowed values: `surface_absent`, `risk_out_of_scope`, `superseded_by_other_lane_with_evidence`, `trigger-not-present` (a documented alias for `surface_absent`), and `no-production-bound-seam` (a documented alias for `risk_out_of_scope`).

## Pairing with proof obligations

| Proof obligation ID | Required by lane decisions | Functions exercised |
|---------------------|----------------------------|----------------------|
| `PO-vb-pg2wq-001` | `VLD-vb-pg2wq-ps001-proptest`, `VLD-vb-pg2wq-ps003-proptest`, `VLD-vb-pg2wq-ps008-proptest`, `VLD-vb-pg2wq-ps009-proptest` | `ps001_duplicate_rejected`, `ps003_dup_fields`, `ps008_dup_before_queue`, `ps009_dup_rejected` |
| `PO-vb-pg2wq-002` | `VLD-vb-pg2wq-ps004a-proptest`, `VLD-vb-pg2wq-ps004b-proptest` | `ps004_no_persist`, `ps004_empty_commit_after_rej` |
| `PO-vb-pg2wq-003` | `VLD-vb-pg2wq-class-proptest` | cross-cutting source-lint scan over `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001/003/004/008/009.rs` |

Every `required` lane decision has at least one paired `proof-obligation/v1` ID, and every proof obligation is paired with at least one `required` lane decision. No `blocked_tooling` row is emitted.