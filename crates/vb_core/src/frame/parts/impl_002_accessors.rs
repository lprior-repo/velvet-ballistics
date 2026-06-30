impl RunFrame {
    /// Run identifier.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Current program counter.
    #[must_use]
    pub const fn pc(&self) -> StepIdx {
        self.pc
    }

    /// Number of transitions executed by this frame.
    #[must_use]
    pub const fn executed(&self) -> u64 {
        self.executed
    }

    /// Number of step states allocated in this frame.
    #[must_use]
    pub const fn step_count(&self) -> u16 {
        self.step_count
    }

    /// Number of slots allocated in this frame.
    #[must_use]
    pub const fn slot_count(&self) -> u16 {
        self.slot_count
    }

    /// Maximum allowed parallel in-flight branches for this workflow.
    #[must_use]
    pub const fn max_parallel_in_flight(&self) -> u16 {
        self.max_parallel_in_flight
    }

    /// Sets the maximum allowed parallel in-flight branches.
    pub fn set_max_parallel_in_flight(&mut self, limit: u16) {
        self.max_parallel_in_flight = limit;
    }

    /// Current number of parallel in-flight branch executions.
    #[must_use]
    pub const fn parallel_in_flight(&self) -> u16 {
        self.parallel_in_flight
    }

    /// Adds to the parallel in-flight counter without exceeding the configured limit.
    pub fn add_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
        let requested = self.parallel_in_flight.checked_add(count).ok_or(
            CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight overflow",
            },
        )?;
        if requested > self.max_parallel_in_flight {
            return Err(CoreError::ParallelLimitExceeded {
                limit: self.max_parallel_in_flight,
            });
        }
        self.parallel_in_flight = requested;
        Ok(())
    }

    /// Subtracts from the parallel in-flight counter.
    pub fn sub_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
        self.parallel_in_flight = self.parallel_in_flight.checked_sub(count).ok_or(
            CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight underflow",
            },
        )?;
        Ok(())
    }

    /// Moves the program counter after bounds validation.
    ///
    /// Rejects step indices outside the frame's step array to prevent
    /// staging an invalid PC that could lead to out-of-bounds state access
    /// on the next `step_once` call.
    pub fn set_pc(&mut self, pc: StepIdx) -> CoreResult<()> {
        if pc.as_usize() >= usize::from(self.step_count) {
            return Err(CoreError::InvalidProgramCounter { step: pc });
        }
        self.pc = pc;
        Ok(())
    }

    /// Increments the executed transition counter.
    pub fn increment_executed(&mut self) -> CoreResult<()> {
        self.executed = self
            .executed
            .checked_add(1)
            .ok_or(CoreError::StepCounterOverflow)?;
        Ok(())
    }
}
