#![forbid(unsafe_code)]

//! Error handler node: redirects execution to an error-handling body.

use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;

use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

pub(crate) fn handle_error_handler(
    run: &mut RunFrame,
    handler_body: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    run.set_pc(handler_body).map_err(RuntimeEngineError::Core)?;
    run.increment_executed().map_err(RuntimeEngineError::Core)?;
    Ok(RuntimeSignal::Continue)
}
