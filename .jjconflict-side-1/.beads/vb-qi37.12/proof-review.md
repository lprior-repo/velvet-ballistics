# Proof Review: vb-qi37.12 State 6 Retry After State 5 Repair

STATUS: APPROVED

## Findings

No rejection findings remain for the repaired State 5 proof package.

## Review Scope

- Reviewed refreshed `proof-writer-report.md`, `proof-evidence.md`, `proof-execution-ledger.jsonl`, repaired `proof-obligations.jsonl`, repaired `proof-obligations.planned.jsonl`, `contract.md`, and `traceability-matrix.jsonl`.
- Review writes were limited to `proof-review.md`, `proof-findings.jsonl`, and `STATE.md`.
- No production code, test implementation, proof/model source, fuzz source, dependency file, CI config, source checkout file, or repair artifact was edited.

## Evidence Run

- Isolation check: `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12`; the path is not `/home/lewis/src/velvet-ballistics` and is not nested under it.
- Artifact gate: `test -s` passed for `STATE.md`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `proof-writer-report.md`, `proof-evidence.md`, `proof-execution-ledger.jsonl`, `contract.md`, `traceability-matrix.jsonl`, `silent-discard-scan-report.md`, and `proof/fuzz/persisted_payload_decode.md`.
- JSONL gate: `jq -c .` passed for `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `proof-execution-ledger.jsonl`, and `traceability-matrix.jsonl`.
- Schema repair gate: required rows in both obligation files produced no non-`planned` statuses; TLA rows produced no missing required metadata; `TEST-JOURNAL-007` and `TEST-RUNTIME-008` produced no generic `moon ci` command rows.
- TLA: `TMPDIR=target/tmp tlc -config .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla` exited 0 with `Model checking completed. No error has been found.`, 795 states generated, 280 distinct states found, 0 states left on queue, depth 8, and temporal properties checked.
- TLA vacuity check: scan found no explicit `Stutter` action and no `CHECK_DEADLOCK` directive in the repaired TLA/cfg files. `Next` uses concrete actions and stuttering is only from `[Next]_vars`.
- Verus: `TMPDIR=target/tmp verus .beads/vb-qi37.12/proof/verus/discard_classification.rs`, `diagnostic_envelope.rs`, and `recovery_decode_class.rs` each exited 0 with `verification results:: 1 verified, 0 errors`.
- Static scan: `.beads/vb-qi37.12/silent-discard-scan-report.md` records 690 raw candidates across 66 files, classified dispositions, and `Unclassified release-critical silent discards: 0`.
- Fuzz target discovery: `cargo fuzz list` included `vb_qi37_12_persisted_payload_decode`.
- Fuzz execution: `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp RUSTC_WRAPPER= CARGO_TARGET_DIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp/cargo-fuzz cargo fuzz run vb_qi37_12_persisted_payload_decode --target x86_64-unknown-linux-gnu -- -runs=1000` exited 0, built the release target, launched `-runs=1000`, and reported no crash artifact.
- Focused tests: `rtk cargo test -p vb_storage decode_rejects -- --nocapture` passed 36 tests; `rtk cargo test -p vb_storage process_lock -- --nocapture` passed 4 tests; `rtk cargo test -p vb_runtime diagnostic -- --nocapture` passed 10 tests.

## Obligation Decision

- `TLA-ACK-001`: APPROVED. TLC checked `NoAckAfterFailedRequiredPersist` and `PersistFailureEventuallyTypedError` on the repaired non-stuttering model.
- `TLA-REC-002`: APPROVED. TLC checked `CorruptionDoesNotHydrateEmptySuccess` and `RecoveryCorruptionEventuallyFailClosed`.
- `TLA-DEADLOCK-011`: APPROVED. The cfg does not disable deadlock checking, `Next` contains no explicit `Stutter`, and TLC completed with no deadlock error.
- `VERUS-CLS-003`, `VERUS-DIAG-004`, `VERUS-DEC-005`: APPROVED as abstract proof kernels with explicit production-linkage boundaries covered by static scan, fuzz, and focused tests.
- `SCAN-DISCARD-006`: APPROVED for State 6. The complete raw scan has a classified report with zero unclassified release-critical silent discards.
- `FUZZ-DECODE-009`: APPROVED for State 6. The named fuzz target is wired and executed for `-runs=1000` with typed corrupt/truncated decode oracles.
- `TEST-JOURNAL-007` and `TEST-RUNTIME-008`: APPROVED as focused State 6 supporting evidence. Full `moon ci` remains correctly deferred to State 11.
- `GATE-RELEASE-010`: NOT APPROVED HERE, non-blocking for State 6. Ownership remains State 11 formal-verifier/release gate per `proof-obligations.jsonl` and `proof-execution-ledger.jsonl`.

## Decision

The repaired State 5 proof package is approval-grade for State 6 proof review. Remaining release validation is explicitly downstream State 11 ownership, not a proof-review blocker.
