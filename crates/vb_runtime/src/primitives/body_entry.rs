#![forbid(unsafe_code)]

//! Body entry policy: sealed trait with two ZST markers selecting
//! legacy (jump_to_body) vs deferred-owned (jump_to) dispatch.

use vb_core::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;

mod sealed {
    pub trait Sealed {}
}

pub(crate) trait BodyEntryPolicy: sealed::Sealed {
    fn jump(run: &mut RunFrame, body: StepIdx) -> Result<vb_core::EngineSignal, EngineError>;
}

pub(crate) struct LegacyEntry;
pub(crate) struct DeferredOwned;

impl sealed::Sealed for LegacyEntry {}
impl sealed::Sealed for DeferredOwned {}

impl BodyEntryPolicy for LegacyEntry {
    fn jump(run: &mut RunFrame, body: StepIdx) -> Result<vb_core::EngineSignal, EngineError> {
        super::helpers::jump_to_body(run, body)
    }
}

impl BodyEntryPolicy for DeferredOwned {
    fn jump(run: &mut RunFrame, body: StepIdx) -> Result<vb_core::EngineSignal, EngineError> {
        super::helpers::jump_to(run, body)
    }
}
