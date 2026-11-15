struct AwaitTimerRegistration {
    state: RunState,
    step: vb_core::ids::StepIdx,
    register_timer: bool,
    timer: Option<PendingTimer>,
    ask_boundary_seq: Option<vb_storage::EventSeq>,
}
