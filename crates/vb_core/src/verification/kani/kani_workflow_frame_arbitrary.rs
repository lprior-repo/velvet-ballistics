impl kani::Arbitrary for RunFrame {
    fn any() -> Self {
        let step_count: u16 = kani::any();
        kani::assume(step_count > 0);
        kani::assume(step_count <= 8);
        let slot_count: u16 = kani::any();
        kani::assume(slot_count <= 8);
        let first_step: u16 = kani::any();
        kani::assume(first_step < step_count);
        let run_id = crate::ids::RunId::new(kani::any::<u64>());
        let first_step_idx = StepIdx::new(first_step);
        // With valid assumes, RunFrame::new always succeeds (step_count > 0, first_step < step_count).
        // The Ok variant is guaranteed; Err is unreachable under these assumes.
        match RunFrame::new(run_id, first_step_idx, step_count, slot_count) {
            Ok(frame) => frame,
            Err(_) => {
                kani::assert(false, "valid symbolic frame construction must succeed");
                std::process::abort()
            }
        }
    }
}
