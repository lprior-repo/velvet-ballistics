#![forbid(unsafe_code)]
//! Suspend step handler.

use crate::workflow::CompiledNode;

use super::{ReplayAction, SuspensionKind};

/// Returns a suspension action for non-deterministic nodes.
pub(super) fn replay_suspend(node: &CompiledNode, kind: SuspensionKind) -> ReplayAction {
    ReplayAction::Suspended {
        step: node.id,
        kind,
    }
}
