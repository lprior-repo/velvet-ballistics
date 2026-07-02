# Final Evidence Decision - vb-815l8

STATUS: APPROVED

## Decision

- **State 12 (formal-verifier): APPROVED.** All 4 cargo-test obligations PASS. All 8 non-behavior waivers validated. No production code mutated. No regressions.
- **State 13 (black-hat review): APPROVED.** All 10 adversarial probes returned no attack surface. 0 defects requiring reroute.
- **State 14 (assurance bundle): APPROVED.** Bead is closure-ready for landing.
- Bead is closure-ready for `landing` skill handoff.
- Do not merge to `main`; landing remains serialized by master.

## Required Raw Evidence

The following raw command outputs are on disk under `.beads/vb-815l8/evidence/` and are cited in the `verification-ledger.jsonl`:

| Command | Raw Output | Result |
|---|---|---|
| `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance recovery_from_corrupt_snapshot_sequence_is_detected` | `evidence/cargo_test_targeted_recovery_from_corrupt_snapshot.log` | 1 passed; 0 failed; 0 ignored; 17 filtered out |
| `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance` | `evidence/cargo_test_integration_runtime_storage_fault_tolerance.log` | 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out |
| `cargo +nightly test -p vb_runtime --lib recovery` | `evidence/cargo_test_vb_runtime_recovery.log` | 13 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out — **no regression** |
| `cargo +nightly test -p vb_runtime --lib` | `evidence/cargo_test_vb_runtime_lib.log` | 1807 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out — **no regression** |
| `cargo +nightly check -p velvet-ballistics-workspace-tests --all-targets --all-features` | `evidence/cargo_check_workspace_tests.log` | exit 0; `Finished dev profile` |
| `cargo +nightly fmt -p velvet-ballistics-workspace-tests` | (recorded in `implementation.md` §142-148) | exit 0; rustfmt reordered two `vb_runtime::…` imports (shorter path first); no semantic change |
| `jj diff --stat` | (recorded in `implementation.md` §143) | `1 file changed, 16 insertions(+), 4 deletions(-)` (test file only) |
| `jj diff crates/vb_storage/src/recovery/types.rs crates/vb_runtime/src/recovery.rs` | (recorded in `implementation.md` §144) | empty (production code untouched) |

## Closure Disposition

| Disposition | Count |
|---|---|
| PASS (cargo-test) | 4 |
| FAIL_LOCAL | 0 |
| FAIL_REGRESSION | 0 |
| FAIL_GLOBAL | 0 |
| WAIVED (non-behavior lane-not-applicable) | 8 |
| BLOCKED_TOOLING | 0 |
| BLOCKED_DEAD_CODE | 0 |
| Cover-only Kani | 0 |
| Commented-out tests | 0 |
| Ignored tests not run | 0 |
| **Behavior-affecting waivers** | **0** |

## Trusted-Base Dispositions

All 8 trusted surfaces from `trusted-base-plan.md` are verified PASS:

1. `cargo test` runner (nextest) — healthy.
2. `cargo +nightly check` for workspace_tests — exit 0.
3. `PartialEq for RuntimeError` via unit tag 10 — discriminates correctly.
4. `assert_eq!` macro (std) — healthy.
5. `RecoveryCannotResumeState::from_seed` (production, forbidden to mutate) — unchanged.
6. `DurableFrameRecoveryBoundary::hydrate_run_frame` (production, forbidden to mutate) — unchanged.
7. `RuntimeError::InvalidRecoveryHydration` (production, unit variant) — unchanged.
8. holzman-rust source lint pipeline — touched file is lint-clean.

## Mapping Status

- All `mapping_status` rows in `proof-obligations.planned.jsonl` are closed (no `planned` rows remaining in `verification-ledger.jsonl`).
- All source/test/harness refs cited in the planned obligations exist on disk and were inspected.
- All behavior-affecting proof obligations (none in this bead) have matching Rust source refs.
- All `trusted-base-plan.md` dispositions are PASS, none pending.
- All `verifier-lane-decisions.jsonl` rows have a final disposition (PASS for 4, `not_applicable`/`WAIVED` for 8).

## Residual Risks (Accepted)

1. **Workspace-wide fmt + test clippy debt** is pre-existing in the parent commit and explicitly classified `BLOCK_GLOBAL` prerequisite repair, not introduced by this bead. The touched test file is lint-clean.
2. **Test-name intent mismatch** (`recovery_from_corrupt_snapshot_sequence_is_detected` vs. body asserting boundary rejection) is flagged in `contract.md §5 Q1` as a P3 follow-up for `test-writer`, out of scope for this bead.
3. **Single-seed workspace_tests witness** covers only the happy-shape rejection path. Other typed-failure branches inside `hydrate_run_frame` are covered at the production crate level (the 8 canonical typed-failure sites). Out of scope for this bead.

## Bookmarks / Handoff

- **JJ change**: `xsylyyxu 4346f453 vb-815l8: p11-holzman-rust — replace tautological recovery assertion` (landed at state 11)
- **JJ workspace**: `cheap25-vb-815l8`
- **JJ workspace root**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8`
- **Parent commit**: `1015cf6e` (cheap25-vb-815l8 empty parent)
- **Upstream main**: `2c8ea33c9`
- **Handoff target**: `landing` skill (state 15)

## Verdict

**APPROVED** — Bead is closure-ready for landing. All 4 cargo-test obligations PASS, all 8 non-behavior waivers are validated, all raw evidence is on disk, all source/test/harness refs exist on disk, no production code mutated, no regressions observed, triple-locked contract holds. The pre-existing workspace-wide lint debt is disclosed as deferred global debt, not laundered as a pass. The P1 bug (tautological assertion) is fixed and cannot re-emerge without simultaneously breaking the canonical unit tests AND the workspace_tests witness AND the `PartialEq` unit-tag dispatch.