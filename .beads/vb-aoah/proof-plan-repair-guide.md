# Proof Plan Repair Guide — vb-aoah State 4 Scope Reduction

## Repair Target

Rejected: over-scoped State 4 proof plan (56 lanes, 36 obligations across 8 verifiers).
Approved: reduced scope (kani + proptest + cargo-fuzz, 18 obligations across 3 verifiers).

## Exact Repairs Required

### 1. Accept reduced verifier lane profile
- **Kani**: 7 obligations (PO-R01–PO-R07), one per proof seed. Bounded migration properties: panic/overflow/unchecked-indexing freedom.
- **Proptest**: 7 obligations (PO-R08–PO-R14), one per proof seed. Behavior/integration tests in `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`.
- **Cargo-fuzz**: 4 obligations (PO-R15–PO-R18), for seeds with hostile input surfaces (001, 004, 006, 007). Hostile manifest/old-record/codec fuzzing.

### 2. Excluded verifiers (not_applicable with evidence)
- **TLA+**: test-first bead, no temporal property requirements. Revisit in production-migration bead.
- **Verus**: no production implementation to bind specs to. GOD RULE forbids vacuum proofs. Revisit post-implementation.
- **Flux**: no refinement type-level enforcement needed at test-first stage. Revisit post-implementation.
- **Loom**: no concurrency in bead scope. Boundary-map.md and hazard-analysis.md confirm absence.
- **Miri**: no unsafe/FFI/raw-pointer code in bead scope.

### 3. Minimum rerun state
- Return to State 5 (proof-writer) with the reduced verifier-lane-review.jsonl and proof-plan-review.md.
- Proof-writer must write only Kani harnesses, proptest properties, behavior tests, and fuzz targets.
- Use `kani::Arbitrary` for Kani shapes (GOD RULE: no hardcoded shapes).
- Behavior tests must test actual production/minimal-infrastructure code, not local adapters.
- Fuzz targets must exercise hostile manifest/old-record/codec inputs.

### 4. Artifact changes
- Overwrite: proof-plan-review.md, verifier-lane-review.jsonl
- New: proof-plan-findings.jsonl, proof-plan-repair-guide.md
- Archive: state4-rejected-over-scoped/, state6-rejected-attempt4/

## Smallest State to Rerun
State 5 (proof-writer) with the reduced plan. All prior State 1-3 artifacts (contract.md, proof-seeds.jsonl, etc.) remain valid.
