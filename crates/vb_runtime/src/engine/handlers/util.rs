#![forbid(unsafe_code)]

//! Utility helpers shared across handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;

use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult};

/// Reads an attempt count from a slot as a u16.
///
/// Returns 0 if the slot is uninitialized, otherwise attempts to
/// interpret the i64 value as a non-negative u16.
pub(crate) fn read_attempt_from_slot(run: &RunFrame, slot: SlotIdx) -> RuntimeEngineResult<u16> {
    match run.read_slot(slot) {
        Ok(value) => match *value {
            SlotValue::I64(v) => u16::try_from(v).map_err(|_| {
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
        Err(EngineError::SlotUninitialized { .. }) => Ok(0),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}
