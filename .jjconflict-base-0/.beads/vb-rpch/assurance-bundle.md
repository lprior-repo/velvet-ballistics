# Assurance Bundle

bead_id: vb-rpch
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/femdation-vb-rpch
commit_or_change: vb-rpch-state13

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| PRE-001 hydrate_run_frame preconditions | PRE-001 | BDD tests + Kani blocked_tooling | proof-review APPROVED, contract-verif APPROVED | PARTIAL — tooling blocked |
| PRE-002 hydrate_run_frame_from_events preconditions | PRE-002 | BDD tests + Kani blocked_tooling | proof-review APPROVED, contract-verif APPROVED | PARTIAL — tooling blocked |
| PRE-003 check_workflow_source_digest | PRE-003 | BDD tests | proof-review APPROVED, contract-verif APPROVED | PASS |
| PRE-004 recover_runtime_summary/frame_seed | PRE-004 | BDD tests | proof-review APPROVED, contract-verif APPROVED | PASS |
| POST-001 workflow digest verification | POST-001 | TLA-REPLAY-001 waived sim 21k states | proof-review APPROVED | WAIVED |
| POST-002 IR digest verification | POST-002 | TLA-REPLAY-001 waived sim 21k states | proof-review APPROVED | WAIVED |
| POST-003 verify_digests GAP-3 | POST-003 | WAIVER-GAP3-ABI/POL approved | proof-review APPROVED | WAIVED |
| POST-004 recover_runtime_summary accuracy | POST-004 | BDD tests | proof-review APPROVED | PASS |
| POST-005 recover_runtime_frame_seed accuracy | POST-005 | BDD tests | proof-review APPROVED | PASS |
| POST-006 hydrate_run_frame slot/taint | POST-006 | BDD tests | proof-review APPROVED | PASS |
| POST-007 hydrate_run_frame_from_events | POST-007 | BDD tests | proof-review APPROVED | PASS |
| POST-008 recover_all_incomplete_runs | POST-008 | BDD tests + TLA-INCOMPLETE-001 waived | proof-review APPROVED | WAIVED |
| POST-009 replay_events non-idempotent blocking | POST-009 | BDD tests + Kani blocked_tooling | proof-review APPROVED | PARTIAL — tooling blocked |
| POST-010 ActionReplayTracker::is_resolved | POST-010 | BDD tests | proof-review APPROVED | PASS |
| INV-001 RecoveryError exhaustiveness | INV-001 | static-scan + BDD tests | proof-review APPROVED | PASS |
| INV-002 UnsupportedRecoveryState::union | INV-002 | Verus blocked_tooling | proof-review APPROVED | BLOCKED_TOOLING |
| INV-003 RecoveryFrameSeed dimensions | INV-003 | Verus blocked_tooling | proof-review APPROVED | BLOCKED_TOOLING |
| INV-004 ActionReplayTracker monotonicity | INV-004 | Verus blocked_tooling | proof-review APPROVED | BLOCKED_TOOLING |
| INV-005 DigestCheck hierarchy | INV-005 | Verus blocked_tooling | proof-review APPROVED | BLOCKED_TOOLING |
| INV-006 OnlyIncompleteRuns | INV-006 | TLA-INCOMPLETE-001 waived | proof-review APPROVED | WAIVED |
| ERR-TerminalStateMismatch | ERR-TerminalStateMismatch | WAIVER-TERM-MISMATCH approved | black-hat-review APPROVED | WAIVED |
| ERR-ActionAbiMismatch | ERR-ActionAbiMismatch | WAIVER-GAP3-ABI approved | proof-review APPROVED | WAIVED |
| ERR-PolicyDigestMismatch | ERR-PolicyDigestMismatch | WAIVER-GAP3-POL approved | proof-review APPROVED | WAIVED |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| TLA-REPLAY-001 | TLA+ TLC | cargo tla check | evidence/specs/RecoveryReplayFull.tla | WAIVED (state space) | yes |
| TLA-INCOMPLETE-001 | TLA+ TLC | cargo tla check | evidence/specs/RecoveryReplayFull.tla | WAIVED (state space) | yes |
| TLA-NONIDEM-001 | TLA+ TLC | cargo tla check | evidence/specs/RecoveryReplayFull.tla | WAIVED (state space) | yes |
| VERUS-INV-002 | Verus | cargo verus | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| VERUS-INV-003 | Verus | cargo verus | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| VERUS-INV-004 | Verus | cargo verus | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| VERUS-INV-005 | Verus | cargo verus | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| VERUS-PRE-001 | Verus | cargo verus | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| VERUS-PRE-002 | Verus | cargo verus | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| VERUS-POST-009 | Verus | cargo verus | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| KANI-PRE-001 | Kani | cargo kani | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| KANI-PRE-002 | Kani | cargo kani | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| KANI-POST-009 | Kani | cargo kani | vb_storage/src/recovery/ | BLOCKED_TOOLING | no |
| WAIVER-TERM-MISMATCH | waiver | formal-waivers.jsonl | .beads/vb-rpch/formal-waivers.jsonl | APPROVED | n/a |
| WAIVER-GAP3-ABI | waiver | proof-obligations.jsonl | proof-obligations.jsonl | APPROVED | n/a |
| WAIVER-GAP3-POL | waiver | proof-obligations.jsonl | proof-obligations.jsonl | APPROVED | n/a |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| BDD test suite | cargo test --package vb_storage --test recovery_bdd_tests | crates/vb_storage/tests/recovery_bdd_tests.rs | 70 PASS |
| Durability gate tests | cargo test --package vb_storage | crates/vb_storage/src/vb_2bok_durability_gate_tests.rs | PASS |
| Recovery unit tests | cargo test --package vb_storage | vb_storage/src/recovery/ | PASS |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| proof-review | .beads/vb-rpch/proof-review.md | APPROVED | TLA+ spec correct, 6 invariants in cfg, BuildSeqFromIndices fixed, 144k+ TLC states |
| contract-verification-review | .beads/vb-rpch/contract-verification-review.md | APPROVED | All 6 invariants declared in cfg, BuildSeqFromIndices type-correct |
| test-plan-review | .beads/vb-rpch/test-plan-review.md | REJECTED | 3 LETHAL findings + MAJOR gaps (pre-existing test density, proptest gaps) |
| black-hat-review | .beads/vb-rpch/black-hat-review.md | APPROVED | All 3 LETHAL findings properly addressed |
| formal-verification-report | .beads/vb-rpch/formal-verification-report.md | PRESENT | PARTIAL — tooling blocked, waivers approved, BDD 70 tests pass |

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| TerminalStateMismatch | No expected-terminal param in public API; DEFERRED_GLOBAL | vb-rpch | Tracked in vb-ty9 | Error type tested in vb_storage/src/recovery/tests.rs:1660-1665 |
| ActionAbiMismatch | GAP-3 not reachable via public API | vb-rpch | Tracked in vb-ty9 | Error type defined in RecoveryError enum |
| PolicyDigestMismatch | GAP-3 not reachable via public API | vb-rpch | Tracked in vb-ty9 | Error type defined in RecoveryError enum |
| TLA+ exhaustive model checking | State space explosion | vb-rpch | None | TLC simulation 21,404 states; all invariants pass |

## Truth Serum Audit

- report: `.beads/vb-rpch/truth-serum-report.md`
- status: UNVERIFIED — truth-serum skill not executable in this context

## Blocker

test-plan-review.md is REJECTED. Key issues (from attempt 12 rejection):
- LETHAL-1: Bare is_ok() with no frame validation — FIXED in state 13
- LETHAL-2: Test density 2.5x vs 5x required — FIXED (70 tests, 5x achieved)
- LETHAL-3: TerminalStateMismatch no formal waiver — FIXED (formal-waivers.jsonl created)
- MAJOR: Proptest invariants claimed (4) vs reality (0) — NOT FIXED
- MAJOR: Unit test inventory claimed (~47) vs reality (0) — NOT FIXED
- MAJOR: Assertion sharpness violations — PARTIALLY FIXED

Note: The formal-verification-report (state 13) claims 70 tests passing and all LETHALs fixed.
The test-plan-review.md (REJECTED) was written before the state 13 fixes were applied.
There is no later APPROVED test-plan-review.md on record.