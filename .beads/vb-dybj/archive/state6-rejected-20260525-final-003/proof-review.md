# Proof Review - vb-dybj State 6

reviewer_skill: proof-reviewer  
reviewer_invocation_id: proof-reviewer-vb-dybj-state6-003  
bead_id: vb-dybj  
state: 6  
sublane: proof-review  
attempt: 3  
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-dybj  
source_checkout: /home/lewis/src/velvet-ballistics  
reviewed_writer_invocation_id: proof-writer-vb-dybj-state5-005  

## Provenance

- Reviewed active State 5 PASS surface: `.beads/vb-dybj/proof-writer-report.md`, `.beads/vb-dybj/proof-evidence.md`, `.beads/vb-dybj/trusted-base-ledger.jsonl`, `.beads/vb-dybj/proof-obligations.planned.jsonl`, `.beads/vb-dybj/state5-ledger-repair-validation-evidence-attempt6.json`, and archived prior rejection `.beads/vb-dybj/archive/state6-rejected-20260525-rereview-002/*`.
- Ledger provenance: `.beads/vb-dybj/agent-invocation-ledger.jsonl` row 13 records the active writer as `proof-writer-vb-dybj-state5-005`; this review is a distinct `proof-reviewer` invocation. No self-approval detected.
- Official State 5 ledger-repair validation reports `status: PASS`, `rows_checked: 13`, and `proof_success_claimed: false`.

## Reviewer command evidence

- `verus verification/verus/vb_dybj_run_id_invariants.rs --crate-type lib --extern vb_core=target/verus/libvb_core.rlib && verus verification/verus/vb_dybj_workflow_digest_invariants.rs --crate-type lib --extern vb_core=target/verus/libvb_core.rlib && verus verification/verus/vb_dybj_record_kind_surface.rs --crate-type lib --extern vb_storage=target/verus/libvb_storage.rlib` -> PASS: `3 verified`, `2 verified`, `3 verified`.
- `cargo flux --manifest-path verification/flux/Cargo.toml` -> FAIL: unresolved `flux_rs` and missing Flux attributes.
- `cargo kani -p vb_storage --harness kani_vb_dybj_record_kind_surface_distinction --output-format regular` -> FAIL before selected harness verification: `could not compile vb_storage (lib) due to 65 previous errors` in unrelated `kani_recovery_hydrate.rs` cfg(kani) code.
- `rtk cargo test -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests trailing_bytes -- --nocapture` -> FAIL. Raw log `/home/lewis/.local/share/rtk/tee/1779745576_cargo_test.log` shows `WorkflowDigest([0;32])` plus suffix `[0]` decodes successfully where rejection is required.
- `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-dybj-state6-review-tlc -deadlock -workers 1 -config verification/tla/VbDybjGoldenFixtureLifecycle.cfg verification/tla/VbDybjGoldenFixtureLifecycle.tla` -> PASS: TLC 2.19, 52,165 states generated, 14,641 distinct states, depth 9.
- `cargo fuzz run vb_dybj_storage_short_decode --target x86_64-unknown-linux-gnu -- -max_total_time=10 -runs=1000` -> PASS: `#1000 DONE`, no crash.

## Findings

### CRITICAL - VB-DYBJ-PROOF-003-001 - Required trailing-byte rejection contract is false

- Obligations: PO-VB-DYBJ-013, PO-VB-DYBJ-014, PO-VB-DYBJ-015.
- Artifacts: `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`, `crates/workspace_tests/src/kani_vb_dybj_trailing_decode.rs`, `fuzz/fuzz_targets/vb_dybj_trailing_decode.rs`, `.beads/vb-dybj/proof-evidence.md`.
- Raw evidence: `/home/lewis/.local/share/rtk/tee/1779745576_cargo_test.log:17-58`; `.beads/vb-dybj/proof-evidence.md:82-107`; `.beads/vb-dybj/proof-writer-report.md:33-34,41`.
- Impact: the selected executable surface accepts a malformed nonempty trailing suffix. This is a counterexample, not an unproved gap.
- Required fix: implement/select an exact no-trailing decode boundary or re-plan the contract with an approved waiver/migration; do not weaken harnesses to prefix-accepting decode.

### CRITICAL - VB-DYBJ-PROOF-003-002 - Verus evidence remains standalone model evidence, not production-bound proof

- Obligations: PO-VB-DYBJ-001, PO-VB-DYBJ-004, PO-VB-DYBJ-007.
- Artifacts: `verification/verus/vb_dybj_run_id_invariants.rs`, `verification/verus/vb_dybj_workflow_digest_invariants.rs`, `verification/verus/vb_dybj_record_kind_surface.rs`, `.beads/vb-dybj/trusted-base-ledger.jsonl`.
- Raw evidence: reviewer Verus command passed with `--extern`, but the files define `RunIdModel`, `WorkflowDigestModel`, and `RecordKindModel` inside the proof artifacts (`verification/verus/vb_dybj_run_id_invariants.rs:12-32`, `verification/verus/vb_dybj_workflow_digest_invariants.rs:11-28`, `verification/verus/vb_dybj_record_kind_surface.rs:11-35`). `.beads/vb-dybj/proof-evidence.md:35` and `trusted-base-ledger.jsonl:1` explicitly admit the proof bodies are standalone mapped models and do not mechanically open production behavior.
- Impact: under the repository Formal Verification Mandates, comment/source anchors plus an unused `--extern` crate are not enough to discharge production-bound Verus obligations.
- Required fix: bind Verus specs/ensures to executable production functions or obtain an explicit approved waiver with downgraded claim and compensating executable evidence.

### CRITICAL - VB-DYBJ-PROOF-003-003 - Required Flux and vb_storage Kani lanes remain blocked without approved waivers

- Obligations: PO-VB-DYBJ-005, PO-VB-DYBJ-008, PO-VB-DYBJ-010.
- Artifacts: `verification/flux/vb_dybj_workflow_digest_shape.rs`, `verification/flux/Cargo.toml`, `crates/vb_storage/src/kani_vb_dybj_record_kind_surface.rs`, `crates/vb_storage/src/kani_vb_dybj_storage_short_decode.rs`, `.beads/vb-dybj/trusted-base-ledger.jsonl`.
- Raw evidence: reviewer `cargo flux --manifest-path verification/flux/Cargo.toml` failed with unresolved `flux_rs`; reviewer `cargo kani -p vb_storage --harness kani_vb_dybj_record_kind_surface_distinction --output-format regular` failed before harness verification with 65 compile errors. `.beads/vb-dybj/proof-evidence.md:60-80,109-130` records the same blockers. Trusted-base rows TB-VB-DYBJ-002/003/004 have `reviewer_disposition: pending-proof-reviewer`, not approved waiver status.
- Impact: required behavior-affecting lanes cannot be treated as discharged by truthful blocker documentation alone.
- Required fix: repair toolchain/compile isolation and rerun the planned verifier commands, or obtain explicit approved waivers with owner, scope, downgraded claim, and compensating evidence.

### HIGH - VB-DYBJ-PROOF-003-004 - Storage-short fuzz evidence is weaker than the planned obligation bound

- Obligation: PO-VB-DYBJ-012.
- Artifacts: `fuzz/fuzz_targets/vb_dybj_storage_short_decode.rs`, `.beads/vb-dybj/proof-obligations.planned.jsonl`, `.beads/vb-dybj/proof-evidence.md`.
- Raw evidence: planned obligation command requires `cargo fuzz run vb_dybj_storage_short_decode -- -max_total_time=60 -runs=10000` (`proof-obligations.planned.jsonl:12`); State 5 and reviewer evidence ran `--target x86_64-unknown-linux-gnu -- -max_total_time=10 -runs=1000` (`proof-evidence.md:152-165`).
- Impact: the smoke result is useful but does not meet the planned evidence bound.
- Required fix: execute the planned bound or amend the obligation through an approved plan/waiver before claiming discharge.

## Trust ledger disposition

- TB-VB-DYBJ-006 is acceptable for the bounded TLA+ reduction: reviewer reproduced TLC success with the same state/depth scale.
- TB-VB-DYBJ-005 is acceptable only for a 1000-run storage-short smoke and a trailing-decode counterexample; it is not approval of PO-VB-DYBJ-015 and does not meet the PO-VB-DYBJ-012 planned 10000-run bound.
- TB-VB-DYBJ-001 through TB-VB-DYBJ-004 and TB-VB-DYBJ-007 remain `pending-proof-reviewer` and cannot substitute for successful verifier output or approved waivers.

## Verdict

Rejected. The active State 5 PASS repaired provenance and truthfully stopped claiming proof success, but State 6 cannot approve proof quality while a required behavior contract is falsified, Verus claims remain standalone model proofs, and required Flux/Kani lanes remain blocked without approved waivers.

STATUS: REJECTED
