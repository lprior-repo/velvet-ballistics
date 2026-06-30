#![forbid(unsafe_code)]

//! EngineSignal conversion helpers. Wraps the core `step_once` fallback
//! path so its `EngineSignal` output is converted into a runtime
//! signal before returning to the dispatcher.

use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::signal::runtime_from_core;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

/// Core fallback: any `CompiledNodeKind` that has not been given a
/// dedicated handler routes through `vb_core::engine::step_once`. The
/// resulting core `EngineSignal` is converted into a runtime signal so
/// the drive loop sees a single uniform value type.
pub(super) fn handle_core_step_once(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> RuntimeEngineResult<RuntimeSignal> {
    let cs = vb_core::engine::step_once(plan, run, store).map_err(RuntimeEngineError::Core)?;
    Ok(runtime_from_core(cs))
}
