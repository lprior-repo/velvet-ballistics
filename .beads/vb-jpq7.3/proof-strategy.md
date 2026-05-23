# Proof Strategy: vb-jpq7.3 Fail-Closed Storage/Recovery Durability

## Scope

Prove and test that storage/recovery durability paths fail closed rather than silently laundering missing/corrupt state.

## Risk Lanes

1. **TLA+ temporal lane**: crash/restart and recovery lifecycle must reach hydrated success only from complete durable evidence, otherwise fail closed. Current candidate: `verification/tla/EngineYamlRecovery.tla`.
2. **Verus replay/recovery lane**: local proof obligations for contiguous event replay, strict snapshot-tail boundary, typed error totality, taint exactness, and dimension boundedness. Current candidate artifacts are partial and require reviewer scrutiny because prior `vb_jpq724_events_for_run_production.rs` was rejected as too abstract.
3. **Behavior tests**: exact typed outcomes for replay bounds, snapshot-tail gaps, corrupt latest snapshot authority, corrupt pre-snapshot range skipping, explicit close/persist result, and taint read fail-closed behavior.
4. **Static source scans**: no lossy `.ok()`, `let _` discard, or hidden fallible-result suppression in runtime/storage/compiler production paths, except audited discard API.
5. **Global readiness**: moon/cargo format/lint/test readiness remains a release gate; current rustfmt failure is `BLOCK_GLOBAL`.

## Proof Obligations Summary

- `POT-REPLAY-001`: replay start after snapshot is `snapshot.seq + 1`; the first tail event must exactly match that sequence or return `JournalError::SequenceGap`.
- `POT-REPLAY-002`: replay bound limits actual collected/decode tail events and returns `JournalError::TooManyEvents` before unbounded growth.
- `POT-REPLAY-003`: lower-bound key range starts at the first needed tail key; corrupt pre-snapshot event records are not decoded after a trusted durable snapshot.
- `POT-SNAPSHOT-001`: latest snapshot authority requires successful snapshot record decode and run/seq consistency; corrupt latest snapshot returns a typed record error before replay.
- `POT-TAINT-001`: tail slot writes preserve existing taint; read failures other than uninitialized slot return `RecoveryError::SlotTaintReadFailed` and never default to `Clean`.
- `POT-DURABILITY-001`: strict persistence is explicit and observable through `persist_strict()`/`close()`; `Drop` does not hide durability failure.
- `POT-DISCARD-001`: fallible result discard scan passes for production source domains.

## Evidence Already Collected

- `cargo check -p vb_storage --all-targets --all-features`: PASS.
- `cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`: PASS, 9 tests.
- `cargo test -p vb_storage events_for_run`: PASS, 24 tests after adding corrupt latest snapshot payload digest and postcard decode cases.
- `cargo test -p vb_storage recovery`: PASS, 186 tests after adding direct taint read fail-closed behavior.
- `cargo test -p vb_storage trimming`: PASS, 25 tests after adding snapshot key/payload run and sequence authority cases.
- `cargo test -p vb_storage close_propagates_persist_errors`: PASS, 1 test.
- `cargo test -p vb_runtime action_queue`: PASS, 18 tests.
- `bash scripts/check-ignored-fallible-results.sh`: PASS, `NoViolationFound`.
- `cargo fmt --all -- --check`: PASS on live rerun.
- `moon ci`: FAIL/BLOCK_GLOBAL on production `unreachable!(...)` and unrelated workspace-test dead code.

## Blockers Before Closure

- Re-run proof reviewers on this artifact set.
- Either update/strengthen Verus/TLA artifacts for vb-jpq7.3 or record reviewer-accepted limitations.
- Resolve canonical `moon ci` blocker or obtain explicit release-owner waiver.
- Do not close bead until black-hat/test/proof reviews accept the traceability.
