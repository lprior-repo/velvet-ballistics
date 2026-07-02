# Assurance Bundle — vb-jpq7 Wave 1 Proof

bead_id: `vb-jpq7-proof-wave1`  
source_checkout: `/home/lewis/src/vb-jpq7-wave1-proof`  
artifact_dir: `.beads/vb-jpq7-proof-wave1/`  
packaged_by: `evidence-packaging`  
packaged_at: `2026-05-23`  
decision_artifact: `.beads/vb-jpq7-proof-wave1/final-evidence-decision.md`

## Packaging Boundary

- No production code was edited by this packaging step.
- This bundle uses existing artifacts, raw command logs, ledgers, and review artifacts only.
- Kani remains `FAIL_GLOBAL` / blocked-global / non-required for this wave. No Kani PASS is claimed.
- Global source-length compile-file debt remains deferred global debt and is not treated as a local Wave 1 blocker; raw `moon ci` evidence records `DEFERRED_GLOBAL` rows for those files.
- Prompt-supplied reviewer pass statuses for final `test-reviewer`, `black-hat`, Holzman, timer Holzman/black-hat, and integrity reviewers are recorded as handoff context only because no corresponding canonical reviewer artifacts were present under `.beads/vb-jpq7-proof-wave1/`. They are not used as raw command evidence.

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| C1 Fjall strict durability | `contract.md:9-11` | `verification-ledger.jsonl:1,9,19-20`; `evidence/OBL-TLA-DUR-001.log`; `evidence/OBL-PROP-DUR-001.log`; `evidence/OBL-STATIC-NO-UNSAFE-001.log`; `evidence/OBL-CARGO-TEST-WAVE1-001.log` | `proof-plan-review.md:75`; `proof-review.md:51`; `proof-to-rust-review.md:48` | PASS |
| C2 Journaled durability | `contract.md:13-15` | `verification-ledger.jsonl:1,9,20`; `evidence/OBL-TLA-DUR-001.log`; `evidence/OBL-PROP-DUR-001.log`; cargo regression log | same as above | PASS |
| C3 Fail-closed replay parsing | `contract.md:17-19` | `verification-ledger.jsonl:2,10,18,20`; `evidence/OBL-TLA-REPLAY-001.log`; `evidence/OBL-PROP-REPLAY-001.log`; `evidence/OBL-FUZZ-JOURNAL-001.log` | same as above | PASS |
| C4 Bounded/streaming replay | `contract.md:21-23` | `verification-ledger.jsonl:2,8,10,17,20`; replay and Holzman TLA/proptest logs | same as above | PASS |
| C5 Replay sequence integrity | `contract.md:25-27` | `verification-ledger.jsonl:2,10,20`; replay TLA/proptest/cargo logs | same as above | PASS |
| C6 Taint corruption fails secret/not clean | `contract.md:29-31` | `verification-ledger.jsonl:3,11,20`; `evidence/OBL-TLA-TAINT-001.log`; `evidence/OBL-PROP-TAINT-001.log` | same as above | PASS |
| C7 Action queue boundedness | `contract.md:33-35` | `verification-ledger.jsonl:4,12,20`; `evidence/OBL-TLA-QUEUE-001.log`; `evidence/OBL-PROP-QUEUE-001.log` | same as above | PASS |
| C8 Timer boundedness | `contract.md:37-39` | `verification-ledger.jsonl:5,13,33-34`; `evidence/OBL-TLA-TIMER-001.log`; `evidence/OBL-PROP-TIMER-001.log`; `proof-to-rust-map.md:37,44` | `proof-to-rust-review.md:33-48`; proof reviews above | PASS |
| C9 Whole-workflow boundedness before persistence | `contract.md:41-43` | `verification-ledger.jsonl:6,14,30-32,35-36`; `evidence/OBL-TLA-ADMISSION-001.log`; `evidence/OBL-PROP-BUDGET-001.log`; runtime admission logs | same as above | PASS |
| C10 Cold exact diagnostics | `contract.md:45-47` | `verification-ledger.jsonl:15,20`; `evidence/OBL-PROP-DIAG-001.log`; cargo regression log | same as above | PASS |
| C11 Fjall operational safety | `contract.md:49-51` | `verification-ledger.jsonl:7,16,18,20`; `evidence/OBL-TLA-FJALL-001.log`; `evidence/OBL-PROP-FJALL-001.log`; fuzz/cargo logs | same as above | PASS |
| C12 Holzman boundedness | `contract.md:53-55` | `verification-ledger.jsonl:8,17,33-34,37-39`; `evidence/OBL-TLA-HOLZMAN-001.log`; `evidence/OBL-PROP-HOLZMAN-001.log`; strict static and `moon ci` logs | `proof-to-rust-review.md:40-48`; proof reviews above | PASS |
| Release tooling: test-integrity fallback | `proof-strategy.md:20`; `proof-coverage-matrix.md:6` | `verification-ledger.jsonl:29,39,41`; `evidence/OBL-TEST-INTEGRITY-FALLBACK-001.log`; `evidence/current-source-lightweight-required-checks-20260523T1644Z.log`; `evidence/current-source-rerun-wave1-freshness.log:595-599` | `formal-verification-report.md:82-99,108-112`; `proof-to-rust-review.md:33-38` | PASS |
| Current-source rerun/freshness | `proof-to-rust-map.md:44` | `verification-ledger.jsonl:39-41`; `evidence/current-source-rerun-wave1-freshness.log:914,1025-1028`; `evidence/current-source-lightweight-required-checks-20260523T1644Z.log:44-66`; bridge hash review | `proof-to-rust-review.md:24-38,48` | PASS |

## Proof Evidence

| Obligation | Tool/Lane | Command Evidence | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| `OBL-TLA-DUR-001` | TLA+ | Direct Java/TLC command recorded in log | `evidence/OBL-TLA-DUR-001.log` | PASS, exit 0 in ledger line 1 | None |
| `OBL-TLA-REPLAY-001` | TLA+ | Direct Java/TLC command recorded in log | `evidence/OBL-TLA-REPLAY-001.log` | PASS, exit 0 in ledger line 2 | None |
| `OBL-TLA-TAINT-001` | TLA+ | Direct Java/TLC command recorded in log | `evidence/OBL-TLA-TAINT-001.log` | PASS, exit 0 in ledger line 3 | None |
| `OBL-TLA-QUEUE-001` | TLA+ | Direct Java/TLC command recorded in log | `evidence/OBL-TLA-QUEUE-001.log` | PASS, exit 0 in ledger line 4 | None |
| `OBL-TLA-TIMER-001` | TLA+ | Direct Java/TLC command recorded in log | `evidence/OBL-TLA-TIMER-001.log` | PASS, exit 0 in ledger lines 5 and 33 | None |
| `OBL-TLA-ADMISSION-001` | TLA+ | Direct Java/TLC command recorded in log | `evidence/OBL-TLA-ADMISSION-001.log` | PASS, exit 0 in ledger lines 6 and 35 | None |
| `OBL-TLA-FJALL-001` | TLA+ | Direct Java/TLC command recorded in log | `evidence/OBL-TLA-FJALL-001.log` | PASS, exit 0 in ledger line 7 | None |
| `OBL-TLA-HOLZMAN-001` | TLA+ | Direct Java/TLC command recorded in log | `evidence/OBL-TLA-HOLZMAN-001.log` | PASS, exit 0 in ledger line 8 | None |
| `OBL-PROP-*` | Proptest/property tests | Cargo commands in individual logs | `evidence/OBL-PROP-*.log` | PASS, exit 0 in ledger lines 9-17 and 34/36 | None |
| `OBL-FUZZ-JOURNAL-001` | cargo-fuzz | Command in log | `evidence/OBL-FUZZ-JOURNAL-001.log` | PASS, exit 0 in ledger line 18 | None |
| `OBL-STATIC-NO-UNSAFE-001` | cargo clippy strict static gate | Command in `proof-to-rust-map.md:38`; log records exit 0 | `evidence/OBL-STATIC-NO-UNSAFE-001.log` | PASS, exit 0 in ledger lines 19 and 37 | None |
| `OBL-CARGO-TEST-WAVE1-001` | cargo test | Command in `proof-to-rust-map.md:39`; log records exit 0 | `evidence/OBL-CARGO-TEST-WAVE1-001.log` | PASS, exit 0 in ledger lines 20 and 38 | None |
| `OBL-KANI-*` | Kani | Not executed as accepted non-required global blocked lane | `proof-obligations.planned.jsonl`; `verification-ledger.jsonl:21-28` | `FAIL_GLOBAL`, no PASS claim | Approved blocked-global/non-required lane, not behavior waiver |
| `OBL-RUNTIME-ADMISSION-*` | cargo/rtk integration tests | Commands in `proof-to-rust-map.md:44`; logs under evidence dir | `evidence/OBL-RUNTIME-ADMISSION-*.log` | PASS, exit 0 in ledger lines 30-32 | None |
| `OBL-TEST-INTEGRITY-FALLBACK-001` | shell tooling self-test + gate | Four commands in log lines 6,45,51,57 | `evidence/OBL-TEST-INTEGRITY-FALLBACK-001.log` | PASS, exit 0 at lines 43,49,55,61 | None |
| `OBL-CURRENT-SOURCE-RERUN-WAVE1-001` | `moon ci` + focused lightweight rerun | `moon ci` in full log; lightweight commands in current-source log | `evidence/current-source-rerun-wave1-freshness.log`; `evidence/current-source-lightweight-required-checks-20260523T1644Z.log` | PASS, exit 0 in ledger lines 39 and 41 | None |

## Test / Gate Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| Current canonical CI | `moon ci` | `evidence/current-source-rerun-wave1-freshness.log` | PASS: 29 completed; 11531/11531 tests passed; `exit_status=0` at lines 914 and 1025-1028 |
| Test integrity fallback | `bash scripts/check-test-integrity.sh --self-test`; env/explicit/default invocations | `evidence/OBL-TEST-INTEGRITY-FALLBACK-001.log` | PASS: `base=workspace-fallback`; exit 0 for all four invocations |
| Post-bridge lightweight freshness | self-test, default test-integrity, `moon run velvet-ballistics:test-integrity` | `evidence/current-source-lightweight-required-checks-20260523T1644Z.log` | PASS: all `EXIT:0`; fallback active |
| Mutation smoke | Moon `mutants-smoke` task | `evidence/current-source-rerun-wave1-freshness.log:634,881-882` | PASS: 1 mutant tested, 1 caught |
| Miri smoke | Moon `miri` task | `evidence/current-source-rerun-wave1-freshness.log:866-870` | PASS: 1 passed, 0 failed |
| Coverage | Moon `coverage` task | `evidence/current-source-rerun-wave1-freshness.log:872-878` | PASS: report saved to `target/llvm-cov/lcov.info` |
| Doc tests | Moon `doc-test` task | `evidence/current-source-rerun-wave1-freshness.log:922-1023` | PASS: doctest task completed |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| proof-plan-reviewer | `.beads/vb-jpq7-proof-wave1/proof-plan-review.md` | `STATUS: APPROVED` at line 75 | Kani non-required/blocked; no waiver candidates; executable plan approved |
| proof-reviewer | `.beads/vb-jpq7-proof-wave1/proof-review.md` | `STATUS: APPROVED` at line 51 | Current raw evidence closes prior blockers; no Kani PASS claimed |
| proof-to-implementation / bridge review | `.beads/vb-jpq7-proof-wave1/proof-to-rust-review.md` | `STATUS: APPROVED` at line 48 | Bridge hashes rechecked; current-source PASS backed by executable logs |
| formal-verifier | `.beads/vb-jpq7-proof-wave1/formal-verification-report.md`; `verification-ledger.jsonl` | PASS rows in report/ledger | 20 executable Wave 1 obligations plus added runtime/test-integrity/current-source rows pass; Kani blocked-global remains |
| test-reviewer | prompt handoff only | PASS asserted by requester | No canonical `.beads/vb-jpq7-proof-wave1/test-plan-review.md` or equivalent current final test-reviewer artifact found; not counted as raw evidence |
| black-hat | prompt handoff only | PASS asserted by requester | No canonical `.beads/vb-jpq7-proof-wave1/black-hat-review.md` found; not counted as raw evidence |
| Holzman / timer Holzman-black-hat / integrity reviewers | prompt handoff only | PASS asserted by requester | No canonical final artifacts found under bead dir; raw Holzman coverage is via TLA/proptest/static/moon evidence, not reviewer prose |

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| Kani global lane | Approved plan marks Kani rows `required:false`, blocked tooling/global; no Kani PASS acceptable for this wave | Proof plan / verifier owners | Resolve in a future Kani-capable wave if Kani lane becomes required | TLA+, proptest, fuzz, strict static gate, cargo regression, runtime admission, `moon ci` |
| Source-length compile files | `moon ci` source-length task records `DEFERRED_GLOBAL` for compile files outside local Wave 1 blocker scope | Global architecture/code-health owner | Separate global debt bead/cleanup, not this proof-wave package | Current `moon ci` still completed 29 tasks and exited 0 |
| Missing canonical final reviewer artifacts in bead dir | Prompt requires inclusion of final reviewer PASS statuses, but only prompt handoff is available for test-reviewer/black-hat/Holzman/integrity reviewer statuses | Delivery coordinator | Attach or copy canonical reviewer artifacts before relying on them as raw evidence | This bundle relies on existing raw command logs and canonical proof/formal reviews for approval calculus |

## Truth Serum Audit

- report: `.beads/vb-jpq7-proof-wave1/truth-serum-report.md`
- status: `APPROVED`
