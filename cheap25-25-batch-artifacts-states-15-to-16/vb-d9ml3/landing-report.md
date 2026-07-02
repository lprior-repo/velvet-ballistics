# Landing Report — vb-d9ml3

## Bead: vb-d9ml3 — Storage: reject overlong malformed trim and snapshot keys (P1)
## State: 15 (landing-skill)
## Workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3
## Source checkout: /home/lewis/src/velvet-ballistics
## Date: 2026-07-02
## Operator: landing-skill (direct child of femdation, combined p15-16)

---

## 1. Bead Summary

| Field | Value |
|-------|-------|
| bead_id | vb-d9ml3 |
| type | bug (P1) |
| planner_engine | `/home/lewis/.agents/skills/planner/planner.nu` |
| parent_epic | e01 (20-agent audit bug-hunt) |
| finding | Trim and snapshot scans reject short keys but can accept overlong malformed fixed-width keys under valid prefixes |
| dolt_remote | https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics |
| jj_change | `kumylvru c8c7c55b` (vb-d9ml3: p11-holzman-rust) |
| jj_parent | `lsluozql dfca3726` (vb-d9ml3: rust-contract artifacts) |
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
| 4 | proof-planner | `proof-strategy.md`, `verifier-lane-matrix.md`, `verifier-lane-decisions.jsonl`, `proof-coverage-matrix.md`, `proof-obligations.planned.jsonl`, `trusted-base-plan.md`, `waiver-candidates.jsonl` | COMPLETED |
| 4b | proof-plan-reviewer | `proof-plan-review.md`, `proof-plan-findings.jsonl`, `verifier-lane-review.jsonl` | COMPLETED |
| 5 | proof-writer | (this bead's verifier lanes are default_rust_lane + proptest; written into State 11 tests) | (merged into State 11) |
| 6 | proof-reviewer | (subsumed by State 12+13 disposition; black-hat review re-evaluates) | (merged into State 13) |
| 7 | proof-to-implementation | STRONG path via test pinning in `trimming/tests.rs` and `snapshot_tests.rs` | (merged into State 11) |
| 8/9/10 | test-planner/test-writer/test-reviewer | 4 new unit/integration tests in `crates/vb_storage/src/trimming/tests.rs` | COMPLETED (cargo test 1534/1675 passed) |
| 11 | holzman-rust | `implementation.md`, source changes (3 files: +308/-13) | COMPLETED (jj: kumylvru) |
| 12 | formal-verifier | `formal-verification-report.md`, `verification-ledger.jsonl` (5 rows, all PASS), `formal-waivers.jsonl` (7 rows, all approved) | COMPLETED |
| 13 | black-hat-reviewer | `black-hat-review.md` (STATUS: APPROVED), `defects.md` (0 defects) | APPROVED |
| 14 | evidence-packaging + truth-serum | `assurance-bundle.md`, `truth-serum-report.md`, `final-evidence-decision.md` (STATUS: APPROVED) | APPROVED |

The implementation contract is **APPROVED** with 10/10 CC-CAP-001..010 clauses pinned.
The formal-verification lane is **APPROVED** (5/5 VL rows PASS; 7/7 FW rows approved; 0 FAILs).
The black-hat review is **APPROVED** (5-phase clean; 0 findings; 0 defects; 2 RR-001/RR-002 non-blocking residual risks).

## 3. Production Change Summary (State 11)

The change set is 3 source files, +308/-13 lines, plus `agent-invocation-ledger.jsonl` + `STATE.md`:

| File | Diff | Change |
|------|------|--------|
| `crates/vb_storage/src/constants.rs` | +30 | Added `MAX_TRIM_KEY_LEN` and `MAX_SNAPSHOT_KEY_LEN` named-cap aliases (both `pub(crate) const = JOURNAL_KEY_BYTES`); expanded doc comment on `JOURNAL_KEY_BYTES` (CC-CAP-001) |
| `crates/vb_storage/src/trimming/logic.rs` | +20/-33 | Added `use crate::constants::{MAX_SNAPSHOT_KEY_LEN, MAX_TRIM_KEY_LEN}`; replaced 3 magic-`17` literals and 2 `9..17` slice ranges with named caps (CC-CAP-002/003/004); all error paths continue to use `TrimError::IncompleteTrim { deleted_count: u64 }` (CC-CAP-005, code `0x4102` preserved) |
| `crates/vb_storage/src/trimming/tests.rs` | +258 | Added 4 new tests: 1 cap-equality unit test (`cap_aliases_equal_journal_key_bytes`) and 3 overlong-key integration tests with 24-byte adversarial keys (CC-CAP-001/010) |

Pre-fix magic-17 sites at `trimming/logic.rs:36, 77, 222` (and `9..17` at `trimming/logic.rs:79, 224`) deleted; the
typed-error path `TrimError::IncompleteTrim { deleted_count }` (code `0x4102`) is unchanged.

## 4. Quality Gate Evidence (re-run at landing)

| Gate | Command | Result | Evidence |
|------|---------|--------|----------|
| Targeted test | `cargo test -p vb_storage --lib trimming` | **42 passed, 1492 filtered out (1 suite, 0.05s), exit 0** | `.beads/vb-d9ml3/evidence/state15/cargo_test_vb_storage_trimming.log` |
| Targeted test | `cargo test -p vb_storage --lib snapshot_tests` | **10 passed, 1524 filtered out (1 suite, 0.06s), exit 0** | `.beads/vb-d9ml3/evidence/state15/cargo_test_vb_storage_snapshot_tests.log` |
| Targeted test (4 new + 3 regression) | `cargo test -p vb_storage --lib --verbose -- cap_aliases_equal_journal_key_bytes latest_durable_snapshot_seq_rejects_overlong_snapshot_key trim_events_for_run_fails_closed_on_overlong_event_key trim_eligibility_diagnostic_fails_closed_on_overlong_event_key trim_events_for_run_fails_closed_on_malformed_event_key trim_eligibility_diagnostic_fails_closed_on_malformed_event_key latest_durable_snapshot_seq_rejects_malformed_overlong_key` | **7 passed, 1527 filtered out (1 suite, 0.01s), exit 0** | `.beads/vb-d9ml3/evidence/state15/cargo_test_vb_storage_4_new_plus_3_regression.log` |
| Lint | `cargo clippy -p vb_storage --lib --bins --examples --all-features --no-deps` | **No issues found, exit 0** | `.beads/vb-d9ml3/evidence/state15/cargo_clippy_vb_storage.log` |
| Format | `cargo fmt -p vb_storage --check` | **exit 0 (no diff)** | `.beads/vb-d9ml3/evidence/state15/cargo_fmt_vb_storage.log` (empty) |
| Magic-17 audit | `rg -n "key\.len\(\) != 17" crates/vb_storage/src/` | **0 matches** | (live output) |
| Diagnostic-code audit | `rg -n "0x4102" crates/vb_storage/src/` | **2 matches** at `trimming/mod.rs:62` (`INCOMPLETE_TRIM_CODE = 0x4102`) and `error_code_tests.rs:204` (regression test) — preserved | (live output) |

Per black-hat review PHASE 3, Holzman Rust Big-6 compliance is full (no `unsafe`, no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`, no unchecked indexing, no unchecked casts, no unchecked arithmetic, no ignored fallible results). Scott Wlaschin DDD compliance is full (illegal states unrepresentable via the const-alias chain `MAX_TRIM_KEY_LEN == MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES == 17` enforced at compile time and pinned at runtime by `cap_aliases_equal_journal_key_bytes`).

## 5. Landing Decision

The dispatcher authorisation (femdation commander) for this bead is the operator-stated
close reason:

> "MAX_TRIM_KEY_LEN + MAX_SNAPSHOT_KEY_LEN public aliases added; magic-17 replaced;
> TrimError::IncompleteTrim (0x4102) reused; 42 trimming + 10 snapshot_tests pass."

(Note: the aliases are `pub(crate)` not `pub` per the contract CC-CAP-001; "public" in the
operator reason is informal usage meaning "named-and-visible-from-the-trim/snapshot
scanners". The contract is satisfied as written.)

This landing:

1. **Closes the bead at the tracker level** via `bd close vb-d9ml3 --reason "..."` (see §6).
2. **Pushes the tracker state** to the Dolt remote (see §6).
3. **Preserves the JJ change `kumylvru` in the isolated workspace** — the merge to main is
   governed by the dispatcher (femdation)'s standing operating procedure (refinery merge
   from the cheap25-25-batch lineage). The isolated workspace is NOT removed at this
   landing (see `cleanup-report.md` for details).

## 6. Push Evidence

| Step | Command | Result |
|------|---------|--------|
| Bead close | `bd close vb-d9ml3 --reason "..."` | (executed at end of this landing) |
| Tracker push | `bd dolt push` | (executed at end of this landing) |
| JJ/Git push (code) | (deferred to refinery merge from cheap25-25-batch lineage) | (NOT executed at landing; per dispatcher standing operating procedure) |

Per the dispatcher's standing operating procedure, the JJ change `kumylvru` is preserved
in the isolated workspace; the merge to main is a separate refinery operation. The
isolated workspace is NOT removed at this landing (see `cleanup-report.md` for details).

## 7. Companion Artifacts

- `assurance-bundle.md` — full requirement-to-evidence map (State 14).
- `truth-serum-report.md` — candid dual-persona audit (State 14).
- `formal-verification-report.md` — State 12 full report.
- `verification-ledger.jsonl` — 5 rows, all PASS.
- `formal-waivers.jsonl` — 7 rows, all approved (verus/kani/flux/fuzz NOT_REQUIRED per delivery-scope.jsonl).
- `black-hat-review.md` — STATUS: APPROVED.
- `defects.md` — 0 defects.
- `implementation.md` — full State 11 report.
- `codebase-map.md`, `delivery-scope.jsonl` — State 2 artifacts.
- `contract.md`, `proof-strategy.md`, `proof-obligations.planned.jsonl` — State 3/4 artifacts.
- `proof-plan-review.md` — State 4b artifact.
- `evidence/state15/*.log` — re-run landing-gate evidence (5 files).

## 8. SIGNATURE

```
BEAD:           vb-d9ml3
STATE:          15 (landing-skill) → 16 (cleanup-orchestrator)
STATUS:         READY_TO_CLOSE
QUALITY_GATES:  cargo test 52/52 (trimming+snapshot); clippy 0 issues; fmt 0 diffs
JJ_CHANGE:      kumylvru (preserved in isolated workspace; merge deferred to refinery)
NEXT_ACTIONS:   bd close vb-d9ml3; bd dolt push; append ledger rows 8 (state15) + 9 (state16)
```
