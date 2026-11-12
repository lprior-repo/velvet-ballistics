# Proof Writer Report: vb-qi37.1

State 5 attempt 3 repair after State 4 attempt 4 plan repair.

## Scope

- Skill: proof-writer repair.
- Workspace verified: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`.
- Source checkout write policy: `/home/lewis/src/velvet-ballistics` was not written.
- Edits made only to verification and bead evidence/state artifacts in the isolated workspace.
- No production source, tests, dependencies, CI config, or public API files were edited.

## Inputs Read

- `.beads/vb-qi37.1/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.1/proof-obligations.jsonl`
- `.beads/vb-qi37.1/proof-strategy.md`
- `.beads/vb-qi37.1/proof-plan-review-input.md`
- `.beads/vb-qi37.1/contract.md`
- `.beads/vb-qi37.1/verification-layers.md`
- `.beads/vb-qi37.1/traceability-matrix.jsonl`
- `.beads/vb-qi37.1/proof-review.md`
- `.beads/vb-qi37.1/proof-findings.jsonl`
- `.beads/vb-qi37.1/proof-repair-guide.md`
- `crates/vb_storage/src/recovery/recover.rs`
- `verification/verus/recovery_verification.rs`

## Artifacts Changed

- `verification/verus/recovery_verification.rs`
- `.beads/vb-qi37.1/proof-writer-report.md`
- `.beads/vb-qi37.1/proof-evidence.md`
- `.beads/vb-qi37.1/STATE.md`

## Repair Delta

- Replaced tautological `proof_typed_recovery_errors_are_decision_outputs` with typed decision/error enums and non-vacuous refinement proofs for `PO-016`.
- Added `spec_recover_frame_decision`, `spec_refine_recovery_error`, and `spec_runtime_decision` so `Err(SpecRecoveryError)` inputs refine to named runtime diagnostics and cannot become `Ok`.
- Added concrete typed-error preservation proofs for workflow-source mismatch, compiled-IR mismatch, and frame-dimension overflow decisions.
- Rescoped required digest proof algebra to `spec_verify_required_digests`, which covers only workflow-source and compiled-IR checks for `PO-017`, `PO-019`, and `PO-020`.
- Kept action ABI and policy digest algebra only under `spec_verify_optional_downstream_digests`, matching waived optional `PO-021` and `PO-022` rows from State 4 attempt 4.
- Preserved existing TLA+ artifacts and reran TLC with explicit `target/tmp` metadir evidence.

## Commands Run

- `pwd -P && test "$PWD" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1" && case "$PWD" in /home/lewis/src/velvet-ballistics|/home/lewis/src/velvet-ballistics/*) exit 1;; esac`: exit `0`; stdout `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`.
- `mkdir -p target/tmp && TMPDIR=target/tmp verus verification/verus/recovery_verification.rs`: exit `0`; `verification results:: 16 verified, 0 errors`.
- `TMPDIR=target/tmp tlc -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla`: exit non-zero; failed before semantic/model checking with `java.io.IOException: Disk quota exceeded` while resolving modules via `/tmp`.
- `TMPDIR=target/tmp tlc -metadir target/tmp/tlc-metadir -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla`: exit `0`; `Model checking completed. No error has been found`; `10740192 states generated`; `8405208 distinct states found`; depth `7`.
- `TMPDIR=target/tmp jq -c . .beads/vb-qi37.1/proof-obligations.planned.jsonl >/dev/null && TMPDIR=target/tmp jq -c . .beads/vb-qi37.1/proof-obligations.jsonl >/dev/null && TMPDIR=target/tmp jq -c . .beads/vb-qi37.1/traceability-matrix.jsonl >/dev/null && TMPDIR=target/tmp test -s .beads/vb-qi37.1/proof-strategy.md && TMPDIR=target/tmp test -s .beads/vb-qi37.1/proof-plan-review-input.md`: exit `0`.
- `sha256sum crates/vb_storage/src/recovery/recover.rs verification/verus/recovery_verification.rs .beads/vb-qi37.1/proof-obligations.planned.jsonl .beads/vb-qi37.1/contract.md .beads/vb-qi37.1/verification-layers.md`: exit `0`; digest values recorded in `proof-evidence.md`.

## Obligation Status

- `PO-001` through `PO-010`: `PASS_MODEL`, TLC exit `0` on `RecoveryHydration.tla/.cfg` with explicit `target/tmp/tlc-metadir`.
- `PO-023`, `PO-024`, `PO-026`: `PASS_MODEL`, TLC exit `0` on replay divergence, no recovery data, and terminal mismatch fail-closed states.
- `PO-011` through `PO-016`, `PO-019`, `PO-020`, `PO-027`, `PO-028`, `PO-029`: `PASS_MODEL`, Verus exit `0` on `recovery_verification.rs`.
- `PO-016`: `PASS_MODEL_NON_VACUOUS`; repaired proof has named typed error inputs, named runtime diagnostic refinements, and explicit not-`Ok` preservation for `Err` decisions.
- `PO-017`: `PASS_MODEL_SCOPED`; required digest scope matches production-visible workflow-source and compiled-IR checks in `verify_digests` lines 53-73.
- `PO-021`, `PO-022`: `WAIVED_OPTIONAL_DOWNSTREAM`; no State 5 blocker remains because State 4 attempt 4 marks these rows `required:false`, `status:waived`, owner_state `4`, with non-null waiver objects.
- `PO-018`, `PO-025`, `PO-030`, `PO-031`, `PO-032`: `NOT_RUN`, owned by later states per `proof-obligations.planned.jsonl`.
- `PO-033`, `PO-034`, `PO-035`, `PO-036`: unchanged waived rows from State 4 attempt 4.

## Digest Scope Evidence

- Production source `crates/vb_storage/src/recovery/recover.rs` lines 53-73 checks workflow source for `WorkflowSourceOnly | WorkflowAndIr | Full`, checks compiled IR for `WorkflowAndIr | Full`, and then returns `Ok(())`.
- No action ABI or policy digest input, lookup, or comparison path exists in that production function for this bead.
- Required State 5 proof evidence is therefore limited to workflow-source and compiled-IR mismatch detection; action ABI and policy digest checks remain explicit optional downstream waivers.

## Reviewer Guidance

- Review `proof_typed_recovery_errors_refine_to_runtime_errors`, `proof_typed_recovery_errors_cannot_succeed`, and the three `proof_recovery_decision_preserves_*_error` proofs for `PO-016` adequacy.
- Review `spec_verify_required_digests` and the workflow/IR mismatch proofs for repaired `PO-017`, `PO-019`, and `PO-020` scope parity.
- Do not treat this State 5 report as cargo test, integration, proptest, `moon ci`, Kani, Flux, Miri, Loom, fuzz, or dependency-audit evidence.

---

## State 5 Attempt 4 Direct PRE-004 Repair

Trigger: State 3/4 repair added direct `VERUS-PRE-004` and `PO-003A`; prior State 5 evidence only covered the same digest surface indirectly through `PO-017`, `PO-019`, and `PO-020`.

### Scope

- Workspace verified: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`.
- Source checkout write policy: `/home/lewis/src/velvet-ballistics` was not written.
- Production source, tests, dependencies, CI config, public API files, and source checkout files were not edited.
- Verification artifact edited: `verification/verus/recovery_verification.rs`.
- Evidence artifacts edited: `.beads/vb-qi37.1/proof-writer-report.md`, `.beads/vb-qi37.1/proof-evidence.md`, `.beads/vb-qi37.1/STATE.md`.

### Repair Delta

- Added direct obligation tag `PO-003A` to the Verus artifact header.
- Added `proof_required_digest_preconditions_by_level` to prove `spec_verify_required_digests` enforces workflow-source digest input for `WorkflowSourceOnly`, and workflow-source plus compiled-IR digest inputs for `WorkflowAndIr` and `Full`.
- Preserved existing optional downstream action ABI and policy algebra under `spec_verify_optional_downstream_digests`; it is not direct `PO-003A` evidence.

### Commands Run

- `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac`: exit `0`; stdout `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`.
- `mkdir -p target/tmp && TMPDIR=target/tmp verus verification/verus/recovery_verification.rs`: exit `0`; `verification results:: 17 verified, 0 errors`.
- `jq -c . .beads/vb-qi37.1/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/traceability-matrix.jsonl >/dev/null`: exit `0`.
- `test -s .beads/vb-qi37.1/proof-writer-report.md && test -s .beads/vb-qi37.1/proof-evidence.md && test -s .beads/vb-qi37.1/contract-verification-review.md && test -s .beads/vb-qi37.1/proof-strategy.md && test -s .beads/vb-qi37.1/proof-plan-review-input.md`: exit `0`.
- `sha256sum verification/verus/recovery_verification.rs crates/vb_storage/src/recovery/recover.rs .beads/vb-qi37.1/proof-obligations.jsonl .beads/vb-qi37.1/proof-obligations.planned.jsonl`: exit `0`; digest values recorded in `proof-evidence.md`.

### Obligation Status Update

- `PO-003A` / `VERUS-PRE-004`: `PASS_MODEL_DIRECT`, Verus exit `0` on `proof_required_digest_preconditions_by_level` and existing required digest spec functions.
- `PO-017`, `PO-019`, `PO-020`: still `PASS_MODEL_SCOPED`; same production-visible workflow-source and compiled-IR digest surface.
- `PO-021`, `PO-022`: `WAIVED_OPTIONAL_DOWNSTREAM`; State 3/4 repaired rows are `required:false`, `status:"planned"`, with non-null waiver objects and promotion triggers.
- TLA+ was not rerun for attempt 4 because direct PRE-004/PO-003A is a Verus-only obligation and the previously recorded TLC evidence remains unchanged by this Verus artifact repair.

### Reviewer Guidance Addendum

- Review `proof_required_digest_preconditions_by_level` against `PO-003A` before consuming this repaired State 5 output downstream.
- Confirm the direct PRE-004 evidence does not claim action ABI or policy digest verification for vb-qi37.1.
