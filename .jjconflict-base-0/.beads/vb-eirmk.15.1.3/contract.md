# vb-eirmk.15.1.3 Contract

`RunFrame::checkpoint_v1(&self)` returns `RunFrameCheckpointV1` directly; a valid base DTO helper returns `CoreResult<RunFrameCheckpointV1>` by applying `?` only to `RunFrame::new` and wrapping the checkpoint DTO in `Ok`.

`RunFrame::from_checkpoint_v1(dto)` returns `Result<RunFrame, RunFrameCheckpointV1Error>`; tests compare its exact typed errors without converting that error to `CoreError` or relying on `std::error::Error`.

A new `populated_frame` helper builds its source frame exclusively through public `RunFrame` APIs and returns `CoreResult<RunFrame>`; the happy checkpoint test uses that source before serialization and restoration.

The populated frame has `RunId(9)`, PC `StepIdx(1)`, two steps `[Pending, Running]`, two slots, slot 0 `I64(7)` with `DerivedFromSecret` taint, slot 1 uninitialized and clean, executed count 1, maximum parallel in-flight 5, and current parallel in-flight 2.

The populated checkpoint roundtrip retains the four original `run_id()`, `pc()`, `step_count()`, and `slot_count()` assertions and compares the complete restored `RunFrame` with its populated source frame.

The ten existing test names and cases remain: `test_happy_path_restore`, `test_step_count_overflow`, `test_slot_count_overflow`, `test_empty_steps`, `test_slot_taint_length_mismatch`, `test_taint_without_value`, `test_parallel_count_out_of_bounds`, `test_invalid_frame_pc`, `test_precedence_slot_count_before_empty_steps`, and `test_precedence_parallel_before_pc`.

Expected error precedence follows the implementation: step-count conversion, slot-count conversion, empty steps, slot/taint length, taint without value, parallel-count bound, then frame construction/PC validation; the two oversized vectors retain length 65536 and precedence tests assert the earlier exact error.

The test file contains no bead/tool-mode metadata or unsupported postcondition claim; comments describe scenario intent only. Helpers remain at most 25 logical lines including signatures. Acceptance is the pinned file-only rustfmt plus the scoped native nextest lane, with source pin and native exit/log evidence recorded; this does not imply broader parent acceptance.
