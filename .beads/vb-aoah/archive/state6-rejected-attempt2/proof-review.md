# Proof Review — vb-aoah State 6 Attempt 2

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-aoah-state6-002
writer_invocation_id: proof-writer-vb-aoah-state5-005
review_state: 6
bead_id: vb-aoah
sublane: proof-review
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-aoah
reviewed_at: 2026-05-25T21:20:00Z

## Reviewed inputs

- `.beads/vb-aoah/state5-validation-evidence.json` — State 5 validator PASS.
- `.beads/vb-aoah/agent-invocation-ledger.jsonl` — writer lineage through ledger sequence 13.
- `.beads/vb-aoah/proof-obligations.planned.jsonl` and `.beads/vb-aoah/verifier-lane-decisions.jsonl`.
- `.beads/vb-aoah/proof-writer-report.md`, `.beads/vb-aoah/proof-evidence.md`, `.beads/vb-aoah/trusted-base-ledger.jsonl`.
- Raw logs under `.beads/vb-aoah/raw-evidence/attempt3/`.
- Current proof artifacts: `verification/verus/vb_aoah_*.rs`, `verification/tla/vb_aoah_*.tla`, `crates/vb_storage/src/vb_aoah_*_{kani,flux}.rs`, `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`, and `fuzz/fuzz_targets/vb_aoah_*.rs`.
- Archived prior rejection under `.beads/vb-aoah/archive/state6-rejected-attempt1/` was reviewed as context only.

## Provenance and ledger checks

- Active writer invocation `proof-writer-vb-aoah-state5-005` is present in `.beads/vb-aoah/agent-invocation-ledger.jsonl` as ledger sequence 13, skill `proof-writer`, state 5, status `completed`.
- This reviewer invocation is `proof-reviewer-vb-aoah-state6-002`; it is distinct from all State 5 proof-writer invocation ids. No self-approval path found.
- State 5 validator evidence reports `status: PASS` with no findings. That validates artifact-shape bookkeeping only; it does not override proof-review substance.
- This review wrote review artifacts only: `.beads/vb-aoah/proof-review.md`, `.beads/vb-aoah/proof-findings.jsonl`, `.beads/vb-aoah/transcript-state6-proof-reviewer.md`, and a normalized State 6 row in `.beads/vb-aoah/agent-invocation-ledger.jsonl`.

## Findings

### CRITICAL — FINDING-PROOF-001 — Verus artifacts remain disconnected local models

Artifacts: `verification/verus/vb_aoah_runtime_open_no_side_effects.rs` and sibling `verification/verus/vb_aoah_*.rs` files.

Obligations: PO-002, PO-007, PO-012, PO-017, PO-023, PO-027, PO-032.

Raw evidence:

- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:3` says the Rust boundary is only modeled for State-7 target `vb_storage::migrations`; there is no import of or binding to actual production migration/open/manifest functions.
- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:8-23` defines proof-local enums and spec functions (`State7OpenOutcome`, `Phase`, `supported_old`, `registry_name`, `open_classify`).
- `verification/verus/vb_aoah_runtime_open_no_side_effects.rs:25-95` proves local helper behavior with trivial or tautological proof functions (`ensures verified ==> verified`, `ensures old_records == 0 ==> old_records == 0`, `ensures 0 == 0`).
- The same local `supported_old`, `registry_name`, and `migration_checked_accounting` pattern appears in all seven current `verification/verus/vb_aoah_*.rs` files.

Impact: The Verus lane still violates the repository “No Vacuum Verus Proofs” rule and the planned assumption “Proof artifacts must bind to production functions/types and cannot use toy-only duplicates.” `verification results:: 15 verified, 0 errors` is not evidence that the implementation satisfies the contract.

Required fix: Replace local Verus transition systems with proofs/specs bound to the actual State 7 Rust API or move these obligations behind an explicit approved waiver/blocker. Do not encode desired outcomes as tautological local helpers.

### CRITICAL — FINDING-PROOF-002 — Kani and proptest prove proof-scoped adapters, not public production behavior

Artifacts: `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs` and sibling Kani files; `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`.

Obligations: PO-003, PO-005, PO-008, PO-010, PO-013, PO-015, PO-018, PO-020, PO-024, PO-025, PO-028, PO-030, PO-033, PO-035.

Raw evidence:

- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs:20-39` defines `state7_*_adapter` helper functions inside the harness instead of calling production migration/open/manifest code.
- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs:41-89` verifies assertions over those local adapters only.
- `.beads/vb-aoah/raw-evidence/attempt3/kani-vb_aoah_runtime_open_no_side_effects.log:107-116` shows Kani successfully checked one harness, but the harness target is the adapter-only module.
- `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs:54-86` defines the same local `state7_*_adapter` functions and `:88-133` tests those helpers, not a public `vb_storage` migration API.

Impact: `kani::Arbitrary` generation is present, but arbitrary input into adapter functions is still detached from executable Rust realization. These obligations cannot be approved as implementation proof evidence.

Required fix: Bind Kani/proptest to production functions or to adapters that are thin wrappers over production functions, and preserve generator coverage for the approved bounded state space.

### HIGH — FINDING-PROOF-003 — Required Flux obligations have no accepted refinements and exact planned command failed

Artifacts: `crates/vb_storage/src/vb_aoah_*_flux.rs`; `.beads/vb-aoah/raw-evidence/attempt3/flux-planned-with-lib.log`; `.beads/vb-aoah/proof-evidence.md`.

Obligations: PO-004, PO-009, PO-014, PO-019, PO-029, PO-034.

Raw evidence:

- `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_flux.rs:17-48` contains only comments describing “Flux refinement intent” and ordinary Rust functions; there are no active `flux_rs` attributes, `#[sig]`, or `#[refined_by]` annotations.
- `.beads/vb-aoah/raw-evidence/attempt3/flux-planned-with-lib.log:1-5` records the exact planned command failed: `error: unexpected argument '--lib' found`.
- `.beads/vb-aoah/raw-evidence/attempt3/flux-compatible.log:1-2` shows only that a substituted package-level command finished; it does not satisfy the planned command nor prove any refinement annotations.

Impact: Required Flux lane decisions remain unsatisfied. Commented “intent” is not a refinement proof, and command substitution is not an approved waiver.

Required fix: Either add real Flux refinements accepted by the installed tool and repair the planned command, or obtain an explicit proof-plan/review waiver for Flux lane non-execution.

### HIGH — FINDING-PROOF-004 — Trusted-base ledger still has pending reviewer dispositions

Artifacts: `.beads/vb-aoah/trusted-base-ledger.jsonl`.

Obligations: PO-001 through PO-036.

Raw evidence:

- `.beads/vb-aoah/trusted-base-ledger.jsonl:1-30` shows bounded-model trust entries with `reviewer_disposition: pending_proof_review` and `status: PENDING_REVIEW`.
- `.beads/vb-aoah/proof-evidence.md:29-32` retains assumptions that State 7 production `crates/vb_storage/src/migrations.rs` is not present and that real Flux attribute insertion remains pending.

Impact: Proof review cannot approve while trust entries remain pending and behavior-affecting production binding is explicitly deferred.

Required fix: Resolve trusted-base ledger rows to approved/rejected dispositions with precise scope, or keep the package rejected until production binding and Flux trust boundaries are reviewed.

## Positive evidence retained but insufficient

- TLA+ raw logs such as `.beads/vb-aoah/raw-evidence/attempt3/tla-vb_aoah_runtime_open_no_side_effects.log:19-24` report bounded TLC completion with no errors. This supports the bounded abstract model only.
- Kani raw logs report successful bounded checks, and fuzz/proptest logs are now durable raw artifacts. They do not overcome the adapter/production-binding gap above.

## Review decision

REJECTED. State 5 validation passed bookkeeping, but the active proof package still proves local models/adapters and commented Flux intent rather than required production-bound obligations.

STATUS: REJECTED
