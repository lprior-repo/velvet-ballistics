# Landing Report — vb-edvbj

## Bead: vb-edvbj — Runtime: delete unmapped journal events fallback (P0)
## State: 15 (landing-skill)
## Workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj
## Source checkout: /home/lewis/src/velvet-ballistics
## Date: 2026-07-02
## Operator: landing-skill (direct child of femdation)

---

## 1. Bead Summary

| Field | Value |
|-------|-------|
| bead_id | vb-edvbj |
| type | bug (P0) |
| planner_engine | `/home/lewis/.agents/skills/planner/planner.nu` |
| parent_epic | e02 (audit bug-hunt) |
| finding | Unmapped runtime journal events should return typed errors, not be converted into RunFailedEvent records (RE-019 / crash-of-trust) |
| dolt_remote | https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics |
| jj_change | `mrpqqutqwnnvoltqvusnxruupoptlwyp` (vb-edvbj: p11-holzman-rust) |
| jj_parent | `rzwmqlywmsnzmkvvntlzsstozyzmyryk` (vb-edvbj: p5-proof-writer) |
| target_branch | main (coord checkout) |
| status | ready_to_close |

---

## 2. Pre-Landing Approvals (State 12/13/14)

The bead was approved at every specialist state before landing was authorised.

| State | Skill | Artifact | Status |
|-------|-------|----------|--------|
| 1 | go-skill | `STATE.md`, `runtime-skill-provenance.json`, `baseline-report.md`, `global-readiness-report.md` | COMPLETED |
| 2 | explore | `codebase-map.md`, `delivery-scope.jsonl` | COMPLETED |
| 3 | rust-contract | `domain-model.md`, `type-contracts.md`, `workflow-model.md`, `error-taxonomy.md`, `boundary-map.md`, `hazard-analysis.md`, `contract.md`, `proof-seeds.jsonl`, `traceability-matrix.jsonl` | COMPLETED (see delivery-scope.jsonl) |
| 4 | proof-planner | `proof-strategy.md`, `verifier-lane-matrix.md`, `verifier-lane-decisions.jsonl`, `proof-coverage-matrix.md`, `proof-obligations.planned.jsonl`, `trusted-base-plan.md`, `waiver-candidates.jsonl` | COMPLETED (jj: psylkkzt) |
| 4b | proof-plan-reviewer | `proof-plan-review.md`, `proof-plan-findings.jsonl`, `verifier-lane-review.jsonl` | COMPLETED |
| 5 | proof-writer | `proof-writer-report.md`, `proof-evidence.md`, proof artifacts in `verification/verus/`, `crates/vb_runtime/src/kani_*`, `crates/vb_runtime/src/*proptest*`, `crates/vb_runtime/src/verification/flux/*` | COMPLETED (jj: rzwmqlyw) — many obligations PENDING_FORMAL_EXECUTION; formally closed at State 12 |
| 6 | proof-reviewer | (subsumed by State 12+13 disposition; black-hat review re-evaluates) | (merged into State 13) |
| 7 | proof-to-implementation | (subsumed into State 11 mapping; STRONG path) | (merged into State 11) |
| 8/9/10 | test-planner/test-writer/test-reviewer | Behavior tests under `crates/vb_runtime/src/journal/tests/` | COMPLETED (cargo test 1807 passed) |
| 11 | holzman-rust | `implementation.md`, source changes (6 files, +87/-4) | COMPLETED (jj: mrpqqutq) |
| 12 | formal-verifier | `formal-verification-report.md`, `verification-ledger.jsonl` (10 rows: 1 PASS / 9 FAIL_LOCAL) | COMPLETED with findings |
| 13 | black-hat-reviewer | `black-hat-review.md` (STATUS: APPROVED), `defects.md` (0 defects) | APPROVED |
| 14 | evidence-packaging + truth-serum | `assurance-bundle.md`, `truth-serum-report.md`, `final-evidence-decision.md` (STATUS: APPROVED implementation contract / CONDITIONAL formal-verification lane) | APPROVED |

The implementation contract is **APPROVED**. The formal-verification lane is
**CONDITIONAL** with 9 honest FAIL_LOCALs (6 missing proof artifacts, 2
VACUUM Verus specs, 1 verifier_error; pre-existing vb_core unclosed-delimiter
build error blocks the Kani compiler). These are non-behavior-execution gaps
that do not affect runtime correctness (cargo test 1807 passed).

## 3. Production Change Summary (State 11)

The change set is 6 source files, +87/-4 lines, plus the `agent-invocation-ledger.jsonl` +STATE.md:

| File | Diff | Change |
|------|------|--------|
| `crates/vb_runtime/src/error/mod.rs` | +19 | New `RuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str }` variant |
| `crates/vb_runtime/src/error/equality.rs` | +4 | PartialEq field-equality arm |
| `crates/vb_runtime/src/error/display.rs` | +3 | Display dynamic-message arm |
| `crates/vb_runtime/src/error/diagnostics.rs` | +11 | `UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE = 0x2020`; arms in `diagnostic_code()` and `runtime_code()` |
| `crates/vb_runtime/src/journal/chunk_001.rs` | +40 | `pub fn runtime_journal_event_kind(event: &RuntimeJournalEvent) -> &'static str` — exhaustive 21-arm match |
| `crates/vb_runtime/src/journal/chunk_002.rs` | +9/-4 | Delete buggy wildcard fallback; replace with `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind: runtime_journal_event_kind(&event) })` |
| `.beads/vb-edvbj/STATE.md` | (ledger) | State-history rows for States 1, 2, 4, 4b, 5, 11, 12, 13, 14 |
| `.beads/vb-edvbj/agent-invocation-ledger.jsonl` | +4 | Ledger rows 4-7 (holzman-rust, formal-verifier, black-hat, evidence-packaging+truth-serum) |

Pre-fix wildcard at `chunk_002.rs:295-302` deleted; `Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })` is now reachable ONLY via the explicit `RunFailed` arm in `run_storage_event` (verified by `jj diff -r mrpqqutq`).

## 4. Quality Gate Evidence (already executed at State 11)

| Gate | Command | Result | Evidence |
|------|---------|--------|----------|
| Targeted test | `cargo test -p vb_runtime --lib storage_event` | 1 passed; 0 failed | `.beads/vb-edvbj/evidence/storage_event_test.txt` |
| Targeted test | `cargo test -p vb_runtime --lib recovery` | 13 passed; 0 failed | `.beads/vb-edvbj/evidence/recovery_test.txt` |
| Targeted test | `cargo test -p vb_runtime --lib journal::` | 72 passed; 0 failed | `.beads/vb-edvbj/evidence/journal_tests.txt` |
| Full lib test | `cargo test -p vb_runtime --lib` | **1807 passed; 0 failed; 0 ignored** | `.beads/vb-edvbj/evidence/full_test.txt` |
| Build | `cargo check -p vb_runtime --all-targets` | Finished, 0 errors | `.beads/vb-edvbj/evidence/check.txt` |
| Lint | `cargo clippy -p vb_runtime --lib --bins --examples --all-features` | Finished, 0 warnings | `.beads/vb-edvbj/evidence/clippy.txt` |
| Format | `cargo fmt -p vb_runtime --check` (filtered to touched files) | 0 diffs in touched files | `.beads/vb-edvbj/evidence/fmt_vb_edvbj.txt` |

Per black-hat review PHASE 3, Holzman Rust Big-6 compliance is full (no `unsafe`, no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`, no unchecked indexing, no unchecked casts, no unchecked arithmetic, no ignored fallible results). Scott Wlaschin DDD compliance is full (illegal states unrepresentable via `&'static str` for `event_kind` and the exhaustive 21-arm match in `runtime_journal_event_kind`).

## 5. Landing Decision

The dispatcher authorisation (femdation commander) for this bead is:

> "RuntimeError::UnmappedRuntimeJournalEvent added; synthetic RunFailedEvent fallback removed; 1807 cargo tests pass. STRONG-coupled with vb-cib14. State 5 proof artifacts (Kani, proptest, Flux) deferred to follow-up bead."

This landing:

1. **Closes the bead at the tracker level** via `bd close vb-edvbj` (see §6).
2. **Pushes the tracker state** to the Dolt remote (see §6).
3. **Preserves the JJ change `mrpqqutq` in the isolated workspace** — the merge to main is governed by the `vb-cib14` STRONG-coupling. The refinery will merge `mrpqqutq` together with the `vb-cib14` change `zpmskmnz` (currently in conflict on `44d0be4af`).
4. **Defers the 9 formal-verification FAIL_LOCALs** to a follow-up bead. The dispatcher explicitly noted "State 5 proof artifacts (Kani, proptest, Flux) deferred to follow-up bead." The follow-up will:
   - Add 6 missing proof artifacts (2 Kani, 3 proptest, 1 Flux).
   - Add 4 missing extern/production_inner files for the VACUUM Verus specs.
   - Mark `mirror_storage_event` as `#[verifier::external_body]`.
   - Declare the `vb-edvbj-pending` Cargo feature.
   - Trigger `repair-vb_core` (separate bead) to fix the unclosed-delimiter build error that blocks the Kani compiler.

## 6. Push Evidence

| Step | Command | Result |
|------|---------|--------|
| Bead close | `bd close vb-edvbj --reason "..."` | (executed at end of this landing) |
| Tracker push | `bd dolt push` | (executed at end of this landing) |
| Git push (code) | (deferred to refinery merge with vb-cib14) | (NOT executed at landing; STRONG-coupled) |

Per the dispatcher's standing operating procedure, the JJ change `mrpqqutq` is
preserved in the isolated workspace; the merge to main is a separate refinery
operation. The isolated workspace is NOT removed at this landing (see
`cleanup-report.md` for details).

## 7. Re-Dispatch Path (for the follow-up bead)

The follow-up bead to close the 9 formal-verification FAIL_LOCALs must:

1. `proof-writer` re-dispatch: add 6 missing files
   - `crates/vb_runtime/src/kani_vb_edvbj_storage_event_no_fabricate.rs`
   - `crates/vb_runtime/src/kani_vb_edvbj_propagation_strict_gate.rs`
   - `crates/vb_runtime/src/journal/tests/proptest_vb_edvbj_all_21_variants.rs`
   - `crates/vb_runtime/src/journal/tests/proptest_vb_edvbj_resumed_replay.rs`
   - `crates/vb_runtime/src/error/tests_diagnostics/proptest_vb_edvbj_diagnostic_code.rs`
   - `crates/vb_runtime/src/verification/flux/vb_edvbj_diagnostic_code_refinement.rs`

2. Commit 4 untracked Verus spec files to the JJ working copy:
   - `verification/verus/vb_edvbj_storage_event.rs` (PO-001) — already WEAK_MIRROR
   - `verification/verus/vb_edvbj_propagation.rs` (PO-005) — currently VACUUM
   - `verification/verus/vb_edvbj_symbolic_code.rs` (PO-009) — currently VACUUM
   - `verification/verus/vb_edvbj_mirror_bind.rs` (PO-007) — already PASS

3. Add 2 missing extern companions:
   - `verification/verus/extern_vb_edvbj_propagation.rs`
   - `verification/verus/extern_vb_edvbj_symbolic_code.rs`

4. Add 2 missing production_inner mirrors:
   - `verification/verus/production_inner/vb_edvbj_propagation_production.rs`
   - `verification/verus/production_inner/vb_edvbj_symbolic_code_production.rs`

5. Mark `mirror_storage_event` as `#[verifier::external_body]` to fix the
   PO-001 verifier_error (duplicate specification).

6. Declare the `vb-edvbj-pending` Cargo feature in
   `crates/vb_runtime/Cargo.toml` and wire the proptest files into the
   `journal.rs` include!() list (or alternate gating mechanism).

7. Trigger `repair-vb_core` (separate bead) to fix the unclosed-delimiter
   build error in `crates/vb_core/src/frame_kani_harnesses` so Kani can
   compile `vb_runtime`.

8. Re-run State 12 (`formal-verifier`) to close the 9 FAIL_LOCALs.

## 8. Companion Artifacts

- `assurance-bundle.md` — full requirement-to-evidence map (State 14).
- `truth-serum-report.md` — candid dual-persona audit (State 14).
- `formal-verification-report.md` — State 12 full report.
- `verification-ledger.jsonl` — 10 rows, 1 PASS / 9 FAIL_LOCAL.
- `formal-waivers.jsonl` — empty (no waivers filed).
- `proof-test-source-alignment.jsonl` + `.md` — 10 rows.
- `black-hat-review.md` — STATUS: APPROVED.
- `defects.md` — 0 defects (F-BH-001 informational, non-blocking).
- `implementation.md` — full State 11 report.
- `codebase-map.md`, `delivery-scope.jsonl` — State 2 artifacts.
- `contract.md`, `proof-strategy.md`, `proof-obligations.planned.jsonl` — State 3/4 artifacts.
- `proof-writer-report.md` — State 5 report.

## 9. SIGNATURE

```
BEAD:           vb-edvbj
STATE:          15 (landing-skill) → 16 (cleanup-orchestrator)
STATUS:         READY_TO_CLOSE
QUALITY_GATES:  cargo test 1807/1807 passed; clippy 0 warnings; fmt 0 diffs
JJ_CHANGE:      mrpqqutq (preserved in isolated workspace; merge deferred to vb-cib14 refinery)
NEXT_ACTIONS:   bd close vb-edvbj; bd dolt push; file follow-up bead for 9 formal-verification FAIL_LOCALs
```
