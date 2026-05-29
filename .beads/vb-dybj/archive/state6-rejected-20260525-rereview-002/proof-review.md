# Proof Review - vb-dybj State 6 Re-review

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-dybj-state6-rereview-002
bead_id: vb-dybj
state: 6
sublane: proof-rereview
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-dybj
source_checkout: /home/lewis/src/velvet-ballistics
reviewed_writer_invocation_id: proof-writer-vb-dybj-state5-004
prior_rejected_review_invocation_id: proof-reviewer-vb-dybj-state6-001

## Provenance

- Reviewed repaired State 5 attempt 4 artifacts: `.beads/vb-dybj/proof-writer-report.md`, `.beads/vb-dybj/proof-evidence.md`, `.beads/vb-dybj/trusted-base-ledger.jsonl`, `.beads/vb-dybj/proof-obligations.planned.jsonl`, `verification/verus/*`, `verification/flux/*`, `verification/tla/*`, Kani harnesses, property tests, and fuzz targets.
- Ledger check: `.beads/vb-dybj/agent-invocation-ledger.jsonl` records the current repaired writer as `proof-writer-vb-dybj-state5-004` at ledger sequence 11 with skill `proof-writer`. The stale State 6 reviewer was `proof-reviewer-vb-dybj-state6-001` at sequence 9. This re-review is a distinct `proof-reviewer` invocation; no self-approval detected.
- Reviewer commands executed in the isolated workdir:
  - `verus --version && verus verification/verus/vb_dybj_run_id_invariants.rs && verus verification/verus/vb_dybj_workflow_digest_invariants.rs && verus verification/verus/vb_dybj_record_kind_surface.rs` -> PASS, Verus `0.2026.05.05.d03e906`, `3 verified`, `2 verified`, `3 verified`.
  - `rtk cargo test -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests trailing_bytes -- --nocapture` -> FAIL. Raw log: `/home/lewis/.local/share/rtk/tee/1779739655_cargo_test.log`.
  - `cargo flux --manifest-path verification/flux/Cargo.toml` -> FAIL, unresolved `flux_rs` attributes.
  - `java -jar tools/tla2tools.jar -deadlock -workers 1 -config verification/tla/VbDybjGoldenFixtureLifecycle.cfg verification/tla/VbDybjGoldenFixtureLifecycle.tla` -> FAIL, missing `tools/tla2tools.jar`.
  - `cargo kani -p vb_storage --harness kani_vb_dybj_record_kind_surface_distinction --output-format regular` -> FAIL before harness verification due unrelated `cfg(kani)` compile errors in `crates/vb_storage/src/kani_recovery_hydrate.rs`.

## Findings

### CRITICAL - VB-DYBJ-PROOF-RR-001 - Required trailing-byte contract is false in executable evidence

- Obligations: PO-VB-DYBJ-013, PO-VB-DYBJ-014, PO-VB-DYBJ-015.
- Artifacts: `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`, `crates/workspace_tests/src/kani_vb_dybj_trailing_decode.rs`, `fuzz/fuzz_targets/vb_dybj_trailing_decode.rs`, `.beads/vb-dybj/proof-evidence.md`.
- Raw evidence: reviewer-run log `/home/lewis/.local/share/rtk/tee/1779739655_cargo_test.log` lines 21-23 and 56-58 shows `trailing_bytes_raw_workflow_digest_postcard_decode_rejects_nonempty_suffix` failed because `decoded.is_err()` was false for `WorkflowDigest([0;32])` plus suffix `[0]`. `.beads/vb-dybj/proof-evidence.md` lines 42-63 records the same production-contract blocker.
- Impact: The repaired artifacts now assert the intended exact-decode rejection property, but the actual current decode surface accepts at least one malformed trailing-byte input. This is not a proof gap; it is a counterexample to the claimed behavior.
- Required fix: Implementation/formal owner must provide an exact/no-trailing decode boundary or re-plan the contract with an approved waiver/migration. Do not weaken the proof harness back to `take_from_bytes` acceptance-with-remainder.

### CRITICAL - VB-DYBJ-PROOF-RR-002 - Verus PASS is standalone model evidence, not production-bound proof for planned obligations

- Obligations: PO-VB-DYBJ-001, PO-VB-DYBJ-004, PO-VB-DYBJ-007.
- Artifacts: `verification/verus/vb_dybj_run_id_invariants.rs`, `verification/verus/vb_dybj_workflow_digest_invariants.rs`, `verification/verus/vb_dybj_record_kind_surface.rs`, `.beads/vb-dybj/proof-obligations.planned.jsonl`, `.beads/vb-dybj/trusted-base-ledger.jsonl` TB-VB-DYBJ-001.
- Raw evidence: reviewer command verified the three standalone files successfully. However, planned commands require production-bound `--extern vb_core=target/verus/libvb_core.rlib` / `--extern vb_storage=target/verus/libvb_storage.rlib` (obligations lines 1, 4, and 7). The evidence command in `.beads/vb-dybj/proof-evidence.md` lines 5-21 omits those `--extern` bindings and explicitly states the files are standalone models mapped by comments because production rlibs were unavailable.
- Impact: Under the repository Formal Verification Mandates, a Verus model that mirrors the desired behavior in separate types/spec functions is not proof that the actual Rust implementation satisfies it. The PASS is useful model evidence, but not discharge of the planned production-bound Verus obligations.
- Required fix: Produce Verus evidence bound to production APIs as planned, or file an explicit approved waiver/bridge disposition that downgrades the claim and supplies compensating implementation evidence.

### CRITICAL - VB-DYBJ-PROOF-RR-003 - Required verifier lanes remain blocked without approved waivers or successful raw verifier output

- Obligations: PO-VB-DYBJ-005, PO-VB-DYBJ-008, PO-VB-DYBJ-010, PO-VB-DYBJ-012, PO-VB-DYBJ-013, PO-VB-DYBJ-015, PO-VB-DYBJ-016.
- Artifacts/evidence:
  - Flux PO-VB-DYBJ-005: `cargo flux --manifest-path verification/flux/Cargo.toml` fails with unresolved `flux_rs`; reviewer reproduced this failure.
  - Kani PO-VB-DYBJ-008/010: `cargo kani -p vb_storage --harness kani_vb_dybj_record_kind_surface_distinction --output-format regular` fails before the selected harness due `kani_recovery_hydrate.rs` compile errors; reviewer reproduced this failure.
  - Kani PO-VB-DYBJ-013: `.beads/vb-dybj/proof-evidence.md` lines 86-104 records timeout before proof closure.
  - TLA+ PO-VB-DYBJ-016: reviewer reproduced `Error: Unable to access jarfile tools/tla2tools.jar`.
  - cargo-fuzz PO-VB-DYBJ-012/015: `.beads/vb-dybj/proof-evidence.md` lines 145-163 records sanitizer/static-musl build failure.
- Impact: These are truthful blockers, not approvals. The trusted-base rows still have `reviewer_disposition: pending-proof-reviewer` and none is an approved waiver. Required behavior-affecting obligations cannot advance as proven.
- Required fix: Repair toolchain/compile blockers and run the planned commands with full raw output, or obtain explicit approved waivers with owner, scope, downgraded claim, and compensating evidence.

### HIGH - VB-DYBJ-PROOF-RR-004 - TLA+ model is syntactically plausible but unchecked and liveness/fairness is unproved

- Obligation: PO-VB-DYBJ-016.
- Artifacts: `verification/tla/VbDybjGoldenFixtureLifecycle.tla`, `verification/tla/VbDybjGoldenFixtureLifecycle.cfg`.
- Evidence: The model includes `TypeOK`, `NoSilentByteChangeAcceptance`, and `ChangedBytesNeedNamedMigration`, but TLC did not run because `tools/tla2tools.jar` is absent. No states generated/distinct states/diameter/deadlock result exists.
- Impact: The model cannot be credited as checked temporal evidence. `CHECK_DEADLOCK FALSE` is a stance, not a successful deadlock/lifecycle proof.
- Required fix: Run TLC with the planned config or provide an approved TLA tooling waiver and compensating evidence.

## Trust ledger disposition

- TB-VB-DYBJ-001 through TB-VB-DYBJ-007 are present, but all rows retain `reviewer_disposition: pending-proof-reviewer` and cannot substitute for successful verifier output or approved waivers.
- TB-VB-DYBJ-001 is accepted only as a description of a standalone model boundary, not as discharge of production-bound Verus obligations.
- TB-VB-DYBJ-003, TB-VB-DYBJ-005, and TB-VB-DYBJ-006 document tool blockers; they do not approve the Flux, fuzz, or TLA+ lanes.

## Verdict

Rejected. The repaired State 5 attempt 4 artifacts fixed several stale-review issues by adding real Verus syntax, strengthening RecordKind and trailing-byte harnesses, and truthfully surfacing blockers. They also expose a concrete counterexample to the required trailing-byte decode property. Required behavior-affecting obligations remain either false, standalone-only, or blocked without approved waivers.

STATUS: REJECTED
