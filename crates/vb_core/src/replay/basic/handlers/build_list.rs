#![forbid(unsafe_code)]
//! BuildList step handler.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::SlotIdx;
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::ValueStore;

use super::shared;
use super::{ReplayAction, ReplayError};

/// Executes a BuildList node: assemble a list from item slots.
pub(super) fn replay_build_list(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &crate::workflow::CompiledNode,
    items: &[SlotIdx],
) -> Result<ReplayAction, ReplayError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(items.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < items.len() {
        let slot = items.get(index).ok_or(ReplayError::Internal {
            reason: "build_list item index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_list item slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(shared::slot_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        values.push(value);
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_list item index overflow",
        })?;
    }
    let handle =
        store
            .insert_list(values.into_boxed_slice())
            .map_err(|_| ReplayError::Internal {
                reason: "insert_list failed",
            })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildList node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::List(handle), accumulated_taint)
        .map_err(shared::slot_to_replay_err)?;
    let next = shared::advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}
