    // --- Reinitialize fully resets step states ---

    #[test]
    fn frame_reinitialize_resets_all_step_states_to_pending() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;

        frame.mark_running(StepIdx::new(0))?;
        frame.mark_succeeded(StepIdx::new(0))?;
        frame.mark_running(StepIdx::new(1))?;
        frame.mark_failed(StepIdx::new(1))?;
        frame.mark_running(StepIdx::new(2))?;
        frame.mark_cancelled(StepIdx::new(2))?;

        frame.reinitialize(RunId::new(2), StepIdx::new(0), 3, 1)?;

        assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Pending);
        assert_eq!(frame.step_state(StepIdx::new(1))?, StepState::Pending);
        assert_eq!(frame.step_state(StepIdx::new(2))?, StepState::Pending);

        Ok(())
    }

    // --- All step state transitions are reachable via mark_ methods ---

    #[test]
    fn frame_all_step_state_variants_are_reachable() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 7, 1)?;

        frame.mark_running(StepIdx::new(0))?;
        assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Running);

        frame.mark_running(StepIdx::new(1))?;
        frame.mark_succeeded(StepIdx::new(1))?;
        assert_eq!(frame.step_state(StepIdx::new(1))?, StepState::Succeeded);

        frame.mark_running(StepIdx::new(2))?;
        frame.mark_failed(StepIdx::new(2))?;
        assert_eq!(frame.step_state(StepIdx::new(2))?, StepState::Failed);

        frame.mark_running(StepIdx::new(3))?;
        frame.mark_skipped(StepIdx::new(3))?;
        assert_eq!(frame.step_state(StepIdx::new(3))?, StepState::Skipped);

        frame.mark_running(StepIdx::new(4))?;
        frame.mark_waiting(StepIdx::new(4))?;
        assert_eq!(frame.step_state(StepIdx::new(4))?, StepState::Waiting);

        frame.mark_running(StepIdx::new(5))?;
        frame.mark_asking(StepIdx::new(5))?;
        assert_eq!(frame.step_state(StepIdx::new(5))?, StepState::Asking);

        frame.mark_running(StepIdx::new(6))?;
        frame.mark_cancelled(StepIdx::new(6))?;
        assert_eq!(frame.step_state(StepIdx::new(6))?, StepState::Cancelled);

        Ok(())
    }

    // =========================================================================
    // Slot lifecycle security regression tests
    // =========================================================================

    // --- Bug 1 fix: write_taint rejects uninitialized slot (prevents taint/value desync) ---

    #[test]
    fn security_write_taint_on_uninitialized_slot_returns_slot_uninitialized() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        // Slot 0 has never been written -- taint write must be rejected
        let result = frame.write_taint(SlotIdx::ZERO, Taint::Secret);
        assert_eq!(
            result,
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );

        // Taint must remain Clean (the default), not Secret
        // Note: read_taint also now requires the slot to be initialized,
        // so we verify by writing a value first, then checking taint.
        frame.write_slot(SlotIdx::ZERO, SlotValue::I64(1))?;
        assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Clean);

        Ok(())
    }

    // --- Bug 1 fix: read_taint rejects uninitialized slot ---

    #[test]
    fn security_read_taint_on_uninitialized_slot_returns_slot_uninitialized() -> CoreResult<()> {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        let result = frame.read_taint(SlotIdx::ZERO);
        assert_eq!(
            result,
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );

        Ok(())
    }

    // --- Bug 1 fix: write_taint succeeds on initialized slot ---

    #[test]
    fn security_write_taint_on_initialized_slot_succeeds() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        frame.write_slot(SlotIdx::ZERO, SlotValue::Bool(true))?;
        assert_eq!(frame.write_taint(SlotIdx::ZERO, Taint::Secret), Ok(()));
        assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Secret);

        Ok(())
    }

    // --- Bug 2 fix: read_slot distinguishes uninitialized from out-of-bounds ---

    #[test]
    fn security_read_slot_distinguishes_uninitialized_from_out_of_bounds() -> CoreResult<()> {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        // Slot 0 exists but is uninitialized -- returns SlotUninitialized
        assert_eq!(
            frame.read_slot(SlotIdx::ZERO),
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );

        // Slot 99 does not exist -- returns SlotOutOfBounds
        assert_eq!(
            frame.read_slot(SlotIdx::new(99)),
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(99)
            })
        );

        Ok(())
    }

    // --- Regression: overwrite of initialized slot is allowed (write-in-place semantics) ---

    #[test]
    fn security_overwrite_initialized_slot_succeeds() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        frame.write_slot(SlotIdx::ZERO, SlotValue::I64(10))?;
        assert_eq!(frame.read_slot(SlotIdx::ZERO)?, &SlotValue::I64(10));

        // Overwrite is allowed (no write-once enforcement)
        frame.write_slot(SlotIdx::ZERO, SlotValue::Bool(false))?;
        assert_eq!(frame.read_slot(SlotIdx::ZERO)?, &SlotValue::Bool(false));

        Ok(())
    }

    // --- Regression: read_taint on out-of-bounds slot returns SlotOutOfBounds ---

    #[test]
    fn security_read_taint_out_of_bounds_returns_slot_out_of_bounds() -> CoreResult<()> {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        assert_eq!(
            frame.read_taint(SlotIdx::new(99)),
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(99)
            })
        );

        Ok(())
    }

    // --- Regression: write_taint on out-of-bounds slot returns SlotOutOfBounds ---

    #[test]
    fn security_write_taint_out_of_bounds_returns_slot_out_of_bounds() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        assert_eq!(
            frame.write_taint(SlotIdx::new(99), Taint::Secret),
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(99)
            })
        );

        Ok(())
    }

    // --- Regression: write_taint after slot cleared by overwrite to None ---

    #[test]
    fn security_write_taint_after_slot_cleared_returns_uninitialized() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1)?;

        // Initialize and verify
        frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(1), Taint::Secret)?;
        assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Secret);

        // Overwrite with None-equivalent (clear via write_slot_with_taint not possible,
        // but write_slot sets the value directly)
        // After reinitialize, the slot should be cleared
        frame.reinitialize(RunId::new(2), StepIdx::ZERO, 2, 1)?;

        // Now write_taint should fail because slot is uninitialized again
        assert_eq!(
            frame.write_taint(SlotIdx::ZERO, Taint::Secret),
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );

        Ok(())
    }

    // =========================================================================
    // Parallel in-flight tracking tests
    // =========================================================================

    #[test]
    fn parallel_in_flight_tracks_peak_when_spawning_branches() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;
        frame.set_max_parallel_in_flight(10);

        assert_eq!(frame.parallel_in_flight(), 0);

        frame.add_parallel_in_flight(3)?;
        assert_eq!(frame.parallel_in_flight(), 3);

        frame.sub_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 1);

        Ok(())
    }

    #[test]
    fn parallel_in_flight_tracks_current_after_join_and_respawn() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 5, 1)?;
        frame.set_max_parallel_in_flight(10);

        frame.add_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 2);

        frame.add_parallel_in_flight(3)?;
        assert_eq!(frame.parallel_in_flight(), 5);

        frame.sub_parallel_in_flight(5)?;
        assert_eq!(frame.parallel_in_flight(), 0);

        frame.add_parallel_in_flight(4)?;
        assert_eq!(frame.parallel_in_flight(), 4);

        frame.add_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 6);

        Ok(())
    }

    #[test]
    fn parallel_in_flight_rejects_configured_limit_without_mutation() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;
        frame.set_max_parallel_in_flight(4);

        frame.add_parallel_in_flight(3)?;
        assert_eq!(frame.parallel_in_flight(), 3);

        assert_eq!(
            frame.add_parallel_in_flight(2),
            Err(CoreError::ParallelLimitExceeded { limit: 4 })
        );
        assert_eq!(frame.parallel_in_flight(), 3);
        assert_eq!(frame.max_parallel_in_flight(), 4);

        Ok(())
    }

    #[test]
    fn parallel_in_flight_together_start_join_lifecycle() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 10, 1)?;
        frame.set_max_parallel_in_flight(10);

        assert_eq!(frame.parallel_in_flight(), 0);

        frame.add_parallel_in_flight(4)?;
        assert_eq!(frame.parallel_in_flight(), 4);

        frame.add_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 6);

        frame.sub_parallel_in_flight(4)?;
        assert_eq!(frame.parallel_in_flight(), 2);

        frame.sub_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 0);

        Ok(())
    }

    #[test]
    fn parallel_in_flight_overflow_returns_error() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1)?;
        frame.set_max_parallel_in_flight(u16::MAX);

        frame.add_parallel_in_flight(u16::MAX)?;
        assert_eq!(
            frame.add_parallel_in_flight(2),
            Err(CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight overflow".into()
            })
        );

        Ok(())
    }

    #[test]
    fn parallel_in_flight_underflow_returns_error() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1)?;

        assert_eq!(
            frame.sub_parallel_in_flight(1),
            Err(CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight underflow".into()
            })
        );

        Ok(())
    }

    // =========================================================================
    // Step state machine: terminal-state isolation tests
    // =========================================================================

    /// VB-CORE-STATE-001: Terminal states (Succeeded, Failed, Cancelled, Skipped)
    /// block ALL transitions out, but allow idempotent re-mark (same→same).
    /// Modeled in TLA+ by StepState.tla.
    #[test]
    fn ut_terminal_state_blocks_transitions() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 1)?;

        // Transition each step to a different terminal state.
        frame.mark_succeeded(StepIdx::ZERO)?;
        frame.mark_failed(StepIdx::new(1))?;
        frame.mark_cancelled(StepIdx::new(2))?;
        frame.mark_skipped(StepIdx::new(3))?;

        // Idempotent re-mark: same state → same state is always valid.
        assert_eq!(frame.mark_succeeded(StepIdx::ZERO), Ok(()));
        assert_eq!(frame.mark_failed(StepIdx::new(1)), Ok(()));
        assert_eq!(frame.mark_cancelled(StepIdx::new(2)), Ok(()));
        assert_eq!(frame.mark_skipped(StepIdx::new(3)), Ok(()));

        // Terminal states cannot transition to any other state.
        // Succeeded cannot go to anything else.
        assert_eq!(
            frame.mark_running(StepIdx::ZERO),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );
        assert_eq!(
            frame.mark_failed(StepIdx::ZERO),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );
        assert_eq!(
            frame.mark_waiting(StepIdx::ZERO),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );

        // Failed cannot go to anything else.
        assert_eq!(
            frame.mark_running(StepIdx::new(1)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );
        assert_eq!(
            frame.mark_succeeded(StepIdx::new(1)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );
        assert_eq!(
            frame.mark_waiting(StepIdx::new(1)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );

        // Cancelled cannot go to anything else.
        assert_eq!(
            frame.mark_running(StepIdx::new(2)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );
        assert_eq!(
            frame.mark_succeeded(StepIdx::new(2)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );

        // Skipped cannot go to anything else.
        assert_eq!(
            frame.mark_running(StepIdx::new(3)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );
        assert_eq!(
            frame.mark_failed(StepIdx::new(3)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition".into()
            })
        );

        Ok(())
    }
