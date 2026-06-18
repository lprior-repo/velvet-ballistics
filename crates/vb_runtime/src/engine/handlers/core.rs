#![forbid(unsafe_code)]

//! Core engine fallback: delegates to vb_core::engine::step_once.

use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

pub(crate) fn handle_core_step_once(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> RuntimeEngineResult<RuntimeSignal> {
    let cs = vb_core::engine::step_once(plan, run, store).map_err(RuntimeEngineError::Core)?;
    Ok(runtime_from_core(cs))
}
