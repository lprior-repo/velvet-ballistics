# Proof-to-Rust Review

Provenance: proof-reviewer + truth-serum audit of TLA+ branch `wip/tla-spec-audit-fixes` after direct active-context TLC execution.

STATUS: REJECTED

## Verdict

The TLA+ artifacts now pass TLC in the bounded configs listed in `proof-to-rust-map.md`, but the proof-to-Rust bridge is not approved. Passing TLC is not implementation proof. Three behavior-affecting models are rejected outright for copy/reality gaps, and three are partial pending exact behavior/refinement evidence.

## Findings

{"kind":"finding/v1","id":"TLA-RUST-ASK-ANSWER-GAP","severity":"critical","artifact":"specs/AskAnswerLifecycle.tla","obligation":"RRO-TLA-ASK-ANSWER-001","summary":"TLA variables AskState/PendingAnswers/SeqNoCounter are not implemented as Rust state.","evidence":"Rust handle_ask_answer writes SlotWritten then AskAnswered at crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:30-46, but no production SeqNoCounter or PendingAnswers was found in the Rust source scan.","required_fix":"Model Rust pending_timers/RuntimeState/journal sequence semantics or add the missing production state with behavior tests."}
{"kind":"finding/v1","id":"TLA-RUST-RETRY-JOURNAL-KEY-MISMATCH","severity":"critical","artifact":"specs/RetryJournal.tla","obligation":"RRO-TLA-RETRY-JOURNAL-001","summary":"TLA duplicate idempotency is keyed by semantic run/step; Rust storage duplicate identity is run/seq.","evidence":"crates/vb_storage/src/journal/internal.rs:32 uses run_event_key(event.run_id(), event.seq()); duplicate branch at lines 33-38 returns DuplicateEvent by run/seq.","required_fix":"Align the TLA journal duplicate model to Rust run/seq keys or intentionally change Rust semantics."}
{"kind":"finding/v1","id":"TLA-RUST-ADMISSION-ERROR-TAXONOMY","severity":"high","artifact":"specs/admission_header_before_ack.tla","obligation":"RRO-TLA-ADMISSION-001","summary":"TLA HeaderPersistenceFailed code does not match production error construction.","evidence":"RuntimeError::AdmissionHeaderPersistenceFailed exists at crates/vb_runtime/src/error/mod.rs:43-47, but JournalError converts to RuntimeError::StorageJournalAppend at crates/vb_runtime/src/error/conversions.rs:13-18; admission append failures return Err(error) at crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:166-179.","required_fix":"Map admission-header append failures to AdmissionHeaderPersistenceFailed or change TLA ErrorCodes and tests."}
{"kind":"finding/v1","id":"TLA-RUST-RESUME-PENDING-GAP","severity":"high","artifact":"specs/ResumeStateMachine.tla","obligation":"RRO-TLA-RESUME-001","summary":"TLA pending set has no Rust counterpart, and drive failure after Resumed append is not modeled.","evidence":"Rust RuntimeState exists at crates/vb_runtime/src/shard/types.rs:722-733; handle_resume appends Resumed before drive_run at crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:299-301 and rolls back state on drive failure at lines 346-358.","required_fix":"Update the TLA model to include post-resume drive failure/rollback or change Rust behavior/tests."}
{"kind":"finding/v1","id":"TLA-RUST-CHOOSE-SCOPE-MIX","severity":"medium","artifact":"verification/tla/ChooseSlot.tla","obligation":"RRO-TLA-CHOOSE-001","summary":"ChooseSlot model mixes compile-time lowering constraints and runtime branch-selection semantics.","evidence":"Fanout/empty validation is in lower_canonical_choose at crates/vb_compile/src/mod_compile_lowering/part_02.rs:225-240; first-true branch selection is in replay_choose_slot at crates/vb_core/src/replay/choose.rs:12-58.","required_fix":"Split the TLA model or create a bridge that maps each action to the correct Rust layer."}

## Approved Subset

- TLC execution itself is approved as bounded temporal evidence for the six repaired configs.
- `RetryFSM.tla` liveness is now explicitly fairness-bound and passes TLC with `RunId={1,2}`, `StepId={1,2}`, `MaxAttemptsValue=2`.

## Not Approved

- No TLA+ claim here is approved as Rust implementation proof.
- Kani/Verus/Loom evidence from earlier audits remains separately rejected unless rerun and reviewed with production-bound harnesses.
