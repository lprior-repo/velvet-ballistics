
pub(crate) fn flow_error(step: crate::ids::StepIdx) -> crate::workflow::SlotFlowError {
    crate::workflow::SlotFlowError {
        step,
        slot: None,
        reason: crate::workflow::SlotFlowFailure::UnsupportedSemantics,
    }
}

macro_rules! kinds {
    () => {
        crate::workflow::CompiledNodeKind::ForEachConcurrentStart { .. }
            | crate::workflow::CompiledNodeKind::TogetherConcurrentStart { .. }
            | crate::workflow::CompiledNodeKind::FirstSuccessStart { .. }
            | crate::workflow::CompiledNodeKind::ConcurrentLaneEnd { .. }
            | crate::workflow::CompiledNodeKind::ConcurrentGroupJoin { .. }
    };
}

pub(crate) use kinds;
