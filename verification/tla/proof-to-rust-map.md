# TLA+ Proof-to-Rust Map

Provenance: generated after adversarial Truth Serum / proof-reviewer / proof-to-implementation audit on `wip/tla-spec-audit-fixes`.

STATUS: REJECTED — TLC now verifies the repaired finite models, but several behavior-affecting TLA+ claims are not yet implementation-bound. TLA+ evidence below is temporal design evidence only, not Rust implementation proof.

## Active TLC Evidence

Command shape used from repository root:

```bash
tlc -config <cfg> <tla>
```

Direct active-context TLC runs on 2026-05-26:

| Model | Config | Bounds | Result |
|---|---|---|---|
| `verification/tla/ChooseSlot.tla` | `verification/tla/ChooseSlot.cfg` | `MaxBranches=3`, `MaxSlots=4`, `MaxSteps=3` | PASS: 968 states generated, 15 distinct, exit 0 |
| `specs/AskAnswerLifecycle.tla` | `specs/AskAnswerLifecycle.cfg` | `MaxRunId=1`, `MaxStepIdx=3`, `MaxSeqNo=4`, `MaxJournalEvents=24` | PASS: 868 states generated, 361 distinct, exit 0 |
| `specs/RetryFSM.tla` | `specs/RetryFSM.cfg` | `RunId={1,2}`, `StepId={1,2}`, `MaxAttemptsValue=2` | PASS: 10,713 states generated, 1,764 distinct, exit 0 |
| `specs/RetryJournal.tla` | `specs/RetryJournal.cfg` | `RunId={1}`, `StepId={1,2}`, `MaxJournalAttempts=1` | PASS: 105 states generated, 39 distinct, exit 0 |
| `specs/ResumeStateMachine.tla` | `specs/ResumeStateMachine.cfg` | `RunIds={r1,r2}`, `MaxJournalLength=4` | PASS: 850 states generated, 313 distinct, exit 0 |
| `specs/admission_header_before_ack.tla` | `specs/admission_header_before_ack.cfg` | `ErrorCodes={HeaderPersistenceFailed, QueueFull}` | PASS: 25 states generated, 13 distinct, exit 0 |

## Bridge Matrix

| TLA+ model | Rust target refs | Independent behavior evidence refs | Mapping status | Reviewer finding |
|---|---|---|---|---|
| `ChooseSlot.tla` | `crates/vb_compile/src/mod_compile_lowering/part_02.rs:225-240` (`lower_canonical_choose` fanout/empty table); `crates/vb_compile/src/compile/mod.rs:350-371` (`lower_choose` materializes `ChooseSlot`); `crates/vb_core/src/replay/choose.rs:12-58` (`replay_choose_slot` first true branch / otherwise) | `crates/vb_compile/src/mod_compile_lowering/tests.rs:524-608`; `crates/vb_core/src/replay/step_tests.rs:1615-1778` | PARTIAL | TLA is now syntactically valid and TLC-checked. Bridge is partial because the model mixes compile-time fanout validation with runtime branch evaluation; refinement must split compile and replay obligations. |
| `AskAnswerLifecycle.tla` | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:2-54` (`handle_ask_answer`); `crates/vb_runtime/src/shard/transitions.rs:88-139` (`await_action`/`await_timer`) | `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:268-344`, `:635-642` | REJECTED | TLA variables `AskState`, `PendingAnswers`, and `SeqNoCounter` do not have direct Rust equivalents. Rust uses `pending_timers`, `RuntimeState::Resumable`, and journal events; the current TLA model remains design-only. |
| `RetryFSM.tla` | `crates/vb_runtime/src/shard/helpers.rs:273-294` (`record_retry_attempt`); `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` action failure handling; `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs:315-354` Kani intent | `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs:315-354` (harness only, not behavior test); workspace runtime action failure tests exist but need exact binding | PARTIAL | TLC liveness now requires explicit weak fairness for retryable failure events. Rust mapping still needs exact source/test refs for stale completion, retry exhaustion, and non-retryable failure behavior. |
| `RetryJournal.tla` | `crates/vb_storage/src/journal/internal.rs:27-48` (`append_unpersisted` duplicate by `(run, seq)`); `crates/vb_storage/src/journal/internal.rs:50-74` (`append_queued_unpersisted` idempotent exact duplicate); `crates/vb_storage/src/keys.rs:41` (`run_event_key`) | `crates/vb_storage/src/journal/tests.rs:830-863`; `crates/workspace_tests/tests/journal_side_index_contracts.rs:483-486` | REJECTED | TLA duplicate model uses semantic `(run, step)` duplicate behavior and appends duplicates; Rust storage rejects duplicates by `(run, seq)` or accepts exact queued duplicates. Model and implementation do not prove the same contract. |
| `ResumeStateMachine.tla` | `crates/vb_runtime/src/shard/types.rs:722-733` (`RuntimeState`); `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:283-359` (`handle_resume`, `append_resumed_event`, rollback); `crates/vb_runtime/src/shard/transitions.rs:36-60` (`apply`) | `crates/vb_runtime/src/shard/tests/chunk_013.rs`, `chunk_016.rs`, `chunk_028.rs`, `chunk_030.rs` | PARTIAL | States map cleanly, but TLA `pending` set has no Rust field. Rust also appends `Resumed` before `drive_run`; a later drive failure rolls runtime state back but leaves `Resumed` in journal, which the TLA model does not express. |
| `admission_header_before_ack.tla` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:160-181` (journal append before `runs.insert`); `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:196-198` (live state after durable append); `crates/vb_runtime/src/error/mod.rs:38-47`; `crates/vb_runtime/src/error/conversions.rs:13-18` | `crates/vb_cli/tests/admission_durability_code.rs`; runtime admission tests need exact binding | PARTIAL / RUST BUG | Durable-before-live-state ordering matches. Error taxonomy does not: `AdmissionHeaderPersistenceFailed` exists but production append failure returns `StorageJournalAppend` through `From<JournalError>`. |

## Required Closure Standard

No row above may be called implementation-proven until it has:

1. exact production `source_refs` naming symbols, not just files;
2. independent behavior tests that would fail if the production behavior were deleted;
3. a refinement harness or proof artifact distinct from the behavior test;
4. raw command evidence for the exact behavior/refinement commands;
5. reviewer approval in `proof-to-rust-review.md`.
