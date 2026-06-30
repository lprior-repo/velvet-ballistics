# STATE.md - vb-qi37.17.1 (cli: Add incident command)

## Bead Info
- **Bead ID**: vb-qi37.17.1
- **Title**: cli: Add incident command
- **Source Checkout**: /home/lewis/src/velvet-ballistics
- **Isolated Workspace**: /home/lewis/src/go-skill-vb-qi37.17.1
- **Workspace Type**: jj workspace (go-skill-vb-qi37.17.1)
- **Status**: claimed
- **Claimed By**: go-skill orchestrator
- **Claimed At**: 2026-05-17

## State Progress
- **Current State**: State 14 (in_progress)
- **Previous States** (properly sequenced):
  - State 1: COMPLETE — STATE.md + baseline-report.md, isolation verified
  - State 2: COMPLETE — codebase-map.md + delivery-scope.jsonl (explore agent corrected: 56 E0061 errors verified)
  - State 3: COMPLETE — contract.md + 4 supporting artifacts
  - State 4: COMPLETE — proof-strategy.md + proof-plan-review-input.md + proof-obligations.planned.jsonl
  - State 5: COMPLETE — proof-writer-report.md + proof-evidence.md (no formal proofs needed)
  - State 6: COMPLETE — proof-review.md APPROVED, contract-verification-review.md, proof-findings.jsonl
  - State 7: COMPLETE — test-plan.md written (test-planner)
  - State 8: COMPLETE — test-writer-report.md written (test-writer), 13 unit + 5 integration tests
  - State 9: COMPLETE — test-plan-review.md APPROVED + test-suite-review.md APPROVED (test-reviewer)
  - All 18 tests (13 unit + 5 integration) compile and pass
  - test-writer-report.md confirms all tests
- State 10: COMPLETE — holzman-rust implementation: 57 compile fixes + 4 unwrap fixes + dead code removal + 18 tests
- State 11: COMPLETE — machine gates STATUS: PASS, formal-verification STATUS: APPROVED
- State 12: COMPLETE — black-hat-review STATUS: APPROVED (v2, after DEFECT-001..004 resolved)
- State 13: COMPLETE — assurance-bundle.md + truth-serum-report.md + final-evidence-decision.md (STATUS: APPROVED)
- **Explore Agent Correction**: Explored agent hallucinated clean build. Verified: 56 E0061 errors across 8 crates. All are recover_full_journal / replay_events / replay_journal signature changes (5-arg and 3-arg variants).

## Isolation Proof
- Source: /home/lewis/src/velvet-ballistics
- Isolated: /home/lewis/src/go-skill-vb-qi37.17.1
- PASS: isolated != source and isolated is not nested under source

## Retry Budget
- Attempt: 0
- Failed Gate: none yet
- Repair Target: none yet

## Baseline
- Parent commit (origin/main): 19a8663f fix: resolve recovery BDD test and kani workflow conflicts
- moon ci baseline: green at 6090845a6 (19/19 completed, 7994/7994 tests)
- Current build: BLOCKED - 1 compile error in cmd_diff (unrelated to incident)
  - `recover_full_journal` call at app_impl.rs:2577 missing 2 digest slice args

## Scope Notes
- Incident command is already scaffolded: parse_incident, Command::Incident, cmd_incident, build_incident_report, build_repair_hints
- Acceptance criteria: incident returns structured failure evidence without stack traces; tests cover failed, missing, and non-failed runs
- Gaps: no dedicated incident command tests; pre-existing build blocker in cmd_diff
