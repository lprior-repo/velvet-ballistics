#![forbid(unsafe_code)]

//! Per-node-kind execution handlers. Each `CompiledNodeKind` variant has
//! a small `handle_*` helper that wraps the corresponding primitive
//! (for_each, together, collect, reduce, repeat, wait/ask, do, error
//! handler) and translates its result into a runtime signal.
//!
//! Helpers are split across `handlers_compound` (for_each + together +
//! collect + reduce + repeat) and `handlers_suspend` (wait/ask + do +
//! error handler + attempt-slot reader) so each sub-module stays under
//! the 300-line drift ceiling.

use vb_core::errors::{CoreError, EngineError};
use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;

use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult};

/// Reads the current attempt counter for a retry policy. RE-003:
/// returns `Ok(None)` on an uninitialized slot rather than collapsing to
/// a silent 0; the caller decides how to interpret the absence.
pub(super) fn read_attempt_from_slot(run: &RunFrame, slot: SlotIdx) -> RuntimeEngineResult<Option<u16>> {
    match run.read_slot(slot) {
        Ok(value) => match *value {
            SlotValue::I64(v) => u16::try_from(v).map(Some).map_err(|_| {
                RuntimeEngineError::Core(EngineError::TypeMismatch {
                    expected: "non-negative u16 attempt count",
                    found: "out-of-range i64",
                })
            }),
            _ => Err(RuntimeEngineError::Core(EngineError::TypeMismatch {
                expected: "number",
                found: value.type_name(),
            })),
        },
        // RE-003: an uninitialized policy slot must NOT collapse to a
        // silent 0 attempt count. Surface the absence as `None` so the
        // caller (`handle_retry_check`) can explicitly treat it as the
        // first attempt AND advance the counter via in-handler write-back.
        Err(CoreError::SlotUninitialized { .. }) => Ok(None),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}