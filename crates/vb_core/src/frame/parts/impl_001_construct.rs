fn try_frame_array<T>(len: usize, make: impl FnMut() -> T) -> CoreResult<Box<[T]>> {
    let mut v = Vec::new();
    v.try_reserve_exact(len)
        .map_err(|_| CoreError::AllocationFailed)?;
    if core::mem::size_of::<T>() != 0 && v.capacity() != len {
        return Err(CoreError::AllocationFailed);
    }
    v.resize_with(len, make);
    Ok(v.into_boxed_slice())
}

fn validate_frame_dimensions(first_step: StepIdx, step_count: u16) -> CoreResult<usize> {
    let states_len = usize::from(step_count);
    if states_len == 0 {
        return Err(CoreError::InvalidCompiledWorkflow {
            reason: "step_count_zero",
        });
    }
    if first_step.as_usize() >= states_len {
        return Err(CoreError::InvalidProgramCounter { step: first_step });
    }
    Ok(states_len)
}

type FrameArrays = (Box<[StepState]>, Box<[crate::value::SlotCell]>);

fn frame_arrays(states_len: usize, slots_len: usize) -> CoreResult<FrameArrays> {
    let states = try_frame_array(states_len, || StepState::Pending)?;
    let cells = try_frame_array(slots_len, crate::value::SlotCell::empty)?;
    Ok((states, cells))
}

impl RunFrame {
    /// Creates a frame with bounded step-state and slot arrays.
    pub fn new(
        run_id: RunId,
        first_step: StepIdx,
        step_count: u16,
        slot_count: u16,
    ) -> CoreResult<Self> {
        let states_len = validate_frame_dimensions(first_step, step_count)?;
        let slots_len = usize::from(slot_count);
        let (states, cells) = frame_arrays(states_len, slots_len)?;
        Ok(Self {
            run_id,
            pc: first_step,
            executed: 0,
            step_count,
            slot_count,
            max_parallel_in_flight: u16::MAX,
            parallel_in_flight: 0,
            states,
            cells,
        })
    }
}

impl RunFrame {
    /// Reinitializes a previously released frame for a new run with identical dimensions.
    pub fn reinitialize(
        &mut self,
        run_id: RunId,
        first_step: StepIdx,
        step_count: u16,
        slot_count: u16,
    ) -> CoreResult<()> {
        let states_len = usize::from(step_count);
        if states_len == 0 {
            return Err(CoreError::InvalidCompiledWorkflow {
                reason: "step_count_zero",
            });
        }
        if first_step.as_usize() >= states_len {
            return Err(CoreError::InvalidProgramCounter { step: first_step });
        }
        if self.step_count != step_count || self.slot_count != slot_count {
            return Err(CoreError::InvalidCompiledWorkflow {
                reason: "frame_dimension_mismatch",
            });
        }

        self.run_id = run_id;
        self.pc = first_step;
        self.executed = 0;
        self.max_parallel_in_flight = u16::MAX;
        self.parallel_in_flight = 0;
        for state in &mut self.states {
            *state = StepState::Pending;
        }
        for cell in &mut self.cells {
            *cell = crate::value::SlotCell::empty();
        }
        Ok(())
    }
}

#[cfg(test)]
mod allocation_tests {
    use super::*;

    #[test]
    fn frame_zero_length() -> CoreResult<()> {
        let arr = try_frame_array(0usize, || 42u8)?;
        let expected: &[u8] = &[];
        assert_eq!(arr.as_ref(), expected);
        Ok(())
    }

    #[test]
    fn frame_small_length() -> CoreResult<()> {
        let arr = try_frame_array(3usize, || 7u8)?;
        assert_eq!(arr.as_ref(), &[7u8, 7, 7]);
        Ok(())
    }

    #[test]
    fn frame_capacity_overflow() {
        let result = try_frame_array(usize::MAX, || 0u8);
        assert!(matches!(result, Err(CoreError::AllocationFailed)));
    }

    #[test]
    fn frame_unit_values() -> CoreResult<()> {
        let arr = try_frame_array(3usize, || ())?;
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.as_ref(), &[(), (), ()]);
        Ok(())
    }
}
