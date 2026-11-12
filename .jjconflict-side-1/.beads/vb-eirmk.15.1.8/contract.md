# VB-EIRMK.15.1.8 RunFrame Constructor and PC Tests Contract

Five named tests verify constructor scalar defaults, array zeroing, zero-slot handling, valid program counter advancement, and out-of-range program counter rejection on a RunFrame with populated state.

Test name: fresh_frame_exact_scalars
Behavior: RunFrame::new with RunId 47, first step index 1, 3 steps, 3 slots returns Ok; identity, PC, count, and capacity accessors return their documented default or specified values including pc 1, executed 0, step_count 3, slot_count 3, max_parallel_in_flight u16::MAX, parallel_in_flight 0, written_slot_count 0.

Test name: fresh_frame_exact_arrays
Behavior: RunFrame::new with the same parameters returns Ok; states_snapshot returns 3 Pending values; slots_snapshot returns 3 None values; taint_snapshot returns 3 Clean values; initialized_slots returns an empty list.

Test name: fresh_frame_zero_slots
Behavior: RunFrame::new with RunId 47, first step index 1, 2 steps, 0 slots returns Ok; slot_count is 0; slots and taint snapshots are empty; states_snapshot returns 2 Pending values; initialized_slots is empty.

Test name: set_pc_updates_frame
Behavior: A RunFrame constructed with 3 steps is populated with mark_running on step 1, a slot value of I64(7) with DerivedFromSecret taint, an increment_executed call, set_max_parallel_in_flight to 5, and add_parallel_in_flight to current count 2; after calling set_pc with step index 2, the resulting frame is structurally equal to a direct private field assignment of pc to 2 on a cloned fixture, proving no collateral mutation occurred.

Test name: set_pc_rejects_out_of_range
Behavior: The same populated RunFrame is used; calling set_pc with step index 3 (equal to step_count) returns Err(CoreError::InvalidProgramCounter) with step 3; calling set_pc with u16::MAX returns Err(CoreError::InvalidProgramCounter) with u16::MAX; after each rejection the entire fixture state remains structurally equal to the original populated frame.
