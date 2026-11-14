struct AwaitTimerRegistration {
    state: RunState,
    step: vb_core::ids::StepIdx,
    register_timer: bool,
    generation: Option<u64>,
    ask_boundary_seq: Option<vb_storage::EventSeq>,
}
