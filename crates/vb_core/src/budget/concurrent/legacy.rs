use super::{Boundary, node_at};
use crate::budget::BudgetTraversalError;
use crate::workflow::CompiledNode;
use crate::workflow::loop_regions::shapes::{control_shape, start_shape};
pub(super) fn valid_latch(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    boundary: Boundary,
) -> Result<bool, BudgetTraversalError> {
    let Boundary::Loop {
        owner, body, done, ..
    } = boundary
    else {
        return Ok(false);
    };
    let shape = match control_shape(node) {
        Some(s) => s,
        None => return Ok(false),
    };
    if shape.body != body || shape.done != done {
        return Ok(false);
    }
    Ok(start_shape(node_at(nodes, owner)?) == Some(shape) && node.id >= body && node.id < done)
}
