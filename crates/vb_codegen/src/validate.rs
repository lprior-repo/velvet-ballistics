#![forbid(unsafe_code)]

use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, ExprOp, WorkflowParts};

use crate::CodegenError;

const MAX_GENERATED_NODES: usize = 512;
const MAX_GENERATED_ACCESSOR_DEPTH: usize = 16;
const ERR_NODE_COUNT: &str = "node count exceeds generated subset";
const ERR_ACCESSOR_DEPTH: &str = "accessor depth exceeds generated subset";
const ERR_ACCESSOR_SLOT: &str = "slot bounds in generated accessor";
const ERR_CONTAINS: &str = "text helper contains requires runtime symbol store";
const ERR_STARTS_WITH: &str = "text helper starts_with requires runtime symbol store";
const ERR_ENDS_WITH: &str = "text helper ends_with requires runtime symbol store";
const ERR_EXPR_HELPER: &str = "expression helper requires runtime value store";
const ERR_NODE_KIND: &str = "node kind outside generated subset";

pub fn validate_generated_subset(workflow: &CompiledWorkflow) -> Result<(), CodegenError> {
    validate_parts(&workflow.to_parts())
}

fn validate_parts(parts: &WorkflowParts) -> Result<(), CodegenError> {
    validate_node_count(parts)?;
    validate_accessors(parts)?;
    validate_expressions(parts)?;
    validate_nodes(parts)
}

fn unsupported(feature: &'static str) -> CodegenError {
    CodegenError::UnsupportedIr { feature }
}

fn validate_node_count(parts: &WorkflowParts) -> Result<(), CodegenError> {
    if parts.nodes.len() <= MAX_GENERATED_NODES {
        Ok(())
    } else {
        Err(unsupported(ERR_NODE_COUNT))
    }
}

fn validate_accessors(parts: &WorkflowParts) -> Result<(), CodegenError> {
    let slot_count = usize::from(parts.slot_count);
    parts.accessors.iter().try_for_each(|accessor| {
        if accessor.root.as_usize() >= slot_count {
            Err(unsupported(ERR_ACCESSOR_SLOT))
        } else if accessor.path.len() > MAX_GENERATED_ACCESSOR_DEPTH {
            Err(unsupported(ERR_ACCESSOR_DEPTH))
        } else {
            Ok(())
        }
    })
}

fn validate_expressions(parts: &WorkflowParts) -> Result<(), CodegenError> {
    parts.expressions.iter().try_for_each(|expr| {
        expr.ops
            .iter()
            .copied()
            .find_map(unsupported_expr_feature)
            .map_or(Ok(()), |feature| Err(unsupported(feature)))
    })
}

fn unsupported_expr_feature(op: ExprOp) -> Option<&'static str> {
    match op {
        ExprOp::Contains => Some(ERR_CONTAINS),
        ExprOp::StartsWith => Some(ERR_STARTS_WITH),
        ExprOp::EndsWith => Some(ERR_ENDS_WITH),
        ExprOp::Has
        | ExprOp::Exists
        | ExprOp::Length
        | ExprOp::Empty
        | ExprOp::Append
        | ExprOp::AppendIf
        | ExprOp::Merge
        | ExprOp::Sum
        | ExprOp::Count
        | ExprOp::Unique => Some(ERR_EXPR_HELPER),
        _ => None,
    }
}

fn validate_nodes(parts: &WorkflowParts) -> Result<(), CodegenError> {
    parts.nodes.iter().try_for_each(|node| match node.kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::Finish { .. } => Ok(()),
        CompiledNodeKind::EvalExpr { expr } => parts
            .expressions
            .get(expr.as_usize())
            .map_or(Err(unsupported(ERR_NODE_KIND)), |_| Ok(())),
        _ => Err(unsupported(ERR_NODE_KIND)),
    })
}
