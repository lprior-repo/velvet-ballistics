    /// K-F2: validate_transition never panics for any of 64 pairs.
    #[kani::proof]
    fn validate_transition_no_panic_random() {
        let current_u8: u8 = kani::any();
        let new_u8: u8 = kani::any();
        let current = step_state_from_u8(current_u8);
        let new = step_state_from_u8(new_u8);
        let _result = validate_transition_inline(current, new);
    }

    /// K-F3: Idempotency — same-state transitions always return true.
    #[kani::proof]
    fn validate_transition_idempotent() {
        let state_u8 = kani::any::<u8>();
        let state = step_state_from_u8(state_u8 % 8);
        let result = validate_transition_inline(state, state);
        kani::assert(result, "self-transition always valid");
    }

    /// K-F4: Running can reach any terminal or suspend state.
    /// Uses kani::any() to symbolically explore valid target states.
    #[kani::proof]
    fn validate_transition_running_to_all_valid_targets() {
        let c = StepState::Running;
        let target: StepState = kani::any();
        // Running can transition to: Running, Succeeded, Failed, Waiting, Asking, Skipped, Cancelled
        // Not valid: Pending
        let result = validate_transition_inline(c, target);
        // If target is not Pending, transition should be valid
        if target != StepState::Pending {
            kani::assert(result, "Running can transition to non-Pending state");
        } else {
            kani::assert(!result, "Running cannot transition to Pending");
        }
    }

    /// K-F5: Terminal states block all non-self transitions EXCEPT Succeeded->Pending.
    /// Uses kani::any() to symbolically verify terminal blocking property.
    /// NOTE: vb_proof_kernels/src/step_state.rs:48 explicitly allows Succeeded->Pending,
    /// so this harness reflects that design decision.
    #[kani::proof]
    fn validate_transition_terminal_blocks_all() {
        let terminal: StepState = kani::any();
        let target: StepState = kani::any();
        // Succeeded, Failed, Skipped, Cancelled are terminal states
        let is_terminal = matches!(
            terminal,
            StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled
        );
        kani::assume(is_terminal);
        let result = validate_transition_inline(terminal, target);
        // Terminal states can transition to themselves (idempotent re-mark)
        if terminal == target {
            kani::assert(result, "terminal->self allowed");
        // Succeeded->Pending is explicitly allowed by proof kernel (step_state.rs:48)
        } else if terminal == StepState::Succeeded && target == StepState::Pending {
            kani::assert(result, "Succeeded->Pending allowed by proof kernel");
        } else {
            kani::assert(!result, "terminal->other blocked");
        }
    }

    /// K-PC1: set_pc never panics when StepIdx < step_count.
    /// Bounds assumption: pc.as_usize() < step_count as usize.
    #[kani::proof]
    fn set_pc_no_panic() {
        let step_count: u16 = kani::any();
        kani::assume(step_count > 0);

        let pc_raw: u16 = kani::any();
        kani::assume(pc_raw < step_count);
        let pc = StepIdx::new(pc_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let result = frame.set_pc(pc);
        kani::assert(result.is_ok(), "set_pc with valid idx returns Ok");
    }

    /// K-PC2: increment_executed never panics.
    /// No bounds assumption needed — executed uses checked_add internally.
    #[kani::proof]
    fn increment_executed_no_panic() {
        let step_count: u16 = kani::any();
        kani::assume(step_count > 0);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let _result = frame.increment_executed();
    }

    /// K-PC3: set_pc returns Err when StepIdx >= step_count (no panic).
    /// Bounds assumption: pc.as_usize() >= step_count as usize.
    #[kani::proof]
    fn set_pc_rejects_out_of_bounds() {
        let step_count: u16 = kani::any();
        kani::assume(step_count > 0);

        let pc_raw: u16 = kani::any();
        kani::assume(pc_raw >= step_count);
        let pc = StepIdx::new(pc_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let result = frame.set_pc(pc);
        kani::assert(result.is_err(), "set_pc with out-of-bounds idx returns Err");
    }

    /// K-S1: read_slot never panics for SlotIdx within valid bounds.
    /// Uses kani::any() for slot_count with assume bound > 0 and <= 16.
    /// NOTE: Tighter bound (slot_count <= 16) prevents Kani timeout from large symbolic state space.
    #[kani::proof]
    fn read_slot_no_panic() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16); // Tighter bound to reduce symbolic state space

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, slot_count);
        kani::assume(frame.is_ok());
        let frame = frame.unwrap();

        let result = frame.read_slot(slot);
        // Result is either Ok or Err(CoreError::SlotUninitialized), both are non-panic
        let _ = result;
    }

    /// K-S2: write_slot never panics for SlotIdx within valid bounds.
    #[kani::proof]
    fn write_slot_no_panic() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16);

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, slot_count);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let value: SlotValue = kani::any();
        let result = frame.write_slot(slot, value);
        let _ = result;
    }

    /// K-S3: read_taint never panics for SlotIdx within valid bounds.
    #[kani::proof]
    fn read_taint_no_panic() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16);

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, slot_count);
        kani::assume(frame.is_ok());
        let frame = frame.unwrap();

        let result = frame.read_taint(slot);
        let _ = result;
    }

    /// K-S4: write_taint never panics when slot is initialized.
    /// NOTE: This harness does NOT prove anything about uninitialized slots.
    /// Security regression tests cover the uninitialized-slot rejection path.
    #[kani::proof]
    fn write_taint_no_panic_initialized() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16);

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let taint: Taint = kani::any();

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, slot_count);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        // First initialize the slot (otherwise this is a different proof obligation)
        let value: SlotValue = kani::any();
        if frame.write_slot(slot, value).is_err() {
            return;
        }

        let result = frame.write_taint(slot, taint);
        let _ = result;
    }

    /// K-S5: slot_count bounds — ensures RunFrame::new handles 0 and 1 correctly.
    #[kani::proof]
    fn slot_count_edge_cases() {
        let slot_count: u16 = kani::any();
        // Only test edge cases 0 and 1
        kani::assume(slot_count <= 1);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, slot_count);
        // slot_count 0 and 1 are both valid, just create empty or single-element arrays
        kani::assert(frame.is_ok(), "slot_count 0 and 1 are valid");
    }

    /// K-S6: write_slot_with_taint sets both value and taint atomically.
    #[kani::proof]
    fn write_slot_with_taint_atomic() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16);

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let value: SlotValue = kani::any();
        let taint: Taint = kani::any();

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, slot_count);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let result = frame.write_slot_with_taint(slot, value, taint);
        kani::assert(
            result.is_ok(),
            "write_slot_with_taint succeeds for valid slot",
        );
    }

    /// K-EXEC1: executed counter starts at 0.
    #[kani::proof]
    fn executed_starts_at_zero() {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1);
        kani::assume(frame.is_ok());
        let frame = frame.unwrap();
        kani::assert(frame.executed() == 0, "executed counter starts at 0");
    }

    /// K-EXEC2: increment_executed advances counter.
    #[kani::proof]
    fn increment_executed_advances() {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let before = frame.executed();
        let result = frame.increment_executed();
        kani::assert(result.is_ok(), "increment_executed returns Ok");
        kani::assert(frame.executed() == before + 1, "executed increments by 1");
    }

    /// K-EXEC3: reinitialize resets executed counter.
    #[kani::proof]
    fn reinitialize_resets_executed() {
        let step_count: u16 = kani::any();
        kani::assume(step_count > 0);
        kani::assume(step_count <= 16);

        let slot_count: u16 = kani::any();
        kani::assume(slot_count <= 16);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, slot_count);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        // Increment executed a few times
        let _ = frame.increment_executed();
        let _ = frame.increment_executed();

        let new_run_id = RunId::new(2);
        let new_pc = StepIdx::ZERO;

        let result = frame.reinitialize(new_run_id, new_pc, step_count, slot_count);
        kani::assert(result.is_ok(), "reinitialize succeeds with valid params");
        kani::assert(
            frame.executed() == 0,
            "executed reset to 0 after reinitialize",
        );
        kani::assert(
            frame.run_id() == new_run_id,
            "run_id updated after reinitialize",
        );
        kani::assert(frame.pc() == new_pc, "pc updated after reinitialize");
    }

    /// K-EXEC4: add_parallel_in_flight increases counter.
    #[kani::proof]
    fn add_parallel_in_flight_increases() {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        frame.set_max_parallel_in_flight(100);

        let before = frame.parallel_in_flight();
        let delta: u16 = kani::any();
        kani::assume(delta > 0);
        kani::assume(delta <= 100);

        let result = frame.add_parallel_in_flight(delta);
        kani::assert(result.is_ok(), "add_parallel_in_flight returns Ok");
        kani::assert(
            frame.parallel_in_flight() == before + delta,
            "parallel_in_flight increases by delta",
        );
    }

    /// K-EXEC5: sub_parallel_in_flight decreases counter.
    #[kani::proof]
    fn sub_parallel_in_flight_decreases() {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        frame.set_max_parallel_in_flight(100);
        let _ = frame.add_parallel_in_flight(10);

        let before = frame.parallel_in_flight();
        let delta: u16 = kani::any();
        kani::assume(delta > 0);
        kani::assume(delta <= before);

        let result = frame.sub_parallel_in_flight(delta);
        kani::assert(result.is_ok(), "sub_parallel_in_flight returns Ok");
        kani::assert(
            frame.parallel_in_flight() == before - delta,
            "parallel_in_flight decreases by delta",
        );
    }
