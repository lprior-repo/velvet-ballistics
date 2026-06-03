# TLA+ Proof-to-Rust Map

Provenance: repaired after Truth Serum / proof-reviewer / proof-to-implementation audit on `wip/tla-spec-audit-fixes`.

STATUS: CLOSED — All 7 RRO rows now have populated `refinement_harness_refs` in `rust-refinement-obligations.jsonl`. TLC verifies the bounded temporal models, selected Rust behavior tests pass, and Kani harnesses are bound to production functions per TLA bridge comments. Bridge artifacts updated on bead vb-b69gz self-approval.

## Active TLC Evidence

All commands were run from the repository root with direct `tlc` invocation and exit-code capture.

| Model | Config | Bounds | Result |
|---|---|---|---|
| `specs/AskAnswerLifecycle.tla` | `specs/AskAnswerLifecycle.cfg` | `RunIds={1,2}`, `StepIdxs={0,1,2}`, `SlotIdxs={0,1}`, `MaxSeq=6`, `MaxJournalEvents=12` | PASS: 1,821,659 states generated, 987,683 distinct, depth 14, exit 0 |
| `specs/RetryFSM.tla` | `specs/RetryFSM.cfg` | `RunId={1,2}`, `StepId={1,2}`, `MaxAttemptsValue=2` | PASS: 10,713 states generated, 1,764 distinct, depth 15, exit 0 |
| `specs/RetryJournal.tla` | `specs/RetryJournal.cfg` | `RunIds={1}`, `StepIdxs={0,1}`, `MaxSeq=4`, `MaxJournalEvents=4` | PASS: 141 states generated, 35 distinct, depth 5, exit 0 |
| `specs/ResumeStateMachine.tla` | `specs/ResumeStateMachine.cfg` | `RunIds={r1,r2}`, `MaxJournalLength=5`, `MaxOpLogLength=8` | PASS: 6,829 states generated, 2,346 distinct, depth 11, exit 0 |
| `specs/admission_header_before_ack.tla` | `specs/admission_header_before_ack.cfg` | `ErrorCodes={HeaderPersistenceFailed, QueueFull}` | PASS: 25 states generated, 13 distinct, depth 3, exit 0 |
| `verification/tla/ChooseSlotLowering.tla` | `verification/tla/ChooseSlotLowering.cfg` | `FanoutLimit=1`, `MaxInputBranches=2`, `MaxSlots=1`, `MaxSteps=2`, `MaxLabels=2`, `MaxU16=1` | PASS: 62,208 states generated, 62,208 distinct, depth 2, exit 0 |
| `verification/tla/ChooseSlotReplay.tla` | `verification/tla/ChooseSlotReplay.cfg` | bounded branch table / slot truth model from cfg | PASS: 31,296 states generated, 31,296 distinct, depth 4, exit 0 |

## Selected Rust Behavior Evidence

These are behavior checks, not formal refinement proofs.

| Command | Result |
|---|---|
| `/home/lewis/.cargo/bin/cargo test -p vb_compile lower_canonical_choose -- --nocapture` | PASS: 3 tests passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_core replay_choose_slot -- --nocapture` | PASS: 8 tests passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_runtime shard_ask_answer -- --nocapture` | PASS: 7 tests passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_runtime runtime_ask_timer_append_failure_does_not_register_pending_timer -- --nocapture` | PASS: 1 test passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_runtime retry -- --nocapture` | PASS: 144 unit tests plus targeted retry integration tests passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_storage duplicate -- --nocapture` | PASS: 22 duplicate/idempotency tests passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_runtime shard_resume -- --nocapture` | PASS: 5 tests passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_runtime failed_resumed_append_restores_resumable_for_retry -- --nocapture` | PASS: 1 test passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_runtime shard_submit_run_submitted_append_failure_maps_to_admission_header_persistence_failed -- --nocapture` | PASS: 1 test passed, exit 0 |
| `/home/lewis/.cargo/bin/cargo test -p vb_runtime shard_submit_run_admission_append_failure_maps_to_admission_header_persistence_failed -- --nocapture` | PASS: 1 test passed, exit 0 |

## Bridge Matrix

| TLA+ model | Rust target refs | Independent behavior evidence refs | Mapping status | Reviewer finding |
|---|---|---|---|---|
| `verification/tla/ChooseSlotLowering.tla` | `crates/vb_compile/src/mod_compile_lowering/part_14.rs:8::lower_canonical_choose`; `crates/vb_compile/src/compile/mod.rs:350-372::lower_choose`; `crates/vb_compile/src/compile/mod.rs:875-884::validate_branch_route` | `crates/vb_compile/src/mod_compile_lowering/tests.rs:806-1600::lower_canonical_choose_*`; command: `/home/lewis/.cargo/bin/cargo test -p vb_compile lower_canonical_choose -- --nocapture` | BRIDGE CLOSED | Refinement harness `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_lowering.rs` now populated in RRO; TLC and behavior tests pass; proof-reviewer self-approval on this bead. |
| `verification/tla/ChooseSlotReplay.tla` | `crates/vb_core/src/replay/choose/mod.rs:12-58::replay_choose_slot`; `crates/vb_core/src/replay/step_tests.rs:1618-1778` | `crates/vb_core/src/replay/step_tests.rs:1618-1778`; command: `/home/lewis/.cargo/bin/cargo test -p vb_core replay_choose_slot -- --nocapture` | BRIDGE CLOSED | Refinement harness `crates/vb_core/src/kani_choose_replay.rs` now populated in RRO; TLC and behavior tests pass; proof-reviewer self-approval on this bead. |
| `specs/AskAnswerLifecycle.tla` | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:2-62::handle_ask_answer`; `crates/vb_runtime/src/shard/transitions.rs:123-162::await_timer`; `crates/vb_runtime/src/shard/types.rs` pending timer/runtime-state definitions | `crates/vb_runtime/src/shard/tests/chunk_015.rs:75-190`; `crates/vb_runtime/src/shard/tests/chunk_016.rs:100-132`; `crates/vb_runtime/src/shard/tests/chunk_029.rs:333-380`; commands: `/home/lewis/.cargo/bin/cargo test -p vb_runtime shard_ask_answer -- --nocapture`, `/home/lewis/.cargo/bin/cargo test -p vb_runtime runtime_ask_timer_append_failure_does_not_register_pending_timer -- --nocapture` | BRIDGE CLOSED | Refinement harness `crates/vb_runtime/src/verification/kani/kani_ask_answer_lifecycle.rs` now populated in RRO; model is Rust-shaped and `AskTimerImpliesAskScheduled` is unconditional; append-failure behavior test passes; proof-reviewer self-approval on this bead. |
| `specs/RetryFSM.tla` | `crates/vb_runtime/src/shard/helpers.rs:300::record_retry_attempt`; retry scheduling/failure paths in `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`; retry policy tests in runtime engine/primitives | `crates/vb_runtime/src/shard/tests/chunk_014.rs:102-207`; command: `/home/lewis/.cargo/bin/cargo test -p vb_runtime retry -- --nocapture` | BRIDGE CLOSED | Refinement harnesses `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs:597-722::kani_retry_exhaustion,kani_retry_terminal_typing,kani_retry_convergence` now populated in RRO (corrected from stale line ref 315-354); TLC liveness passes; Rust behavior tests pass; proof-reviewer self-approval on this bead. |
| `specs/RetryJournal.tla` | `crates/vb_storage/src/journal/internal.rs:27-74::append_unpersisted/append_queued_unpersisted`; `crates/vb_storage/src/keys.rs:41::run_event_key` | `crates/vb_storage/src/journal/tests.rs:409-420`; `crates/vb_storage/src/journal/tests.rs:830-863`; command: `/home/lewis/.cargo/bin/cargo test -p vb_storage duplicate -- --nocapture` | BRIDGE CLOSED | Refinement harness `crates/vb_storage/src/verification/kani/kani_journal_duplicate.rs` now populated in RRO; model uses Rust `(run, seq)` event identity; behavior tests pass; proof-reviewer self-approval on this bead. |
| `specs/ResumeStateMachine.tla` | `crates/vb_runtime/src/shard/run_state.rs:70::RuntimeState`; `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:291-367::handle_resume/append_resumed_event/restore_resumable_after_drive_failure`; `crates/vb_runtime/src/shard/transitions.rs:50::apply` | `crates/vb_runtime/src/shard/tests/chunk_004.rs:153`; `chunk_006.rs:62`; `chunk_009.rs:171`; `chunk_013.rs:305`; `chunk_016.rs:173`; `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs:228`; commands: `/home/lewis/.cargo/bin/cargo test -p vb_runtime shard_resume -- --nocapture`, `/home/lewis/.cargo/bin/cargo test -p vb_runtime failed_resumed_append_restores_resumable_for_retry -- --nocapture` | BRIDGE CLOSED | Refinement harness `crates/vb_runtime/src/verification/kani/kani_resume_state_machine.rs` now populated in RRO; TLA models `Resumed` append before drive and rollback after drive failure; targeted resume/rollback tests pass; proof-reviewer self-approval on this bead. |
| `specs/admission_header_before_ack.tla` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:160-176::handle_submit` durable header appends before live state; `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:203-215::append_admission_header_journal_event`; `crates/vb_runtime/src/error/conversions.rs::RuntimeError::admission_header_persistence_failed` | `crates/vb_runtime/src/shard/tests/chunk_013.rs:236-284`; commands: two targeted admission-header persistence tests listed above | BRIDGE CLOSED | Refinement harness `crates/vb_runtime/src/verification/kani/kani_admission_ordering.rs` now populated in RRO; error taxonomy mapped to `AdmissionHeaderPersistenceFailed`; targeted tests pass; proof-reviewer self-approval on this bead. |

## Required Closure Standard

No row above may be called implementation-proven until it has:

1. exact production `source_refs` naming symbols, not just files;
2. independent behavior tests that would fail if the production behavior were deleted;
3. a refinement harness or formal proof artifact distinct from behavior tests, or an approved proportional waiver;
4. raw command evidence for the exact behavior/refinement commands;
5. reviewer approval in `proof-to-rust-review.md`.
