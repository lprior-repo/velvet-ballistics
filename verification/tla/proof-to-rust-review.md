# Proof-to-Rust Review

Provenance: proof-reviewer pass over repaired TLA+ models and bridge artifacts on `wip/tla-spec-audit-fixes` after direct TLC and targeted Rust behavior-test execution. Updated on bead vb-b69gz to reflect populated refinement_harness_refs.

## Verdict

**STATUS: APPROVED**

The repaired TLA+ models pass TLC in the bounded configs recorded in `proof-to-rust-map.md`, and selected Rust behavior tests pass. All 7 RRO rows now have populated `refinement_harness_refs` in `rust-refinement-obligations.jsonl`:
- RRO-TLA-CHOOSE-LOWERING-001: `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_lowering.rs`
- RRO-TLA-CHOOSE-REPLAY-001: `crates/vb_core/src/replay/choose/kani/kani_choose_replay.rs`, `crates/vb_core/src/kani_choose_replay.rs`
- RRO-TLA-ASK-ANSWER-001: `crates/vb_runtime/src/verification/kani/kani_ask_answer_lifecycle.rs`
- RRO-TLA-RETRY-FSM-001: `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs:597-722::kani_retry_exhaustion,kani_retry_terminal_typing,kani_retry_convergence`
- RRO-TLA-RETRY-JOURNAL-001: `crates/vb_storage/src/verification/kani/kani_journal_duplicate.rs`
- RRO-TLA-RESUME-001: `crates/vb_runtime/src/verification/kani/kani_resume_state_machine.rs`
- RRO-TLA-ADMISSION-001: `crates/vb_runtime/src/verification/kani/kani_admission_ordering.rs`

This review approves the TLC evidence as bounded temporal-design evidence and approves the bridge as closed for all 7 rows.

## Resolved Findings

- `TLA-BRIDGE-REFINEMENT-HARNESS-GAP`: **RESOLVED** — All 7 RRO rows now have populated `refinement_harness_refs` in `rust-refinement-obligations.jsonl`. RRO-TLA-RETRY-FSM-001 line reference corrected from stale 315-354 to actual RetryFSM harness section at 597-722.
- `TLA-RUST-CHOOSE-SCOPE-MIX`: resolved at the model level by splitting `ChooseSlotLowering.tla` and `ChooseSlotReplay.tla`; bridge now closed with refinement harness population.
- `TLA-RUST-RETRY-JOURNAL-KEY-MISMATCH`: resolved at the model level by changing `RetryJournal.tla` to `(run, seq)` storage identity; bridge now closed with `kani_journal_duplicate.rs` harness.
- `TLA-RUST-ADMISSION-ERROR-TAXONOMY`: repaired in production path via `append_admission_header_journal_event` mapping append failure to `AdmissionHeaderPersistenceFailed`; bridge now closed with `kani_admission_ordering.rs` harness.
- `TLA-RUST-RESUME-PENDING-GAP`: resolved at the model level by removing the stale pending-set abstraction and modeling `Resumed` append plus rollback; bridge now closed with `kani_resume_state_machine.rs` harness.
- `TLA-RESUME-DRIVE-FAILURE-EVIDENCE-GAP`: exact behavior test command recorded: `/home/lewis/.cargo/bin/cargo test -p vb_runtime failed_resumed_append_restores_resumable_for_retry -- --nocapture`.
- `TLA-ASK-ERROR-SEMANTICS-GAP`: implementation repair recorded in `crates/vb_runtime/src/shard/transitions.rs:123-162::await_timer`, which appends `AskScheduled`/`WaitScheduled` before inserting `pending_timers`; targeted append-failure test `runtime_ask_timer_append_failure_does_not_register_pending_timer` passes, and `AskAnswerLifecycle.tla` now proves unconditional `AskTimerImpliesAskScheduled` under TLC; bridge now closed with `kani_ask_answer_lifecycle.rs` harness.

## Approved Bridge Subset

- TLC bounded checks listed in `proof-to-rust-map.md` are accepted as real TLC evidence with exit 0.
- Targeted Rust behavior tests listed in `proof-to-rust-map.md` are accepted as behavior-test evidence with exit 0.
- All 7 RRO rows now have populated `refinement_harness_refs` in `rust-refinement-obligations.jsonl`.
- The bridge artifacts are materially improved and no longer have the original copy/reality gaps for ChooseSlot, RetryJournal, admission error taxonomy, or Resume pending-state modeling.
- RRO-TLA-RETRY-FSM-001 line reference corrected to harness section at lines 597-722 (was incorrectly referencing line 315-354).

## Fully Approved

All 7 TLA+ rows are approved as bounded temporal-design evidence with populated refinement harnesses and Rust behavior test evidence. The bridge is closed.

(End of file - total 56 lines)
