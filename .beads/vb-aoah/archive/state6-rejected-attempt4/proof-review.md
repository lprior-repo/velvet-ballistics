# Proof Review — vb-aoah State 6 attempt 4

Reviewer: proof-reviewer
Bead: vb-aoah
State: 6
Sublane: proof-review
Attempt: 4
Workspace: `/home/lewis/isolated/femdation-velvet-ballistics/vb-aoah`
Date: 2026-05-26

## Verdict

Rejected. The State 5 package has valid shape/provenance bookkeeping (`raw-evidence/attempt7/state5-official-validator.log` line 1: `STATUS: PASS`), but proof review is about proof substance. The package itself admits the core proof obligations are not discharged: `proof-writer-report.md` lines 16-19 preserves the prior findings, line 24 says no proof/harness/model/test behavior changed, and lines 45-46 preserve `PRODUCTION_BINDING_PENDING` plus `FLUX_EXACT_COMMAND_INCOMPATIBLE`.

## Findings

### CRITICAL — FINDING-PROOF-001 — Verus proofs are abstract/vacuous and not bound to production Rust

- Obligations: PO-002, PO-007, PO-012, PO-017, PO-023, PO-027, PO-032.
- Artifacts: `verification/verus/vb_aoah_*.rs`; representative: `verification/verus/vb_aoah_runtime_open_no_side_effects.rs`.
- Evidence:
  - `verification/verus/vb_aoah_runtime_open_no_side_effects.rs` lines 3 and 8-23 define a State-7 target comment and standalone model enums/spec fns instead of importing or verifying real `vb_storage` migration/open/manifest code.
  - Lines 25-33 implement a local `runtime_open_without_migration` function; lines 35-38 prove only a model implication. Lines 59-61, 67-69, 85-87 are tautological proof functions.
  - `proof-evidence.md` line 8 explicitly says Verus results are “abstract” and “not production-binding approval”.
  - `proof-writer-report.md` line 16 says this was not substantively discharged and State 7 production API is absent.
- Required fix: bind Verus specs/ensures to actual production Rust APIs/types after implementation, or provide an approved waiver. Abstract duplicate models cannot approve behavior-affecting obligations.

### CRITICAL — FINDING-PROOF-002 — Kani/proptest harnesses verify local adapters, not the production migration/open behavior

- Obligations: PO-003, PO-005, PO-008, PO-010, PO-013, PO-015, PO-018, PO-020, PO-024, PO-025, PO-028, PO-030, PO-033, PO-035.
- Artifacts: `crates/vb_storage/src/vb_aoah_*_kani.rs`; `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`.
- Evidence:
  - Representative Kani artifact `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs` lines 20-39 defines `state7_*_adapter` helpers; lines 41-89 assert over those helpers rather than the real migration implementation.
  - It does use `kani::Arbitrary` at line 4 and `kani::any()` at line 43, so this is not the GOD RULE hardcoded-shape failure; the blocker is adapter detachment and assume-shaped boundedness (`kani::assume` lines 44-47).
  - Workspace test artifact lines 54-86 defines `state7_*_adapter` helpers, then properties at lines 88-133 test those local helpers.
  - `proof-writer-report.md` line 17 admits this remains bounded adapter evidence until production migration API exists.
- Required fix: call real production APIs and assert public outcomes/side-effect observations. Keep generated inputs, but remove proof-only behavior clones as the target of proof.

### HIGH — FINDING-PROOF-003 — Flux lane has no executable Flux refinements and exact planned command fails

- Obligations: PO-004, PO-009, PO-014, PO-019, PO-029, PO-034.
- Artifacts: `crates/vb_storage/src/vb_aoah_*_flux.rs`; raw logs `raw-evidence/attempt3/flux-planned-with-lib.log`, `raw-evidence/attempt3/flux-compatible.log`.
- Evidence:
  - Representative Flux artifact `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_flux.rs` lines 17, 21, 23, 25, 32, 37, 42, and 47 contain only commented “Flux ... intent” markers, not active `#[flux_rs::sig]`, `#[flux_rs::refined_by]`, or field refinements.
  - Grep over `crates/vb_storage/src/vb_aoah_*_flux.rs` found only commented Flux intent lines, no active attributes.
  - Exact planned command evidence fails: `raw-evidence/attempt3/flux-planned-with-lib.log` lines 1-5 reports `error: unexpected argument '--lib' found`.
  - `proof-writer-report.md` line 18 and line 46 preserve this as not substantively discharged / exact-command-incompatible.
- Required fix: either repair the planned Flux command and add executable Flux annotations, or update the plan via approved review/waiver. A compatible package-level pass over comments is not refinement proof.

### HIGH — FINDING-PROOF-004 — Behavior-affecting trusted-base rows still require independent approval and cannot be treated as discharged

- Obligations: PO-001 through PO-036.
- Artifacts: `.beads/vb-aoah/trusted-base-ledger.jsonl`; `.beads/vb-aoah/proof-evidence.md`.
- Evidence:
  - Trusted-base rows for all obligations retain `reviewer_disposition":"requires_independent_proof_review"` and `status":"RECORDED_REQUIRES_REVIEW"`; representative rows 1-36 show behavior-affecting bounded model trust boundaries.
  - `proof-evidence.md` lines 18-19 states the ledger was normalized but remains explicitly review-required.
  - These rows are model-bound constraints (`MAX_RECORDS=4`, `MAX_BYTES=16`) that cannot stand as final proof approval for production behavior until reviewed with production bridge evidence.
- Required fix: obtain independent review disposition for each behavior-affecting trust boundary after production binding/bridge evidence exists, or narrow/waive with explicit approved rationale.

## Positive evidence that is insufficient for approval

- TLA+ bounded logs exist and pass: grep over `raw-evidence/attempt3/tla-vb_aoah_*.log` found all five logs report `Model checking completed. No error has been found` at line 19.
- The representative TLA+ model is bounded and includes TypeOK/error transitions (`verification/tla/vb_aoah_runtime_open_no_side_effects.tla` lines 8-17, 51-54, 94-106), so no new TLA+ unbounded-Nat blocker is raised here.
- State 5 validator shape/provenance passed (`state5-official-validator.log` line 1), but that validator pass does not override proof-review substance.

## Review evidence read/inspected

- `.beads/vb-aoah/contract.md`
- `.beads/vb-aoah/proof-strategy.md`
- `.beads/vb-aoah/proof-obligations.planned.jsonl`
- `.beads/vb-aoah/verifier-lane-decisions.jsonl`
- `.beads/vb-aoah/proof-evidence.md`
- `.beads/vb-aoah/proof-writer-report.md`
- `.beads/vb-aoah/trusted-base-ledger.jsonl`
- `.beads/vb-aoah/agent-invocation-ledger.jsonl`
- Representative proof/harness/test/model artifacts and raw logs listed in findings.

STATUS: REJECTED
