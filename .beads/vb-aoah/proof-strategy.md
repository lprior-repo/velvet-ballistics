# Proof Strategy (Reduced Scope) — vb-aoah

State 4 reduced-scope replan per approved proof-plan-review.md (invocation: proof-plan-reviewer-vb-aoah-state4-reduced-001). Plans proof obligations only. No proof/model/harness/test/production code is written here.

## Risk classification

- **Temporal/state-machine**: runtime open rejection/no side effects; verify-before-advance; cleanup-before-success; reopen isolation; empty no-op explicitness. Covered by Kani (bounded phase transition verification) and proptest (behavior/integration tests). TLA+ excluded — test-first bead, revisit post-implementation.
- **Rust-local invariant/bounded**: storage version classification, registry uniqueness, migration phase ordering, cleanup/verification outcome lattice. Covered by Kani (panic/overflow/index freedom) and proptest (property-based behavior tests).
- **Bounded state/arithmetic**: migrated/deleted record counts and byte totals must use checked bounded arithmetic. Covered by Kani (checked arithmetic verification) and proptest (overflow error behavior tests).
- **Hostile input/corruption**: old/future/corrupt manifest/record and malformed old fixture paths require fuzz coverage where parser/codec boundaries are touched (seeds 001, 004, 006, 007).
- **Concurrency/unsafe/UB**: no seed or source map identifies implementation concurrency, unsafe, FFI, aliasing, or provenance scope. Loom/Miri are explicit not_applicable rows.
- **Refinement/type-state** (Verus/Flux): excluded. Test-first bead — no production Rust implementation exists for Verus specs to bind to and no refinement type-level enforcement is needed at skeleton stage. Revisit post-implementation.

## Lane plan (reduced)

Three verifier lanes, 18 obligations total:

### Kani (7 obligations — PO-R01 through PO-R07)
Bounded model checking for panic/overflow/unchecked-indexing/assertion freedom across all 7 proof seeds. All Kani harnesses must use `kani::Arbitrary` or bounded generators — never hardcoded shapes (GOD RULE). Harnesses must exercise actual production/minimal-infrastructure functions in `vb_storage`, not proof-only local adapters.

### Proptest (7 obligations — PO-R08 through PO-R14)
Property-based behavior/integration tests at `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`. Each test exercises actual `vb_storage` migration infrastructure against generated bounded storage fixtures. Typed error paths and success postconditions are both exercised.

### Cargo-fuzz (4 obligations — PO-R15 through PO-R18)
Fuzz targets for seeds with hostile input surfaces (001, 004, 006, 007) at manifest/codec/record boundaries. Targets exercise corrupt, truncated, malformed, and boundary-value inputs with no panics allowed — all failures must be typed errors.

### Excluded lanes

| Verifier | Reason | Revisit trigger |
|---|---|---|
| TLA+ | Test-first bead; no production temporal behavior exists | Production-migration bead after implementation |
| Verus | Test-first bead; GOD RULE forbids vacuum proofs without production Rust to bind to | Post-implementation bead |
| Flux | Test-first bead; no refinement type-level enforcement needed at skeleton stage | Post-implementation bead |
| Loom | No concurrency scope; boundary-map.md and hazard-analysis.md confirm pure-core | If implementation later adds threads/async/shared-state |
| Miri | No unsafe/FFI/raw-pointer scope; boundary-map.md and hazard-analysis.md confirm safe Rust | If implementation later adds unsafe/FFI/raw-pointer code |

## Non-negotiable proof constraints

- **No hardcoded Kani shapes**: use `kani::Arbitrary` or bounded generators (GOD RULE).
- **No proof-only local adapters**: Kani harnesses and proptest tests must exercise actual production/minimal-infrastructure functions in `vb_storage`, not duplicated models.
- **No proof-contract weakening**: any failing Kani/proptest/fuzz evidence must drive implementation repair, not obligation modification.
- **Trusted base**: Fjall persistence, Postcard codec, and bounded model constants remain trusted external dependencies per `trusted-base-plan.md`. Their behavior must be validated by integration/proptest/fuzz evidence at the boundaries.
- **No behavior-affecting waivers**: WC-001 (non-behavior performance evidence) is the only waiver candidate; no new behavior-affecting waivers are introduced.

## Downstream artifacts

- `proof-obligations.planned.jsonl`: 18 planned obligations for proof-writer (State 5)
- `verifier-lane-decisions.jsonl`: 56 lane decisions (7 seeds × 8 verifiers)
- `proof-coverage-matrix.md`: requirement-to-obligation traceability
- `trusted-base-plan.md`: trusted surface ledger (unchanged from prior plan)
- `waiver-candidates.jsonl`: WC-001 pending, non-behavior
- `proof-to-implementation-input.md`: bridge input for proof-to-implementation (State 12)
