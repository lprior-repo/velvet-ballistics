# Test Plan: vb-scxh State 7 Evidence-Audit Plan

## Summary

- Status: PLANNED.
- Scope: Truth Serum recovery/evidence integrity for false 12-bead closure and green CI recovery.
- Write scope for this State 7 artifact: `.beads/vb-scxh/test-plan.md` only.
- Forbidden scope: no production code, tests, proof artifacts, evidence reports, Red Queen runs, or writes under `/home/lewis/src/Velvet-ballistics`.
- Skill basis: `/home/lewis/.claude/skills/test-planner/SKILL.md` and `/home/lewis/.agents/skills/test-planner/SKILL.md` were read; they are equivalent. The `.agents` copy wins on conflict. Applied rules: do not write implementation/test code (lines 8-10), test behavior not methods (lines 12-23), BDD Given/When/Then (lines 75-95), proptest/fuzz/Kani planning (lines 96-142), mutation checkpoints (lines 143-155), and no `is_ok()`/`is_err()`-only assertions (lines 170-171). Also read `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md`: behavior contracts/public API (lines 5-10), acceptance tests as specification (lines 12-23), Testing Trophy integration emphasis (lines 32-50), real implementations/fakes over mocks (lines 54-67), and anti-patterns (lines 106-116).
- Upstream approvals required before this plan: `.beads/vb-scxh/proof-review.md` line 3 `STATUS: APPROVED`; `.beads/vb-scxh/contract-verification-review.md` line 3 `STATUS: APPROVED`.
- Behaviors identified: 12 evidence-audit behaviors.
- Trophy allocation: 0 unit / 10 integration-manual-audit / 2 final acceptance / 0 executable E2E. Deviation rationale: this is an artifact-only recovery bead with no Rust implementation target; State 8 must not create tests unless a later implementation target appears. State 11/12 manual/raw evidence audits are the executable acceptance layer.
- Proptest invariants: 0 active; waived by `WAIVE-PROPFUZZ-SCXH-001` because no parser/codec/classifier implementation exists.
- Fuzz targets: 0 active; same waiver.
- Kani harnesses: 0 active; waived by `WAIVE-KANI-SCXH-001` because no Rust code/harness target changes in `vb-scxh`.
- Mutation quality checkpoint: State 11 audit must preserve `FAIL_UNVIABLE` / `DEFERRED`; mutation adequacy is not PASS.

## 1. Behavior Inventory

1. Workspace guard accepts evidence capture only when `pwd -P` is exactly `/home/lewis/src/vb-scxh`.
2. Artifact presence audit rejects recovery when State 1/2 or referenced `vb-gvmt` evidence artifacts are missing/non-empty checks fail.
3. BD audit enumerates exactly 12 false-closure IDs and per-ID reopened/linked/follow-up evidence from raw BD output.
4. BD raw-source audit rejects stale prose for bead-state/link claims.
5. Safety anchor audit verifies bundle and bookmark/ref as primary raw evidence, or records `BLOCK_LOCAL`.
6. CI evidence audit accepts green CI only with raw `moon ci` markers: PASS, 19 completed tasks, 8276/8276 tests passed, runtime marker, and artifact path or fresh rerun.
7. Mutation audit classifies cargo-mutants evidence as `FAIL_UNVIABLE` / `DEFERRED`, never adequacy PASS.
8. Scope-control audit preserves generated parity deferral to `vb-gvmt` / `vb-qi37.10` and rejects `vb-scxh` closure proof conflation.
9. Laundering audit rejects subagent-only claims as acceptance evidence unless backed by distinct raw command/artifact evidence.
10. TLA path audit preserves canonical `.beads/vb-scxh/tla/ScxhRecovery.*` paths and rejects `.beads/vb-scxh/specs/` as authoritative unless fully moved/rerun.
11. Assurance/final decision blocks close/unblock unless all required raw lanes pass or have approved waivers.
12. Error trace audit maps each failed lane to the exact `Error::*` variant and preserves `BLOCK_LOCAL` for safety anchor failures.

## 2. Trophy Allocation

| Layer | Count | Behaviors | Rationale |
|---|---:|---|---|
| Static/ledger audit | 0 active | Waiver rows only | No production/static source target in scope. |
| Unit/calc | 0 active | None | No pure Rust function/classifier implementation exists. If later added, add unit/proptest/Kani before acceptance. |
| Integration/manual raw-evidence audit | 10 | 1-10 | Real BD, git, moon, artifact, and TLA evidence surfaces; no mocks/subagents. |
| Acceptance/final decision | 2 | 11-12 | Truth Serum/final evidence decision consumes State 11 artifacts and enforces close/unblock gate. |

## 3. Contract-to-Audit Traceability

| Contract / obligation | Target artifact | State | Exact command or audit action | Required assertion |
|---|---|---:|---|---|
| PRE-SCXH-001 / PATH-SCXH-001 | `.beads/vb-scxh/path-guard-report.md` | 11 | `pwd -P` | stdout exactly `/home/lewis/src/vb-scxh`; otherwise `Error::WrongWorkspace`. |
| INV-SCXH-003 / SCOPEWRITE-SCXH-001 | `.beads/vb-scxh/path-guard-report.md` | 11 | `git diff --name-only` | changed paths are only allowed `.beads/vb-scxh/` artifacts for the relevant state; no `/home/lewis/src/Velvet-ballistics` writes. |
| PRE-SCXH-002 / ART-SCXH-001 | `.beads/vb-scxh/artifact-presence-report.md` | 11 | `test -s .beads/vb-scxh/STATE.md && test -s .beads/vb-scxh/delivery-scope.jsonl && test -s .beads/vb-scxh/codebase-map.md && test -s .beads/vb-gvmt/moon-ci-or-static-scan-report.md && test -s .beads/vb-gvmt/formal-verification-report.md && test -s .beads/vb-gvmt/verification-ledger.jsonl && test -s .beads/vb-gvmt/parity-test-report.md && test -s .beads/vb-gvmt/mutation-report.md` | every listed input exists and is non-empty; missing input maps to `Error::MissingRecoveryInput`. |
| POST-SCXH-001 / BD-SCXH-001 | `.beads/vb-scxh/bd-closure-audit.md` | 11 | `bd --db /home/lewis/src/.beads/dolt show vb-scxh --json` | artifact records `EXACT_FALSE_CLOSURE_COUNT=12`, all 12 false-closure IDs, and per-ID raw reopened/linked/follow-up evidence. If IDs cannot be extracted from raw BD, report `BLOCKED` not inference. |
| PRE-SCXH-003 / BD-SCXH-002 | `.beads/vb-scxh/bd-closure-audit.md` | 11 | `bd --db /home/lewis/src/.beads/dolt show vb-scxh --json && bd --db /home/lewis/src/.beads/dolt list --json` | all bead-state/link claims quote raw BD JSON; prose-only claims are rejected as `Error::MissingRawEvidence`. |
| Per-ID false closure verification | `.beads/vb-scxh/bd-closure-audit.md` | 11 | `bd --db /home/lewis/src/.beads/dolt show <FALSE_CLOSURE_ID> --json` for each of the 12 IDs extracted from raw `vb-scxh` BD evidence | each ID has raw status plus reopen/link/follow-up evidence; missing count, ID, status, or link maps to `Error::FalseClosureUnverified`. |
| POST-SCXH-007 / SAFETY-SCXH-001 | `.beads/vb-scxh/safety-anchor-report.md` | 11 | `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle && git show-ref rescue-vb-scxh-ci-green-20260513T030158Z` | bundle verifies and bookmark/ref resolves; any bundle-open/ref failure is `failure_classification: BLOCK_LOCAL` and blocks State 12 close/unblock. |
| POST-SCXH-003 / CI-SCXH-001 | `.beads/vb-scxh/moon-ci-evidence-audit.md` | 11 | audit `.beads/vb-gvmt/moon-ci-or-static-scan-report.md` for command `moon ci`, PASS, 19 completed tasks, 8276/8276 passed, runtime marker, and artifact path; if stale/missing, rerun `moon ci` from `/home/lewis/src/vb-scxh` | no subagent narrative may replace raw CI markers. |
| POST-SCXH-004 / MUT-SCXH-001 | `.beads/vb-scxh/mutation-classification-audit.md` | 11 | audit `.beads/vb-gvmt/mutation-report.md` and `.beads/vb-gvmt/verification-ledger.jsonl` for `FAIL_UNVIABLE`, `DEFERRED`, and `35/35` unviable markers | mutation adequacy remains unsatisfied; PASS/adequacy claim maps to `Error::MutationMisclassified`. |
| POST-SCXH-005 / SCOPE-SCXH-001 | `.beads/vb-scxh/scope-control-audit.md` | 11 | `bd --db /home/lewis/src/.beads/dolt show vb-gvmt --json && bd --db /home/lewis/src/.beads/dolt show vb-qi37.10 --json` | generated parity remains deferred/owned by `vb-gvmt` / `vb-qi37.10`; not closure proof for `vb-scxh`; conflation maps to `Error::ScopeConflation`. |
| TLA-SCXH-005 / ERR-SCXH-010 | `.beads/vb-scxh/tla-report.md` | 8/11 audit only | `test -s .beads/vb-scxh/tla/ScxhRecovery.tla && test -s .beads/vb-scxh/tla/ScxhRecovery.cfg && tlc -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla` | canonical `.beads/vb-scxh/tla/` paths only; `.beads/vb-scxh/specs/` target mismatch maps to `Error::TlaPathMismatch`. |
| POST-SCXH-002 / INV-SCXH-001 / ERR-SCXH-004 | `.beads/vb-scxh/truth-serum-report.md` | 12 | review `.beads/vb-scxh/assurance-bundle.md` and all State 11 reports for `SUBAGENT_CLAIM` acceptance attempts | subagent-only acceptance is named, rejected, and mapped to `Error::LaunderedSubagentClaim` unless separate raw backing exists. |
| POST-SCXH-006 / TRUTH-SCXH-001 | `.beads/vb-scxh/final-evidence-decision.md` | 12 | review `.beads/vb-scxh/assurance-bundle.md` and write `.beads/vb-scxh/truth-serum-report.md` plus `.beads/vb-scxh/final-evidence-decision.md` | `APPROVE_CLOSE_OR_UNBLOCK` only if exact 12 BD IDs, safety anchor, CI raw evidence, mutation classification, scope deferral, TLA path-consistent proof, and waiver ledger pass/are approved; otherwise `BLOCKED`. |

## 4. BDD Scenarios

### Behavior: Workspace guard accepts only isolated workspace

Test/audit name: `workspace_guard_accepts_only_vb_scxh_when_capturing_evidence`

Given: State 11 evidence capture is invoked.
When: `pwd -P` runs from `/home/lewis/src/vb-scxh`.
Then: stdout is exactly `/home/lewis/src/vb-scxh`.
And: any other path maps to `Error::WrongWorkspace` and blocks acceptance.

### Behavior: Required recovery inputs are present

Test/audit name: `artifact_presence_blocks_when_required_input_is_missing`

Given: State 11 needs State 1/2 and referenced `vb-gvmt` inputs.
When: the exact chained `test -s` command in section 3 runs.
Then: every listed artifact is non-empty.
And: a missing artifact maps to `Error::MissingRecoveryInput`.

### Behavior: False closure set is exact and raw

Test/audit name: `bd_audit_records_exact_twelve_false_closures_when_raw_links_exist`

Given: `vb-scxh` recovery must prove false-closure repair.
When: raw BD JSON for `vb-scxh`, BD list, and each extracted false-closure ID are captured.
Then: `bd-closure-audit.md` contains `EXACT_FALSE_CLOSURE_COUNT=12` plus all 12 IDs and per-ID status/link/follow-up evidence.
And: truncated output, prose, or fewer/more IDs maps to `Error::FalseClosureUnverified`.

### Behavior: Safety anchor is a close/unblock blocker

Test/audit name: `safety_anchor_blocks_close_when_bundle_or_bookmark_fails`

Given: the safety bundle path and rescue bookmark/ref are required primary evidence.
When: `git bundle verify ... && git show-ref ...` runs.
Then: both commands must succeed for close/unblock approval.
And: any bundle-open/ref failure is `Error::SafetyAnchorMissing` with `failure_classification: BLOCK_LOCAL`.

### Behavior: CI evidence is raw and complete

Test/audit name: `ci_audit_accepts_green_ci_only_with_raw_moon_markers`

Given: green CI recovery is claimed.
When: `moon-ci-evidence-audit.md` inspects existing report or reruns `moon ci`.
Then: evidence contains command `moon ci`, PASS, 19 completed tasks, 8276/8276 tests passed, runtime marker, and artifact path/fresh output.
And: subagent narrative or stale prose maps to `Error::MissingRawEvidence`.

### Behavior: Mutation unviable is not adequacy pass

Test/audit name: `mutation_audit_rejects_fail_unviable_as_adequacy_pass`

Given: mutation evidence comes from `vb-gvmt` reports.
When: mutation audit reads mutation report and verification ledger.
Then: `FAIL_UNVIABLE` / `DEFERRED` and `35/35` unviable markers are preserved.
And: any adequacy PASS relabel maps to `Error::MutationMisclassified`.

### Behavior: Generated parity is deferred scope only

Test/audit name: `scope_control_rejects_generated_parity_as_vb_scxh_closure_proof`

Given: generated parity work belongs to `vb-gvmt` / `vb-qi37.10`.
When: scope-control audit captures raw BD for both beads.
Then: generated parity artifacts are scope-control inputs only.
And: use as `vb-scxh` closure proof maps to `Error::ScopeConflation`.

### Behavior: Subagent evidence cannot be laundered

Test/audit name: `truth_serum_rejects_subagent_only_acceptance_evidence`

Given: required evidence items may include narrative/subagent claims.
When: Truth Serum reviews the assurance bundle.
Then: every claim is classified as `RAW_COMMAND`, `ARTIFACT_DERIVED`, `SUBAGENT_CLAIM`, `DEFERRED`, or `BLOCKED`.
And: `SUBAGENT_CLAIM` cannot satisfy required acceptance evidence unless separate raw/artifact evidence exists.

### Behavior: Canonical TLA paths remain authoritative

Test/audit name: `tla_path_audit_rejects_specs_path_mismatch`

Given: State 5 proof artifacts are approved on `.beads/vb-scxh/tla/ScxhRecovery.*`.
When: TLA path audit reviews obligations/reports and optional TLC rerun.
Then: authoritative targets and commands use `.beads/vb-scxh/tla/`.
And: `.beads/vb-scxh/specs/` as active target maps to `Error::TlaPathMismatch` unless all paths are moved and rerun exactly.

### Behavior: Final decision blocks premature engine unblock

Test/audit name: `final_decision_blocks_unblock_until_all_required_lanes_pass`

Given: `vb-engine-yaml` remains blocked.
When: final evidence decision reviews State 11 reports.
Then: close/unblock approval requires all mandatory raw lanes pass or approved waivers.
And: premature unblock maps to `Error::BlockedEngineUnblock`.

## 5. Proptest Invariants

No active proptest target in `vb-scxh` State 7 because no parser/codec/classifier implementation is in scope. If a pure classifier is later introduced, add properties before closure:

- Evidence classification is total for every required evidence item.
- `SUBAGENT_CLAIM` never refines to accepted required evidence without a distinct raw/artifact record.
- `FAIL_UNVIABLE` never refines to mutation adequacy PASS.
- Generated parity deferral owner remains `vb-gvmt` or `vb-qi37.10`.

## 6. Fuzz Targets

No active fuzz target in this artifact-only bead. If later code parses BD JSON, moon reports, mutation ledgers, or assurance bundle markdown, fuzz those parsers with empty, truncated, malformed, duplicated-ID, and contradictory-status inputs.

## 7. Kani Harnesses

No active Kani harness for State 7. If a Rust evidence classifier/state machine is introduced, require bounded harnesses for:

- no accepted state from subagent-only evidence;
- no close/unblock when any required raw lane is missing/blocked;
- `FAIL_UNVIABLE` non-pass classification;
- exact-12 false-closure count gate.

## 8. Mutation Checkpoints

- Required State 11 classification: mutation evidence is `FAIL_UNVIABLE` / `DEFERRED`, not adequacy PASS.
- Target artifacts: `.beads/vb-scxh/mutation-classification-audit.md`, `.beads/vb-gvmt/mutation-report.md`, `.beads/vb-gvmt/verification-ledger.jsonl`.
- Critical mutants, if classifier code later exists:
  - Change `FAIL_UNVIABLE` to PASS must be killed by `mutation_audit_rejects_fail_unviable_as_adequacy_pass`.
  - Remove `DEFERRED` handling must be killed by mutation audit scenario.
  - Remove generated parity deferral guard must be killed by scope-control scenario.
  - Allow subagent-only evidence must be killed by laundering scenario.
- Threshold if code exists: >=90% mutation kill rate. Current bead cannot claim mutation adequacy because current mutation evidence is unviable/deferred.

## 9. Combinatorial Coverage Matrix

| Scenario | Input class | Expected output | Layer |
|---|---|---|---|
| workspace path valid | `pwd -P` == `/home/lewis/src/vb-scxh` | accept path guard | integration/manual |
| workspace path invalid | any other path | `Error::WrongWorkspace`, BLOCKED | integration/manual |
| artifacts all present | all listed `test -s` pass | accept input presence | integration/manual |
| artifact missing | any listed path absent/empty | `Error::MissingRecoveryInput`, BLOCKED | integration/manual |
| false closure exact | raw BD yields exactly 12 IDs with per-ID status/link | accept BD audit lane | integration/manual |
| false closure count wrong | <12, >12, duplicate, or truncated | `Error::FalseClosureUnverified`, BLOCKED | integration/manual |
| BD claim prose-only | no raw JSON marker | `Error::MissingRawEvidence`, BLOCKED | integration/manual |
| safety bundle/ref pass | bundle verifies and ref resolves | safety lane PASS | integration/manual |
| safety bundle/ref fail | bundle open fails or ref absent | `Error::SafetyAnchorMissing`, `BLOCK_LOCAL` | integration/manual |
| CI raw markers complete | `moon ci`, PASS, 19 tasks, 8276/8276, runtime, artifact path | CI lane PASS | integration/manual |
| CI markers missing/stale | missing command/status/count/runtime/path | `Error::MissingRawEvidence`, BLOCKED | integration/manual |
| mutation unviable | `FAIL_UNVIABLE` / `DEFERRED`, `35/35` unviable | mutation lane DEFERRED/non-pass | integration/manual |
| mutation mislabeled | unviable represented as PASS | `Error::MutationMisclassified`, BLOCKED | integration/manual |
| parity deferred | `vb-gvmt` / `vb-qi37.10` own follow-up | scope lane PASS | integration/manual |
| parity used for closure | generated parity used as `vb-scxh` proof | `Error::ScopeConflation`, BLOCKED | integration/manual |
| subagent-only evidence | required evidence item has only narrative | `Error::LaunderedSubagentClaim`, BLOCKED | acceptance |
| final all pass | all required raw lanes pass/approved waivers | `APPROVE_CLOSE_OR_UNBLOCK` permitted | acceptance |
| final any blocker | any required lane missing/blocked, especially safety anchor | `BLOCKED`, no close/unblock | acceptance |

## 10. State 8 / State 11 Execution Notes

- State 8 test-writing should not create Rust tests for this bead unless a new implementation target is explicitly added. This plan is satisfied by State 11/12 evidence-audit artifacts and exact commands.
- State 11 must capture command, workdir, exit status, stdout/stderr markers, and artifact path for each raw command lane.
- State 11 must not rely on truncated tool output. If a tool truncates, rerun/capture to a file-backed artifact and quote exact markers.
- State 12 must consume State 11 reports and block close/unblock if any required marker is absent, stale, subagent-only, or blocked.

## 11. Required Validation for This State 7 Plan

Run from `/home/lewis/src/vb-scxh`:

```text
pwd -P
test -s .beads/vb-scxh/test-plan.md
grep -n '^STATUS: APPROVED$' .beads/vb-scxh/proof-review.md
grep -n '^STATUS: APPROVED$' .beads/vb-scxh/contract-verification-review.md
```

Expected:

- `pwd -P` prints `/home/lewis/src/vb-scxh`.
- `test -s .beads/vb-scxh/test-plan.md` exits 0.
- both upstream approval grep commands find exactly the `STATUS: APPROVED` status line.

## Open Questions / Blockers for Later States

- State 7 does not know the exact 12 false-closure IDs independently of raw BD output. State 11 must derive and quote them from raw `bd --db /home/lewis/src/.beads/dolt ...` output; absence is `BLOCKED`, not inferred.
- Safety anchor is known to be a downstream close/unblock blocker if raw verification fails; no State 7 wording waives it.
