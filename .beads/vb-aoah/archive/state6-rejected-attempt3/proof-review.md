# Proof Review — vb-aoah State 6 Attempt 3

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-aoah-state6-003
writer_invocation_id: proof-writer-vb-aoah-state5-006
review_state: 6
bead_id: vb-aoah
sublane: proof-review
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-aoah
reviewed_at: 2026-05-25T23:05:00Z

## Findings

### CRITICAL — FINDING-PROOF-001 — Verus artifacts remain disconnected local models

Artifacts: `verification/verus/vb_aoah_runtime_open_no_side_effects.rs` and sibling `verification/verus/vb_aoah_*.rs` files.

Obligations: PO-002, PO-007, PO-012, PO-017, PO-023, PO-027, PO-032.

Raw evidence:

- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:3` says the Rust boundary is only modeled for State-7 target `vb_storage::migrations`; it does not import or bind to a production migration/open/manifest API.
- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:8-23` defines proof-local enums and spec functions (`State7OpenOutcome`, `Phase`, `supported_old`, `registry_name`, `open_classify`).
- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:35-95` contains tautological proof functions such as `ensures verified ==> verified`, `ensures old_records == 0 ==> old_records == 0`, and `ensures 0 == 0`.
- `.beads/vb-aoah/proof-writer-report.md:16` admits these findings were not substantively discharged and that Verus artifacts remain abstract pending State 7 implementation binding.

Impact: The Verus lane still violates the planned assumption that proof artifacts bind to production functions/types and cannot use toy-only duplicates. Raw Verus success logs prove local helper specifications, not implementation behavior.

Required fix: Bind Verus specs/proofs to actual State 7 Rust API or obtain an explicit reviewed waiver/blocker. Remove tautological local proof helpers.

### CRITICAL — FINDING-PROOF-002 — Kani and proptest prove proof-scoped adapters, not public production behavior

Artifacts: `crates/vb_storage/src/vb_aoah_*_kani.rs`; `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`.

Obligations: PO-003, PO-005, PO-008, PO-010, PO-013, PO-015, PO-018, PO-020, PO-024, PO-025, PO-028, PO-030, PO-033, PO-035.

Raw evidence:

- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs:20-39` defines `state7_*_adapter` helper functions inside the harness instead of calling production migration/open/manifest code.
- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs:41-89` verifies assertions over those local adapters only.
- `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs:54-86` defines the same local `state7_*_adapter` functions, and `:88-133` tests those helpers rather than a public `vb_storage` migration API.
- `.beads/vb-aoah/proof-writer-report.md:17` admits these Kani/proptest findings were not substantively discharged and remain bounded adapter evidence until a production migration API exists.

Impact: `kani::Arbitrary` generation is present, but generated input into proof-local adapters is still detached from executable production Rust realization. The implementation obligations remain unproven.

Required fix: Make Kani/proptest exercise production functions or thin wrappers over production functions while preserving bounded generated input coverage.

### HIGH — FINDING-PROOF-003 — Required Flux obligations have no accepted refinements and exact planned command failed

Artifacts: `crates/vb_storage/src/vb_aoah_*_flux.rs`; `.beads/vb-aoah/raw-evidence/attempt3/flux-planned-with-lib.log`; `.beads/vb-aoah/raw-evidence/attempt3/flux-compatible.log`.

Obligations: PO-004, PO-009, PO-014, PO-019, PO-029, PO-034.

Raw evidence:

- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_flux.rs:17-48` contains comments describing “Flux refinement intent” and ordinary Rust functions; there are no active `flux_rs` attributes, `#[sig]`, or `#[refined_by]` annotations.
- `.beads/vb-aoah/raw-evidence/attempt3/flux-planned-with-lib.log:1-5` records the exact planned command failed: `error: unexpected argument '--lib' found`.
- `.beads/vb-aoah/raw-evidence/attempt3/flux-compatible.log:1-2` shows only that a substituted package-level command finished; it does not satisfy the planned command nor prove any refinement annotations.
- `.beads/vb-aoah/proof-writer-report.md:18` admits the Flux finding was not substantively discharged and that the compatible command remains compensating evidence only.

Impact: Required Flux lane decisions remain unsatisfied. Commented “intent” is not a refinement proof, and command substitution is not an approved waiver.

Required fix: Add real Flux refinements accepted by the installed tool and repair the planned command, or obtain an explicit proof-plan/review waiver for Flux lane non-execution.

### HIGH — FINDING-PROOF-004 — Trusted-base ledger records unapproved behavior-affecting bounds

Artifacts: `.beads/vb-aoah/trusted-base-ledger.jsonl`; `.beads/vb-aoah/proof-evidence.md`.

Obligations: PO-001 through PO-036.

Raw evidence:

- `.beads/vb-aoah/trusted-base-ledger.jsonl:1-36` shows every bounded-model trust entry with `reviewer_disposition: requires_independent_proof_review` and `status: RECORDED_REQUIRES_REVIEW`, not an approved disposition.
- `.beads/vb-aoah/proof-evidence.md:22-26` explicitly lists downstream assumptions: State 7 production `crates/vb_storage/src/migrations.rs` is absent, Flux attribute insertion remains pending, and proof-writer did not set reviewer approval dispositions.
- `.beads/vb-aoah/proof-writer-report.md:47-48` retains blockers `PRODUCTION_BINDING_PENDING` and `FLUX_EXACT_COMMAND_INCOMPATIBLE`.

Impact: Proof review cannot approve while behavior-affecting trust boundaries are recorded as requiring independent review and the package itself admits production binding and Flux exact-command blockers.

Required fix: Resolve trusted-base ledger rows to approved/rejected dispositions with precise scope after production-bound proof repair, or keep the package rejected until an approved waiver/blocker explicitly covers these limitations.

## Reviewed inputs

- `.beads/vb-aoah/state5-validation-evidence.json` — official State 5 validator PASS (`raw-evidence/attempt6/state5-official-validator.log:1`).
- `.beads/vb-aoah/agent-invocation-ledger.jsonl` — lineage through ledger sequence 15.
- `.beads/vb-aoah/proof-obligations.planned.jsonl`, `.beads/vb-aoah/verifier-lane-decisions.jsonl`, `.beads/vb-aoah/proof-writer-report.md`, `.beads/vb-aoah/proof-evidence.md`, `.beads/vb-aoah/trusted-base-ledger.jsonl`.
- Current proof artifacts: `verification/verus/vb_aoah_*.rs`, `verification/tla/vb_aoah_*.tla`, `crates/vb_storage/src/vb_aoah_*_{kani,flux}.rs`, `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`, and `fuzz/fuzz_targets/vb_aoah_*.rs`.
- Archived rejected reviews under `.beads/vb-aoah/archive/state6-rejected-attempt1/` and `.beads/vb-aoah/archive/state6-rejected-attempt2/`.

## Provenance and ledger checks

- Active writer invocation `proof-writer-vb-aoah-state5-006` is present in `.beads/vb-aoah/agent-invocation-ledger.jsonl` as ledger sequence 15, skill `proof-writer`, state 5, status `completed`.
- This reviewer invocation is `proof-reviewer-vb-aoah-state6-003`; it is distinct from all State 5 proof-writer invocation ids. No self-approval path found.
- State 5 validator evidence reports `STATUS: PASS`. That validates State 5 artifact/ledger shape only; it does not approve proof substance.
- This review writes only review/provenance artifacts under `.beads/vb-aoah/`: `proof-review.md`, `proof-findings.jsonl`, `transcript-state6-proof-reviewer.md`, and a normalized State 6 ledger row.

## Positive evidence retained but insufficient

- TLA+ raw logs under `.beads/vb-aoah/raw-evidence/attempt3/tla-vb_aoah_*.log` report bounded TLC completion. This supports bounded abstract models only.
- Kani raw logs report successful bounded checks, and proptest/fuzz logs are durable. They do not overcome the adapter/production-binding gap above.
- The official State 5 validator passed after attempt 6, but State 6 proof review must reject because required proof obligations remain open or admitted as pending.

## Review decision

REJECTED. The active State 5 PASS package is validator-shaped, but it still proves local models/adapters and commented Flux intent rather than the required production-bound obligations. The package also explicitly preserves production-binding and exact Flux-command blockers.

STATUS: REJECTED
