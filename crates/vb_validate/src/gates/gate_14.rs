#![forbid(unsafe_code)]
//! Gate 15: Determinism proof

use crate::{ValidationError, ValidationResult};
use vb_core::action::ActionContract;
use vb_core::capability::Capability;
use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{
use vb_core::span::Span;
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};

pub fn validate_gate_15_determinism_proof(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();

    for (node_index, node) in parts.nodes.iter().enumerate() {
        if !is_non_deterministic(&node.kind) {
            continue;
        }

        match node.next {
            Some(next_step) if next_step.as_usize() < node_count => {
                match parts.nodes.get(next_step.as_usize()) {
                    Some(next_node) if is_non_deterministic(&next_node.kind) => {
                        return Err(ValidationError::NonDeterministicPath {
                            from_node: node_index,
                            to_node: next_step.as_usize(),
                         span: Span::ZERO});
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn is_non_deterministic(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::Do { .. } | CompiledNodeKind::Ask { .. }
    )
}
