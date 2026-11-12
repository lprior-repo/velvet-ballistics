# State 8 Preliminary Audit Harness Output: vb-scxh

Generated UTC: 2026-05-14T19:47:18.935207+00:00

STATUS: RED_PRELIMINARY_NOT_STATE11_EVIDENCE

This file is failing-first scaffolding output only. It is not `.beads/vb-scxh/assurance-bundle.md`, not `.beads/vb-scxh/truth-serum-report.md`, and not `.beads/vb-scxh/final-evidence-decision.md`.

## Lane Results

### workspace_path

- Status: PASS_PRELIM
- Proof obligations: PATH-SCXH-001, SCOPEWRITE-SCXH-001, ERR-SCXH-001
- Command/check: `pwd -P`
- Expected: stdout exactly /home/lewis/src/vb-scxh
- Error mapping: none
- Actual/raw/prelim:

```text
exit=0
stdout='/home/lewis/src/vb-scxh\n'
stderr=''
```

### approved_inputs

- Status: PASS_PRELIM
- Proof obligations: ART-SCXH-001
- Command/check: `test -s approved inputs and grep STATUS: APPROVED`
- Expected: test-plan non-empty and both upstream reviews approved
- Error mapping: none
- Actual/raw/prelim:

```text
failures=[]
```

### artifact_presence

- Status: PASS_PRELIM
- Proof obligations: ART-SCXH-001, ERR-SCXH-002
- Command/check: `test -s .beads/vb-scxh/STATE.md && test -s .beads/vb-scxh/delivery-scope.jsonl && test -s .beads/vb-scxh/codebase-map.md && test -s .beads/vb-gvmt/moon-ci-or-static-scan-report.md && test -s .beads/vb-gvmt/formal-verification-report.md && test -s .beads/vb-gvmt/verification-ledger.jsonl && test -s .beads/vb-gvmt/parity-test-report.md && test -s .beads/vb-gvmt/mutation-report.md`
- Expected: all required State 1/2 and referenced vb-gvmt artifacts are non-empty
- Error mapping: none
- Actual/raw/prelim:

```text
missing_or_empty=[]
```

### bd_command_plan

- Status: NOT_RUN_STATE11_REQUIRED
- Proof obligations: BD-SCXH-001, BD-SCXH-002, ERR-SCXH-005
- Command/check: `bd --db /home/lewis/src/.beads/dolt show vb-scxh --json && bd --db /home/lewis/src/.beads/dolt list --json && per-ID bd show commands`
- Expected: exact 12 false-closure IDs and per-ID raw reopened/linked/follow-up evidence
- Error mapping: Error::FalseClosureUnverified or Error::MissingRawEvidence
- Actual/raw/prelim:

```text
State 8 scaffold only; raw capture intentionally deferred.
```

### safety_anchor_preflight

- Status: RED_PRELIM
- Proof obligations: SAFETY-SCXH-001, ERR-SCXH-006
- Command/check: `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle && git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`
- Expected: bundle verifies and rescue bookmark/ref resolves
- Error mapping: Error::SafetyAnchorMissing; failure_classification=BLOCK_LOCAL
- Actual/raw/prelim:

```text
exit=1
stdout=
stderr=error: could not open '/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle'
```

### moon_ci_marker_audit

- Status: RED_PRELIM
- Proof obligations: CI-SCXH-001, ERR-SCXH-003
- Command/check: `audit .beads/vb-gvmt/moon-ci-or-static-scan-report.md markers; require artifact path and fresh rerun marker before PASS_PRELIM; rerun moon ci in State 11 if stale/missing`
- Expected: raw command moon ci, PASS, 19 completed tasks, 8276/8276 passed, runtime marker, artifact path evidence, fresh rerun marker
- Error mapping: Error::MissingRawEvidence
- Actual/raw/prelim:

```text
missing_markers=['artifact path evidence marker', 'fresh rerun marker']
```

### mutation_marker_audit

- Status: RED_PRELIM
- Proof obligations: MUT-SCXH-001, TLA-SCXH-003, ERR-SCXH-007
- Command/check: `audit .beads/vb-gvmt/mutation-report.md and verification-ledger.jsonl`
- Expected: FAIL_UNVIABLE/DEFERRED preserved; 35/35 unviable; no adequacy PASS
- Error mapping: Error::MutationMisclassified
- Actual/raw/prelim:

```text
missing=['mutation-report missing 35/35 unviable', 'verification-ledger missing 35/35 unviable']
forbidden=[]
```

### scope_command_plan

- Status: NOT_RUN_STATE11_REQUIRED
- Proof obligations: SCOPE-SCXH-001, TLA-SCXH-004, ERR-SCXH-008
- Command/check: `bd --db /home/lewis/src/.beads/dolt show vb-gvmt --json && bd --db /home/lewis/src/.beads/dolt show vb-qi37.10 --json`
- Expected: generated parity remains deferred/owned by vb-gvmt or vb-qi37.10
- Error mapping: Error::ScopeConflation
- Actual/raw/prelim:

```text
State 8 scaffold only; raw capture intentionally deferred.
```

### laundering_negative_fixture

- Status: NOT_RUN_STATE11_REQUIRED
- Proof obligations: TRUTH-SCXH-001, TLA-SCXH-002, ERR-SCXH-004
- Command/check: `State 12 review of assurance-bundle classifications`
- Expected: SUBAGENT_CLAIM without distinct raw backing is rejected/blocked
- Error mapping: Error::LaunderedSubagentClaim
- Actual/raw/prelim:

```text
State 8 scaffold only; raw capture intentionally deferred.
```

### tla_path_preflight

- Status: PASS_PRELIM
- Proof obligations: TLA-SCXH-005, ERR-SCXH-010
- Command/check: `test -s .beads/vb-scxh/tla/ScxhRecovery.tla && test -s .beads/vb-scxh/tla/ScxhRecovery.cfg; audit obligation paths`
- Expected: canonical .beads/vb-scxh/tla paths exist and no active specs/ model/config target remains
- Error mapping: none
- Actual/raw/prelim:

```text
missing=[]
active_specs_targets=[]
```

### final_gate_negative_fixture

- Status: NOT_RUN_STATE11_REQUIRED
- Proof obligations: TRUTH-SCXH-001, TLA-SCXH-001, ERR-SCXH-009
- Command/check: `State 12 final-evidence-decision review after State 11 reports exist`
- Expected: APPROVE_CLOSE_OR_UNBLOCK forbidden while any required lane is missing/blocked
- Error mapping: Error::BlockedEngineUnblock
- Actual/raw/prelim:

```text
State 8 scaffold only; raw capture intentionally deferred.
```
