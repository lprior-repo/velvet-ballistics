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

