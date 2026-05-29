# Proof Review — vb-aoah State 6

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-aoah-state6-001
writer_invocation_id: proof-writer-vb-aoah-state5-002
review_state: 6
bead_id: vb-aoah
sublane: proof-review
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-aoah
reviewed_at: 2026-05-25T00:00:00Z

## Reviewed inputs

- `.beads/vb-aoah/agent-invocation-ledger.jsonl`
- `.beads/vb-aoah/proof-obligations.planned.jsonl`
- `.beads/vb-aoah/proof-plan-review.md`
- `.beads/vb-aoah/proof-writer-report.md`
- `.beads/vb-aoah/proof-evidence.md`
- `.beads/vb-aoah/trusted-base-ledger.jsonl`
- `.beads/vb-aoah/state5-validation-evidence.json`
- `verification/verus/vb_aoah_*.rs`
- `verification/tla/vb_aoah_*.tla` and `verification/tla/vb_aoah_*.cfg`
- `crates/vb_storage/src/vb_aoah_*_kani.rs`
- `crates/vb_storage/src/vb_aoah_*_flux.rs`
- `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`
- `fuzz/fuzz_targets/vb_aoah_*.rs`

## Provenance

- Writer invocation `proof-writer-vb-aoah-state5-002` is present in `.beads/vb-aoah/agent-invocation-ledger.jsonl` as ledger sequence 9, skill `proof-writer`, state 5, status `completed`.
- Reviewer invocation `proof-reviewer-vb-aoah-state6-001` differs from writer invocation `proof-writer-vb-aoah-state5-002`; no self-approval detected in reviewed ledger rows.
- State 4 proof-plan review status was `STATUS: APPROVED`.
- This review did not modify proof artifacts; it wrote review artifacts only.

## Findings

### CRITICAL — FINDING-PROOF-001 — Verus proofs are disconnected and assumption-shaped

Artifacts: `verification/verus/vb_aoah_runtime_open_no_side_effects.rs` and the other six `verification/verus/vb_aoah_*.rs` files.

Obligations: PO-002, PO-007, PO-012, PO-017, PO-023, PO-027, PO-032.

Raw evidence:

- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:3` says the production binding target is a **future** API.
- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:8-17` defines local proof-only enums/spec functions instead of binding to production `vb_storage` types/functions.
- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:25-65` proves each core claim by restating it in `requires` and repeating it in `ensures` with empty proof bodies.
- Grep evidence found the same `Production binding target: future vb_storage migration API` and local `pub enum OpenOutcome` pattern in all seven `verification/verus/vb_aoah_*.rs` files.

Impact: This violates the repository GOD RULE “No Vacuum Verus Proofs” and the approved plan’s non-vacuity constraint. Verus success on these files does not prove Rust implementation behavior or even a meaningful model theorem.

Required fix: Replace the toy Verus artifacts with Rust-bound Verus proofs/specs whose executable functions/types correspond to the actual State 7 implementation boundary, and remove assumption-shaped `requires` that encode the desired result.

### CRITICAL — FINDING-PROOF-002 — Kani/proptest artifacts verify self-contained toy functions, not production behavior

Artifacts: `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs` and sibling Kani modules; `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`.

Obligations: PO-003, PO-005, PO-008, PO-010, PO-013, PO-015, PO-018, PO-020, PO-024, PO-025, PO-028, PO-030, PO-033, PO-035.

Raw evidence:

- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs:4-14` defines a local `MigrationFrame`.
- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs:21-40` defines local helper functions (`supported_old`, `registry_entry`, `migration_required_no_write`, `checked_accounting`) instead of calling production migration/open/manifest code.
- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs:42-90` asserts properties over those local helpers.
- `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs:54-86` defines local helper functions and `:88-133` tests those helpers rather than public production APIs.

Impact: Kani and proptest evidence is detached from executable Rust realization. The harness uses `kani::Arbitrary`, but arbitrary inputs into a toy transition system do not satisfy the planned requirement that artifacts bind to production functions/types and cannot use toy-only duplicates.

Required fix: Rework Kani harnesses and property tests to exercise production/public APIs or proof-only adapters that directly call production implementation functions, with generated inputs covering the approved bounded state space.

### HIGH — FINDING-PROOF-003 — Flux lane has no Flux refinements and the planned command did not pass

Artifacts: `crates/vb_storage/src/vb_aoah_*_flux.rs`; `.beads/vb-aoah/proof-evidence.md`.

Obligations: PO-004, PO-009, PO-014, PO-019, PO-029, PO-034.

Raw evidence:

- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_flux.rs:5-42` contains ordinary Rust enums/struct/functions only; no `flux_rs`, `#[sig]`, `#[refined_by]`, or equivalent refinement annotations are present.
- Grep for `flux_rs|#[sig|#[refined_by|#[trusted|#[ignore|#[requires|#[ensures]` across `crates/vb_storage/src/vb_aoah_*_flux.rs` returned no matches.
- `.beads/vb-aoah/proof-evidence.md:33-39` records that the exact planned command `cargo flux check -p vb_storage --lib --features proof-vb-aoah-migration` failed with exit 2 and was replaced by a different command.

Impact: The Flux lane proves only that ordinary Rust type-checks under Flux tooling, not the planned refinement/type-state claims. The exact obligation command is not satisfied.

Required fix: Add actual Flux refinements for the claimed state predicates or explicitly waive Flux with approved reviewer disposition; repair planned command drift before using it as proof evidence.

### HIGH — FINDING-PROOF-004 — Raw verifier evidence is summarized, incomplete, and fuzz obligations are pending

Artifacts: `.beads/vb-aoah/proof-evidence.md`; `.beads/vb-aoah/proof-writer-report.md`.

Obligations: PO-001 through PO-036, especially PO-006, PO-021, PO-031, PO-036.

Raw evidence:

- `.beads/vb-aoah/proof-evidence.md:10-51` provides summarized command blocks, not durable raw log artifact paths for each obligation.
- `.beads/vb-aoah/proof-evidence.md:47-51` shows fuzz evidence used `-runs=1`, not the planned `-max_total_time=60 -runs=10000` campaign.
- `.beads/vb-aoah/proof-evidence.md:59` explicitly says full fuzz campaigns are pending formal execution.
- `.beads/vb-aoah/proof-writer-report.md:33-36` repeats pending formal execution and command-substitution decisions.

Impact: Required proof obligations cannot be approved on summaries, command substitutions, and pending execution. Proof review requires raw verifier output or an approved waiver, neither of which is present for full fuzz obligations and exact command drift.

Required fix: Store per-obligation raw verifier logs with stable artifact paths, rerun exact approved commands or repair the proof plan, and complete full fuzz campaigns or obtain explicit approved waivers.

## Review decision

REJECTED. The State 5 proof package is non-vacuous only at the level of toy models. Verus proofs encode the desired results as preconditions, Kani/proptest/Flux artifacts are disconnected from production behavior, and evidence contains pending fuzz execution plus summarized rather than raw per-obligation logs.

STATUS: REJECTED
