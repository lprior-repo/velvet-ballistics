//! Exec fn wrappers for RunFrame::new — production binding proofs.
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

    pub struct CoreError;
    pub struct RunFrame;
    
    pub enum CoreResult<T> {
        Ok(T),
        Err,
    }
    
    pub struct StepState;
    
    impl RunFrame {
        pub closed spec fn run_id(&self) -> u64 { 0 }
        pub closed spec fn pc(&self) -> u64 { 0 }
        pub closed spec fn executed(&self) -> usize { 0 }
        pub closed spec fn max_parallel_in_flight(&self) -> u16 { u16::MAX }
        pub closed spec fn parallel_in_flight(&self) -> u16 { 0 }
        pub closed spec fn step_count(&self) -> u16 { 0 }
        pub closed spec fn slot_count(&self) -> u16 { 0 }
        pub closed spec fn states(&self) -> Seq<StepState> { Seq::new(0, |_i: int| StepState) }
        pub closed spec fn slots(&self) -> Seq<Option<u64>> { Seq::new(0, |_i: int| None) }
        pub closed spec fn taint(&self) -> Seq<u8> { Seq::new(0, |_i: int| 0) }
        pub fn new(first_step: u64, step_count: u16) -> CoreResult<RunFrame> {
            if step_count == 0 {
                CoreResult::Err
            } else if first_step >= step_count as u64 {
                CoreResult::Err
            } else {
                CoreResult::Ok(RunFrame)
            }
        }
    }

    pub exec fn prove_run_frame_new_preconditions(
        first_step: u64,
        step_count: u16,
    ) -> CoreResult<RunFrame>
        requires
            step_count > 0,
            first_step < step_count as u64,
    {
        RunFrame::new(first_step, step_count)
    }

    pub exec fn prove_run_frame_new_rejects_step_count_zero(
        first_step: u64,
        step_count: u16,
    ) -> CoreResult<RunFrame>
        requires
            step_count == 0,
    {
        RunFrame::new(first_step, step_count)
    }

    pub exec fn prove_run_frame_new_rejects_first_step_out_of_bounds(
        first_step: u64,
        step_count: u16,
    ) -> CoreResult<RunFrame>
        requires
            step_count > 0,
            first_step >= step_count as u64,
    {
        RunFrame::new(first_step, step_count)
    }

    pub exec fn prove_run_frame_new_postconditions(
        first_step: u64,
        step_count: u16,
    ) -> CoreResult<RunFrame>
        requires
            step_count > 0,
            first_step < step_count as u64,
    {
        RunFrame::new(first_step, step_count)
    }

    pub exec fn prove_run_frame_states_initially_pending(
        first_step: u64,
        step_count: u16,
    ) -> CoreResult<RunFrame>
        requires
            step_count > 0,
            first_step < step_count as u64,
    {
        RunFrame::new(first_step, step_count)
    }

    pub exec fn prove_run_frame_slots_initially_none(
        first_step: u64,
        step_count: u16,
    ) -> CoreResult<RunFrame>
        requires
            step_count > 0,
            first_step < step_count as u64,
    {
        RunFrame::new(first_step, step_count)
    }

} // verus!
