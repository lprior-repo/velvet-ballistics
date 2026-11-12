//! Total pure constructors for [`CompiledNode`] routing planes.
//!
//! The ~1800 node-construction sites name routing with exhaustive enum
//! literals. These free functions collapse that boilerplate: every site
//! supplies plain [`Option`] indices and the existing [`From`] bridges select
//! the routing variants. They are total (defined for every input) and pure
//! (no I/O, no panics, no indexing, no arithmetic, no casts).

use super::{
    CompiledNode, CompiledNodeKind, ErrorRouting, NodeContinuation, NodeOutput, NodeRouting,
};
use crate::ids::{SlotIdx, StepIdx};

/// Assembles a node with plain (error-free) routing.
///
/// `output` selects the [`NodeOutput`] plane: `None` discards the result,
/// `Some(slot)` writes it. `next` selects the [`NodeContinuation`] plane:
/// `None` ends the path, `Some(step)` falls through. Failures fail the run
/// ([`ErrorRouting::None`]).
///
/// ```
/// use vb_core::ids::StepIdx;
/// use vb_core::workflow::{CompiledNodeKind, node};
///
/// let built = node(StepIdx::new(0), CompiledNodeKind::Nop, None, Some(StepIdx::new(1)));
/// assert_eq!(built.id, StepIdx::new(0));
/// ```
#[must_use]
pub fn node(
    id: StepIdx,
    kind: CompiledNodeKind,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CompiledNode {
    CompiledNode {
        id,
        output: NodeOutput::from(output),
        routing: NodeRouting {
            next: NodeContinuation::from(next),
            error: ErrorRouting::None,
        },
        kind,
    }
}

/// Assembles a node that routes failures to a handler step.
///
/// Behaves like [`node`], except failures route to `handler` instead of
/// failing the run. `error_slot` names the slot receiving failure information
/// before routing, when set.
///
/// ```
/// use vb_core::ids::{SlotIdx, StepIdx};
/// use vb_core::workflow::{CompiledNodeKind, node_with_handler};
///
/// let built = node_with_handler(
///     StepIdx::new(0),
///     CompiledNodeKind::Nop,
///     None,
///     None,
///     StepIdx::new(1),
///     Some(SlotIdx::new(2)),
/// );
/// assert_eq!(built.routing.error.handler(), Some(StepIdx::new(1)));
/// ```
#[must_use]
pub fn node_with_handler(
    id: StepIdx,
    kind: CompiledNodeKind,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    handler: StepIdx,
    error_slot: Option<SlotIdx>,
) -> CompiledNode {
    CompiledNode {
        id,
        output: NodeOutput::from(output),
        routing: NodeRouting {
            next: NodeContinuation::from(next),
            error: ErrorRouting::Handler {
                handler,
                error_slot,
            },
        },
        kind,
    }
}
