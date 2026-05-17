# Traceability Matrix — vb-2yb8

| Bead Req | Contract Clause | Test Evidence | File | Status |
|----------|----------------|---------------|------|--------|
| Per-primitive matrix | contract.md §Postconditions | `test_matrix_has_row_for_every_primitive` | TBD | PENDING |
| Event type mapping | contract.md §Contract Signatures | `test_row_maps_primitive_to_record_kind` | TBD | PENDING |
| Storage partition | contract.md §Contract Signatures | `test_row_names_storage_partition` | TBD | PENDING |
| Ack point | contract.md §Invariants | `test_handler_appends_before_return` | TBD | PENDING |
| Replay assertion | contract.md §Invariants | `test_replay_produces_identical_state` | TBD | PENDING |
| Missing evidence → beads | contract.md §Postconditions | `test_missing_evidence_fails_gate` | TBD | PENDING |
| Wired into release gate | contract.md §Postconditions | `moon run :ci` | TBD | PENDING |

## Primitive → Event Mapping (Draft)

| Primitive | CompiledNodeKind | Journal Events | ShardCommand | Handler |
|-----------|------------------|----------------|--------------|---------|
| set | SetConst | StepStarted, SlotWritten, StepSucceeded | Submit/Resume | handle_submit → drive_run |
| do | Do | StepStarted, ActionScheduled, ActionCompleted/ActionFailed, SlotWritten, StepSucceeded | ActionCompleted, ActionFailed | handle_action_completion, handle_action_failure |
| choose | Choose | StepStarted, StepSucceeded | Submit/Resume | handle_submit → drive_run |
| for_each | ForEach | StepStarted, SlotWritten, StepSucceeded | Submit/Resume | handle_submit → drive_run |
| together | Together | StepStarted, StepSucceeded | Submit/Resume | handle_submit → drive_run |
| collect | Collect | StepStarted, SlotWritten, StepSucceeded | Submit/Resume | handle_submit → drive_run |
| reduce | Reduce | StepStarted, SlotWritten, StepSucceeded | Submit/Resume | handle_submit → drive_run |
| repeat | Repeat | StepStarted, StepSucceeded | Submit/Resume | handle_submit → drive_run |
| wait | WaitUntil | StepStarted, WaitScheduled, WaitResolved | TimerFired | handle_timer |
| ask | Ask | StepStarted, AskScheduled, AskAnswered, SlotWritten, StepSucceeded | AskAnswered | handle_ask_answer |
| finish | Finish | StepStarted, RunFinished | Submit/Resume | handle_submit → drive_run |
| (meta) | ErrorHandler | StepStarted, SlotWritten, StepSucceeded | ActionFailed | handle_action_failure |
| (meta) | Retry | RetryScheduled | ActionFailed | handle_action_failure |

## Ack Point Audit

| Handler | Journal Append Before Return? | Evidence |
|---------|------------------------------|----------|
| handle_submit | Yes — RunSubmitted, RunAdmission before `self.runs.insert` | lifecycle.rs:109-117 |
| handle_resume | Indirect — via drive_run → flush_evidence | lifecycle.rs:159-161 |
| handle_action_completion | Yes — SlotWritten, StepSucceeded, ActionCompleted before `drive_run` | lifecycle.rs:192-207 |
| handle_legacy_action_completion | Yes — StepSucceeded before `drive_run` | lifecycle.rs:225-229 |
| handle_action_failure | Yes — ActionFailed before match outcome | lifecycle.rs:248-252 |
| handle_ask_answer | Yes — AskAnswered, SlotWritten, StepSucceeded before `drive_run` | lifecycle.rs:333-351 |
| handle_timer | Yes — WaitResolved before `drive_state` + flush_evidence after | lifecycle.rs:363-373 |
| handle_cancel | Yes — RunCancelled before `runs.swap_remove` | lifecycle.rs:379-380 |
| handle_inspect | N/A — read-only, no mutation | lifecycle.rs:390-392 |
