# Proof-to-Rust Review

Provenance: proof-reviewer pass over repaired TLA+ models and bridge artifacts on `wip/tla-spec-audit-fixes` after direct TLC and targeted Rust behavior-test execution.

STATUS: REJECTED

## Verdict

The repaired TLA+ models now pass TLC in the bounded configs recorded in `proof-to-rust-map.md`, and selected Rust behavior tests pass. The bridge is still not approved as implementation proof because most rows lack an independent refinement harness or an explicit approved proportional waiver. The ask-answer storage-error repair now has source/test/TLC evidence recorded, but still needs independent proof-reviewer approval.

This review approves the TLC evidence as bounded temporal-design evidence only. It does not approve any TLA+ row as full Rust implementation proof.

## Current Findings

{"kind":"finding/v1","id":"TLA-BRIDGE-REFINEMENT-HARNESS-GAP","severity":"high","artifact":"verification/tla/rust-refinement-obligations.jsonl","obligation":"ALL-PARTIAL-RRO","summary":"Most bridge rows cite TLC plus behavior tests but no separate refinement harness or approved waiver.","evidence":"RRO rows for ChooseSlotLowering, ChooseSlotReplay, AskAnswerLifecycle, RetryJournal, ResumeStateMachine, and admission_header_before_ack have empty refinement_harness_refs. RetryFSM cites a Kani harness but its binding remains marked partial.","required_fix":"Add implementation-bound refinement harnesses or explicit proportional waivers for each behavior-affecting row, then rerun proof-reviewer."}

## Resolved Prior Findings

- `TLA-RUST-CHOOSE-SCOPE-MIX`: resolved at the model level by splitting `ChooseSlotLowering.tla` and `ChooseSlotReplay.tla`; bridge still partial pending refinement closure.
- `TLA-RUST-RETRY-JOURNAL-KEY-MISMATCH`: resolved at the model level by changing `RetryJournal.tla` to `(run, seq)` storage identity; bridge still partial pending refinement closure.
- `TLA-RUST-ADMISSION-ERROR-TAXONOMY`: repaired in production path via `append_admission_header_journal_event` mapping append failure to `AdmissionHeaderPersistenceFailed`; targeted admission tests pass.
- `TLA-RUST-RESUME-PENDING-GAP`: resolved at the model level by removing the stale pending-set abstraction and modeling `Resumed` append plus rollback.
- `TLA-RESUME-DRIVE-FAILURE-EVIDENCE-GAP`: exact behavior test command now recorded: `/home/lewis/.cargo/bin/cargo test -p vb_runtime failed_resumed_append_restores_resumable_for_retry -- --nocapture`.
- `TLA-ASK-ERROR-SEMANTICS-GAP`: implementation repair recorded in `crates/vb_runtime/src/shard/transitions.rs:123-162::await_timer`, which appends `AskScheduled`/`WaitScheduled` before inserting `pending_timers`; targeted append-failure test `runtime_ask_timer_append_failure_does_not_register_pending_timer` passes, and `AskAnswerLifecycle.tla` now proves unconditional `AskTimerImpliesAskScheduled` under TLC. This is not independent proof-reviewer approval.

## Approved Subset

- TLC bounded checks listed in `proof-to-rust-map.md` are accepted as real TLC evidence with exit 0.
- Targeted Rust behavior tests listed in `proof-to-rust-map.md` are accepted as behavior-test evidence with exit 0.
- The bridge artifacts are materially improved and no longer have the original copy/reality gaps for ChooseSlot, RetryJournal, admission error taxonomy, or Resume pending-state modeling.

## Not Approved

- No TLA+ claim is approved as full Rust implementation proof.
- The ask-answer storage-error repair lacks independent proof-reviewer approval and refinement-harness/waiver closure.
- Rows with empty `refinement_harness_refs` are not closed unless a future reviewer approves an explicit proportional waiver.
