//! DFS frame for iterative graph traversal.

use crate::errors::CoreError;
use crate::root_bundle::errors::{ArenaNode, OwnedRootBundleError};
use crate::value::Taint;

use super::super::arena::invariant;
use super::super::{Color, State};

/// DFS frame tracking traversal state for a single node.
pub(super) struct Frame {
    pub(super) node: ArenaNode,
    pub(super) cursor: usize,
    pub(super) max_child_height: u32,
    pub(super) taint: Taint,
}

impl Frame {
    /// Create a new frame for the given node.
    pub(super) fn new(node: ArenaNode) -> Self {
        Self {
            node,
            cursor: 0,
            max_child_height: 0,
            taint: Taint::Clean,
        }
    }

    /// Create a new stack with the root frame.
    pub(super) fn stack(node: ArenaNode) -> Result<Vec<Frame>, OwnedRootBundleError> {
        let mut stack = Vec::new();
        stack
            .try_reserve_exact(64)
            .map_err(|_| OwnedRootBundleError::Checkpoint(CoreError::AllocationFailed))?;
        stack.push(Frame::new(node));
        Ok(stack)
    }

    /// Compute the frame's height with overflow and depth checks.
    pub(super) fn checked_height(&self) -> Result<u32, OwnedRootBundleError> {
        let height = self.max_child_height.checked_add(1).ok_or(invariant())?;

        if height > 64 {
            return Err(OwnedRootBundleError::GraphDepth {
                node: self.node,
                depth: height,
            });
        }

        Ok(height)
    }

    /// Create the completed state for this frame.
    pub(super) fn completed_state(&self, height: u32) -> State {
        State {
            color: Color::Black,
            height,
            taint: self.taint,
        }
    }
}
