# Implementation Evidence: vb-jpq7.3

## Code Changes Represented In This Evidence Set

- Replay uses `EventReplayLimit` and `events_for_run_bounded`.
- Snapshot-tail replay starts at `snapshot.seq + 1` and validates the first tail event exactly.
- Replay range scan begins at the first tail key instead of scanning/decoding pre-snapshot records.
- Latest durable snapshot lookup decodes snapshot payload and checks run/seq consistency before trusting key authority.
- Recovery tail slot writes return `RecoveryError::SlotTaintReadFailed` for taint read failures other than uninitialized slots.
- `FjallJournal::close()` and `persist_strict()` expose strict durability failures as `Result`.
- Test-only persist failure hook validates the strict close failure path.
- Added direct behavior test `apply_tail_events_fails_closed_when_taint_read_fails`.
- Added public hydration fail-closed behavior coverage and supplemental exact source assertions so `test-integrity` accepts the source-scan-to-behavior-test replacement.
- Added snapshot authority behavior tests for payload run mismatch, payload seq mismatch, payload digest mismatch, and postcard decode failure.
- Repaired global Moon CI blockers discovered during closure: runtime `VecDeque::push_back`, codegen parity `SlotCountMismatch` typed error, validation Gate 8 accessor depth, lifecycle queued-submit tests, Gate 11 isolated validation fixtures, source-length admission refactor, and sanitized tracked `tarpaulin-report.json` residue.
- Strengthened TLA+ recovery/replay model for bounded `EventSeq`, snapshot authority statuses, strict `snapshot.seq + 1`, `SequenceGap`, and overflow fail-closed behavior.
- Strengthened Verus replay seam contract for typed snapshot-authority errors and exact first-tail equality when non-empty.
- Added allocation-free Kani seams and harnesses for replay push limit classification, next-sequence overflow, recovery metadata validation, taint read resolution, and admission proof-core digest/claim invariants.
- Repaired proof-planning package shape: canonical `proof-obligation/v1`, `verifier-lane-decision/v1`, `waiver-candidate/v1`, `verification-ledger/v1`, and `traceability/v1` rows now map requirements to scoped evidence and limitations.
- Fixed full-journal recovery taint metadata handling: corrupt versioned slot-write taint envelope payloads now return `RecoveryError::CorruptSlotTaint { slot }`; absent envelope taint uses the legacy taint fallback.
- Repaired the follow-on schema-parity bug by introducing a versioned slot-write extra envelope that stores taint plus optional legacy frame extra bytes; legacy frame extra remains recoverable and is no longer misclassified as corrupt taint metadata.
- Strengthened the ignored-fallible-result scanner to catch embedded and split `.ok()` / `.err()` lossy conversions on recognized fallible sources, with fixtures for both shapes.
- Repaired production lossy fallible-result handling in CLI/UI decode paths and runtime journal taint sidecar encoding; storage event conversion now fails closed on encode errors.
- Repaired the cargo-vet supply-chain gate by adding scoped exemptions for the updated transitive dependency versions and formatting the vet store config.

## Power-of-Ten Rules Affected

- Rule 2 bounded loops/resources: replay collection has explicit `EventReplayLimit`; range start avoids unbounded pre-snapshot decode work.
- Rule 5 invariant density: snapshot key/payload consistency, sequence gap, taint-read failure, corrupt taint envelope payloads, and legacy frame-extra compatibility are explicit typed branches.
- Rule 7 checked returns: close/persist failures, snapshot lookup errors, full-journal corrupt taint metadata, and runtime journal sidecar encode failures are propagated as typed errors.
- Rule 10 zero warnings/static analysis: local compile/test/source scan/supply-chain pass and fresh canonical `moon ci` pass are recorded after the versioned slot-write extra envelope plus P0 taint/scanner/supply-chain edits.

## Performance-Layer Decision

No performance claim is made. The range-start change is reported as a correctness/boundedness improvement only. No benchmark/profiler numbers are attached; `WV-PERF-001` records this explicitly.

## Commands Run

See `verification-ledger.jsonl` for exact commands and pass/fail status.

## Residual Risks

- Kani is no longer blocked at inventory/compile for scoped vb-jpq7.3 seams; however Kani coverage remains scoped to allocation-free seams, not live Fjall or full `RunFrame` hydration.
- Strengthened TLA+/Verus/Kani artifacts still require independent proof-review acceptance.
- Fresh `moon ci` passed after the versioned slot-write extra envelope, P0 full-journal taint, scanner, runtime encode, and supply-chain repairs: `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`.
- No cargo-fuzz/Flux closure is claimed for this bead.
