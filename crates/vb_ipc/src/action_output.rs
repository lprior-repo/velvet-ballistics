#![forbid(unsafe_code)]
//! IPC action output types.

use serde::{Deserialize, Serialize};
use vb_core::action::ActionOutputReady;
use vb_core::ids::SlotIdx;
use vb_core::value::{SlotValue, Taint};

/// Typed IPC action output payload carried by `CompleteAction`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcActionOutputPayload {
    /// Output slot receiving the action result.
    pub output_slot: SlotIdx,
    /// Runtime value produced by the action.
    pub value: SlotValue,
    /// Taint attached to the result.
    pub taint: Taint,
}

impl IpcActionOutputPayload {
    /// Converts the wire payload into the runtime completion shape.
    pub fn into_action_output(self, encoded_len: u32) -> ActionOutputReady {
        ActionOutputReady {
            output_slot: self.output_slot,
            value: self.value,
            taint: self.taint,
            encoded_len,
        }
    }
}
