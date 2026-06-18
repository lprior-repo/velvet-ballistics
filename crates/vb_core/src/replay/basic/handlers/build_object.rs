#![forbid(unsafe_code)]
//! BuildObject step handler.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::SymbolId;
use crate::ids::SlotIdx;
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::ObjectField;
use crate::value_store::ValueStore;

use super::shared;
use super::{ReplayAction, ReplayError};

/// Executes a BuildObject node: assemble an object from field slots.
pub(super) fn replay_build_object(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &crate::workflow::CompiledNode,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<ReplayAction, ReplayError> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(fields.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < fields.len() {
        let (key, slot) = fields.get(index).ok_or(ReplayError::Internal {
            reason: "build_object field index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            EngineError::SlotUninitialized { slot: s } => {
                ReplayError::SlotNotAvailable { slot: s }
            }
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_object field slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(shared::slot_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        entries.push(ObjectField {
            key: *key,
            value,
            taint: slot_taint,
        });
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_object field index overflow",
        })?;
    }
    let handle = store
        .insert_object(entries.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "insert_object failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildObject node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::Object(handle), accumulated_taint)
        .map_err(shared::slot_to_replay_err)?;
    let next = shared::advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}
