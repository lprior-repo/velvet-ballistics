# Assurance Bundle - vb-815l8

## Scope

- **Bead**: `vb-815l8` — Tests: replace tautological recovery fault-tolerance assertion (P1)
- **Goal**: Replace a tautological test assertion (`assert!(result.is_ok() || result.is_err())`) that silently passes for any hydration outcome with a typed-failure assertion (`assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...")`) that genuinely discriminates the runtime boundary contract.
- **Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8`
- **JJ change**: `xsylyyxu 4346f453 vb-815l8: p11-holzman-rust — replace tautological recovery assertion`
- **Production files**: untouched (`crates/vb_storage/src/recovery/types.rs`, `crates/vb_runtime/src/recovery.rs`)
- **Modified files**: `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs` (only)
- **Diff**: `1 file changed, 16 insertions(+), 4 deletions(-)`

## Requirement-to-Evidence Map

| Requirement | Contract Clause | Obligation | Evidence |
|---|---|---|---|
| Runtime frame hydration rejects every seed | C-1 | PO-001, PO-002, PO-003, PO-004 | `verification-ledger.jsonl` (4 rows, all PASS); `formal-verification-report.md` |
| Boundary seed validation is invariant, not permissive | C-2 | PO-002, PO-004 (no regression) | `verification-ledger.jsonl` PO-002 / PO-004; `formal-verification-report.md` |
| Test uses typed assertion, not tautological `is_ok() \|\| is_err()` | C-3 | PO-001 (targeted test) | `evidence/cargo_test_targeted_recovery_from_corrupt_snapshot.log` (1 passed); `formal-verification-report.md` |
| Import is added at lines 7-13 | C-4 | PO-001 (compile + test) | `evidence/cargo_test_targeted_recovery_from_corrupt_snapshot.log` (exit 0); `formal-verification-report.md` |

## Evidence Index

### Proof / Verification Artifacts

- `formal-verification-report.md` — STATUS: APPROVED. 4 of 4 cargo-test obligations PASS, 8 non-behavior waivers (verus/kani/flux/proptest/loom/miri/tla+/cargo-fuzz) all `not_applicable` per bead scope.
- `verification-ledger.jsonl` — 4 rows, all PASS.
  - PO-001: targeted test → 1 passed
  - PO-002: full integration_runtime_storage_fault_tolerance.rs → 18 passed
  - PO-003: `cargo test -p vb_runtime --lib recovery` → 13 passed (no regression)
  - PO-004: `cargo test -p vb_runtime --lib` → 1807 passed (no regression)
- `formal-waivers.jsonl` — 8 rows, all `not_applicable`, all `behavior_affecting: false`.
- `proof-plan-review.md` — dispositions each verifier-lane-decision row, signs off on the 2 selected lanes (cargo-test, source-lint) and the 8 non-applicable lanes, rejects any waiver candidate.
- `proof-strategy.md` — risk classification, lane selection, obligation summary, trusted-base plan.
- `proof-obligations.planned.jsonl` — 4 planned obligations (PO-001..PO-004), all closed PASS.
- `verifier-lane-decisions.jsonl` — 12 rows: 4 required (cargo-test x2, source-lint x2), 8 not_applicable (verus, kani, flux, proptest, loom, miri, tla+, cargo-fuzz).
- `waiver-candidates.jsonl` — 8 waiver candidates, all `not_applicability: true`, none `behavior_affecting`.
- `trusted-base-plan.md` — 8 trusted surfaces, all verified PASS.

### Test / Behavior Artifacts

- `implementation.md` — exact diff (import + comment cleanup + assertion replacement), Holzman Rust rules affected, production contract walk-through, exact commands run, performance-layer decision, residual risks.
- `evidence/cargo_test_targeted_recovery_from_corrupt_snapshot.log` — `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance recovery_from_corrupt_snapshot_sequence_is_detected` → 1 passed.
- `evidence/cargo_test_integration_runtime_storage_fault_tolerance.log` — `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance` → 18 passed.
- `evidence/cargo_test_vb_runtime_recovery.log` — `cargo +nightly test -p vb_runtime --lib recovery` → 13 passed, 0 failed, 1794 filtered out.
- `evidence/cargo_test_vb_runtime_lib.log` — `cargo +nightly test -p vb_runtime --lib` → 1807 passed.
- `evidence/cargo_check_workspace_tests.log` — `cargo +nightly check -p velvet-ballistics-workspace-tests --all-targets --all-features` → exit 0, `Finished dev profile`.

### Review Artifacts

- `contract.md` — 4 contract clauses (C-1..C-4), exact required edits, out-of-scope exclusions, acceptance criteria, downstream routing.
- `proof-plan-review.md` — proof-plan reviewer disposition.
- `proof-plan-findings.jsonl` — 3 findings (all addressed: 1 ACCEPTED, 2 noted).
- `verifier-lane-review.jsonl` — 12 verifier-lane dispositions.
- `black-hat-review.md` — STATUS: APPROVED. 10 adversarial probes, 0 blocking findings, 0 defects requiring reroute.
- `defects.md` — empty (no defects).

### Provenance Artifacts

- `STATE.md` — bead delivery state, current state 12 → 13 → 14, JJ workspace status.
- `agent-invocation-ledger.jsonl` — agent invocation hash chain.
- `routing-ledger.jsonl` — controller routing records.
- `runtime-skill-provenance.json` — go-skill v10.1.0, loaded_skill_name=go-skill.
- `baseline-report.md` — pre-flight baseline: workspace outside source checkout, parent rsvywymk 1d6c017f.
- `global-readiness-report.md` — pre-flight: GREEN for State 2 dispatch.

### Boundary / Domain / Hazard Artifacts

- `boundary-map.md` — bead-scoped boundary inventory.
- `codebase-map.md` — 7 sections, Q1..Q7, including the production contract walk-through.
- `domain-model.md` — bead-scoped domain model.
- `error-taxonomy.md` — RuntimeError unit variants and unit-tag dispatch.
- `hazard-analysis.md` — 6 hazards (H-001..H-006), all mitigated.
- `type-contracts.md` — type-level contracts for the changed surface.
- `workflow-model.md` — bead-scoped workflow model.

## Raw Gate Evidence

| Command | Exit | Raw Output Path | Test Summary |
|---|---|---|---|
| `cargo +nightly check -p velvet-ballistics-workspace-tests --all-targets --all-features` | 0 | `evidence/cargo_check_workspace_tests.log` | `Finished dev profile` |
| `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance recovery_from_corrupt_snapshot_sequence_is_detected` | 0 | `evidence/cargo_test_targeted_recovery_from_corrupt_snapshot.log` | 1 passed; 0 failed; 0 ignored; 17 filtered out |
| `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance` | 0 | `evidence/cargo_test_integration_runtime_storage_fault_tolerance.log` | 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out |
| `cargo +nightly test -p vb_runtime --lib recovery` | 0 | `evidence/cargo_test_vb_runtime_recovery.log` | 13 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out |
| `cargo +nightly test -p vb_runtime --lib` | 0 | `evidence/cargo_test_vb_runtime_lib.log` | 1807 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out |
| `cargo +nightly fmt -p velvet-ballistics-workspace-tests` | 0 | (recorded in `implementation.md` §142-148) | rustfmt reordered two `vb_runtime::…` imports (shorter path first); no semantic change |
| `jj diff --stat` | 0 | (recorded in `implementation.md` §143) | `1 file changed, 16 insertions(+), 4 deletions(-)` |
| `jj diff crates/vb_storage/src/recovery/types.rs crates/vb_runtime/src/recovery.rs` | 0 | (recorded in `implementation.md` §144) | empty (production code untouched) |
| `jj root` | 0 | (recorded in `implementation.md` §140) | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8` |

## Machine Gates Summary

- **All required cargo-test obligations PASS.**
- **All required source-lint sub-gates PASS on the touched file.**
- **Workspace-wide lint debt is pre-existing** (recorded as `BLOCK_GLOBAL` prerequisite repair, not introduced by this bead).
- **No production code mutations.**
- **No regressions** at workspace_tests, vb_runtime::recovery, or vb_runtime::lib.

## Anti-Laundering Self-Check

- [x] No `assume`, `axiom`, `admit`, `external_body` in any proof (no proof code at all).
- [x] No `cover!`-as-proof (no Kani harness).
- [x] No copied harness models without bridge row (no harness models at all).
- [x] No generic waivers; all 8 non-applicable lanes have concrete `non_applicability_evidence_refs`.
- [x] No Verus obligations; therefore no production-binding gate triggered.
- [x] No behavior-affecting waiver candidate (all 8 waivers are `behavior_affecting: false`).
- [x] No silent omission of demanded lane — every demanded lane (cargo-test, source-lint) has at least one obligation row; every non-demanded lane has an explicit `not_applicable` row.
- [x] Source refs use `path::symbol` form (e.g., `crates/vb_runtime/src/recovery.rs::reject_unsupported_live_frame_state`).
- [x] All raw command evidence exists on disk and matches the planned obligation text.
- [x] All PASS rows have `exit_code: 0`, existing workdir, existing raw log, existing evidence artifact.
- [x] All `mapping_status` rows are closed (no `planned` rows remaining in the verification ledger).
- [x] All trusted-base dispositions are PASS (no pending dispositions).
- [x] Every behavior-affecting proof obligation has a matching Rust source ref (no behavior-affecting obligations in this bead; all `behavior_affecting: false`).

## Decision

**APPROVED** — Bead is closure-ready for landing. All 4 cargo-test obligations PASS, all 8 non-behavior waivers are validated, all raw evidence is on disk, all source/test/harness refs exist on disk, no production code mutated, no regressions observed. See `final-evidence-decision.md` and `truth-serum-report.md` for the final disposition.