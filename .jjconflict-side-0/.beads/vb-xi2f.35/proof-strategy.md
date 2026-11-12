# Proof Strategy: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Strategy Version

`velvet-ballastics/v1` — proof-planner state 4

## Context

`canonical_digest()` in both compilation paths (`part_05.rs:116-138`, `compile/mod.rs:220-241`) hashes ONLY source-level properties (version, name, trigger, step IDs, primitives). ZERO ResourceContract fields are hashed. Changing any of the 17 contract fields produces the identical digest — violating the fundamental contract that the digest identifies a workflow's complete semantics.

Additionally, two `ResourceContract` types coexist: a 17-field canonical type in `workflow/mod.rs` and a 15-field duplicate in `compiled_workflow.rs` (missing `max_transitions_per_tick` and `allows_secret_results`). Validation code uses the 15-field type and cannot validate the two missing dimensions.

## Contract Status (from contract.md)

| Clause | Status | Severity | Proof Priority |
|--------|--------|----------|---------------|
| C1: Digest-Contract Binding | VIOLATED | CRITICAL | P0 |
| C2: Single Canonical Type | VIOLATED | HIGH | P0 |
| C3: Entry Point Contract | VIOLATED | HIGH | P1 |
| C4: Taint Digest Sensitivity | VIOLATED | HIGH | P0 |
| C5: Full Validation Coverage | VIOLATED | HIGH | P1 |
| C6: Dual Path Consistency | AT-RISK | MEDIUM | P1 |
| C7: YAML Contract Parsing | NOT-IMPLEMENTED | MEDIUM | P2 (waivable) |
| C8: Backward Compatibility | NEEDS-PLANNING | MEDIUM | P1 |
| C9: Proof Obligation | NOT-STARTED | REQUIRED | (this artifact) |
| C10: Non-Requirements | CONFIRMED | N/A | — |

## Proof Architecture

The proof strategy follows a defense-in-depth layering from cheapest to most expensive:

```
Layer 1: Kani bounded model checking (panic-freedom, determinism, bounded injectivity)
    ↓    Proves local invariants on bounded inputs.
Layer 2: Proptest property-based testing (broad random input coverage)
    ↓    Catches regressions and edge cases Kani bounds miss.
Layer 3: Verus formal proofs (for-all quantifier, encoding injectivity)
    ↓    Proves universal properties that bounded checks cannot.
Layer 4: Cargo-fuzz adversarial input (parser/deserialization robustness)
         Only applied where untrusted input boundaries exist (P2 YAML parsing).
```

### Lane Selection Rationale

| Verifier | Applied? | Rationale |
|----------|----------|-----------|
| **Kani** | YES — 14 obligations | Primary lane for bounded Rust invariants: determinism, field sensitivity, type identity, validation boundaries. All 17 proof seeds have Kani-suitable bounded properties. |
| **Verus** | YES — 4 obligations | Formal for-all proofs: contract inequality ⇒ digest inequality, encoding injectivity, allows_secret_results injectivity, runtime contract identity. Applies to PS-001, PS-003, PS-008, PS-009, PS-016. |
| **Proptest** | YES — 7 obligations | Broad-input regression for field sensitivity, entry-point contracts, dual-path equivalence, determinism. Applies to PS-002, PS-007, PS-008, PS-010, PS-013, PS-014, PS-017. |
| **cargo-fuzz** | YES — 1 obligation (waived) | Applies to PS-011 (YAML contract parsing) — P2 priority, waived for this P1 bead. |
| **Flux RS** | NO | Numeric refinements add no material value when Kani already covers bounded invariants. The 17-field struct does not have index-struct relationships expressible better as refinements. |
| **TLA+** | NO | No temporal/state-machine behavior. `canonical_digest()` is a pure, stateless hash function. Runtime enforcement (`handle_ask_answer`) is deterministic per-state. No queue ordering, retry protocol, lease, or distributed protocol. |
| **Loom** | NO | No concurrent interleavings. Digest computation is single-threaded. No atomics, channels, locks, or async shutdown in affected code. |
| **Miri** | NO | No unsafe code, FFI, raw pointers, or interior mutability in affected paths. `#![forbid(unsafe_code)]` confirmed on all relevant modules. |

### Phase Plan

#### Phase 0: Prerequisites (implementation pre-work)
- Resolve duplicate `ResourceContract` types (C2)
- Add `contract: ResourceContract` parameter to `canonical_digest()` and `compile_source()` (C1, C3)
- Implement tagged field hashing (C1, C4)

#### Phase 1: Kani Bounded Verification (14 obligations)
- Determinism of `canonical_digest(source, contract)`
- Single-field sensitivity for each of 17 fields
- Domain-tag encoding prevents cross-field collisions
- `allows_secret_results` changes digest
- Runtime enforcement matches hashed contract
- Dual compilation path equivalence
- Validation boundary checks for all 17 fields
- Migration digest relationship from old to new
- Type identity across code paths
- `compile_source_with_default` equivalence

#### Phase 2: Proptest Property Testing (7 obligations)
- Per-field digest sensitivity with random values
- All-fields-randomized digest difference
- Entry point contract parametrization
- Dual-path equivalence fuzzing
- Determinism across random contract pairs
- `compile_source_with_default` equivalence fuzzing

#### Phase 3: Verus Formal Proofs (4 obligations)
- For-all contract inequality ⇒ digest inequality
- Encoding injectivity proof
- `allows_secret_results` injectivity proof
- Runtime contract identity through serialization/deserialization

#### Phase 4: Cargo-fuzz (waived, P2)
- YAML parser contract section fuzzing — waived for P1

## Hazard Coverage Map

| Hazard | Proof Seeds | Kani Obligations | Verus Obligations | Proptest Obligations |
|--------|-------------|-----------------|-------------------|---------------------|
| H-001: Digest orphan | PS-001..PS-004 | PO-K01..PO-K04 | PO-V01 | PO-P01, PO-P07 |
| H-002: Duplicate types | PS-005, PS-006 | PO-K05, PO-K06 | — | — |
| H-003: Hardcoded DEFAULT | PS-007, PS-017 | PO-K07, PO-K13 | — | PO-P02, PO-P06 |
| H-004: Taint silent match | PS-008, PS-009 | PO-K08, PO-K09 | PO-V03, PO-V04 | PO-P03 |
| H-005: Dual path drift | PS-010 | PO-K10 | — | PO-P04 |
| H-006: Missing YAML parsing | PS-011 | — (waived) | — | — (waived) |
| H-007: Validation gap | PS-012 | PO-K11 | — | — |
| H-008: No test coverage | PS-013, PS-014 | PO-K01 | — | PO-P05 |
| H-009: Digest split | PS-015 | PO-K14 | — | — |
| H-010: Field name stability | PS-016 | PO-K12 | PO-V02 | — |

## Non-Vacuity Assurance

1. **Kani harnesses**: All harnesses use `kani::any()` or exhaustive generators. No hardcoded dummy structs. Bounds are tight but representative (e.g., u16 fields bounded 0..256, u64 fields bounded 0..64, bool fields fully enumerated).
2. **Verus proofs**: All `proof fn` and `spec fn` models bind to actual Rust `exec fn` implementations. No standalone model-only proofs.
3. **Proptest strategies**: All 17 fields have `proptest::arbitrary::Arbitrary` implementations or manual strategies. Bounds reflect realistic domain values.
4. **No proof-only mutations**: Harnesses test the same code path that production uses. Stubs only for external deps (blake3) and only with equivalence proofs.

## Risk Classification Summary

| Risk Trigger | Classification | Lane Decision |
|-------------|---------------|--------------|
| Pure hash determinism | Rust-local invariant | Kani + Verus + Proptest |
| Injectivity of encoding | Pure/core function correctness | Verus + Kani |
| Type identity/import hygiene | Compile-time | Kani |
| Digest sensitivity (17 fields) | Bounded state machine | Kani + Proptest |
| Runtime enforcement | Rust-local invariant | Kani + Verus |
| Dual compilation paths | Redundancy/drift | Proptest + Kani |
| YAML parser input | Untrusted input boundary | Fuzz (P2, waived) |
| Backward compatibility | Migration | Kani |

## Tool Requirements

| Tool | Version Required | Install State |
|------|-----------------|---------------|
| Kani | latest (via `cargo kani`) | `cargo kani --version` |
| Verus | latest (via `verus`) | `verus --version` |
| proptest | 1.x (Cargo dependency) | In Cargo.toml |
| cargo-fuzz | latest (via `cargo fuzz`) | `cargo fuzz --version` |

## Artifact Layout

```
verification/
├── kani/
│   ├── vb_compile/
│   │   ├── digest_determinism.rs          (PO-K01, PO-K14)
│   │   ├── digest_field_sensitivity.rs    (PO-K02, PO-K08)
│   │   ├── digest_cross_field_collision.rs (PO-K03)
│   │   ├── migration_digest.rs            (PO-K04)
│   │   ├── entry_point_contract.rs        (PO-K07)
│   │   ├── dual_path_equivalence.rs       (PO-K10)
│   │   └── with_default_equivalence.rs    (PO-K13)
│   ├── vb_core/
│   │   ├── type_canonical_fields.rs       (PO-K05)
│   │   ├── type_identity_paths.rs         (PO-K06)
│   │   ├── encoding_injectivity.rs        (PO-K12)
│   │   └── validation_17_fields.rs        (PO-K11)
│   └── vb_runtime/
│       └── secret_result_enforcement.rs   (PO-K09)
├── verus/
│   ├── vb_compile/
│   │   ├── digest_contract_binding.rs     (PO-V01)
│   │   ├── encoding_injectivity.rs        (PO-V02)
│   │   └── secret_results_injectivity.rs  (PO-V03)
│   └── vb_runtime/
│       └── contract_identity_tracking.rs  (PO-V04)
└── fuzz/
    └── fuzz_targets/ (waived P2)
```

## Trusted Base

See `trusted-base-plan.md` for complete trust assumptions:
- BLAKE3 collision resistance (trusted external crate)
- Rust type system soundness (trusted compiler)
- `postcard` serialization determinism (trusted for policy digest)
- Bounded field ranges for Kani (documented per-obligation)

## Waiver Candidates

| Candidate | Clause | Reason | P1 Scope |
|-----------|--------|--------|----------|
| WC-001 | C7 (YAML Contract Parsing) | P2 priority, explicitly out of scope for P1 bead. No behavior-affecting change — all contracts are DEFAULT until YAML parsing exists. | P1 exclusions confirmed in contract.md Section C10. |

See `waiver-candidates.md` and `waiver-candidates.jsonl`.

## Blocker Assessment

No blockers identified. All required tools are installable. The P1 proof obligations cover all CRITICAL and HIGH severity hazards.

## Reviewer Instructions

This plan is ready for `proof-plan-reviewer` review under `proof-plan-review.md` and `verifier-lane-review.jsonl`. The reviewer owns:
- Lane decision validation (are we missing any lanes?)
- Obligation sufficiency (is each obligation strong enough?)
- Non-vacuity verification (are harnesses honest?)
- Waiver approval/rejection
