//! Validation functions for generated Rust subset.

use vb_core::{CompiledWorkflow, CompiledNodeKind, ExprOp, AccessorIdx, StepIdx};

use crate::CodegenResult;

pub fn validate_generated_subset(workflow: &CompiledWorkflow) -> CodegenResult<()> {
    validate_generated_nodes(workflow)?;
    validate_generated_expressions(workflow)?;
    validate_generated_accessors(workflow)
}

fn validate_generated_nodes(workflow: &CompiledWorkflow) -> CodegenResult<()> {
    let mut step_idx = 0u16;
    while step_idx < workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step)
            && let Some(feature) = unsupported_node_feature(&node.kind)
        {
            return Err(CodegenError::UnsupportedIr { feature });
        }
        step_idx = step_idx.saturating_add(1);
    }
    Ok(())
}

fn validate_generated_expressions(workflow: &CompiledWorkflow) -> CodegenResult<()> {
    let mut expr_idx = 0u16;
    loop {
        let idx = vb_core::ExprIdx::new(expr_idx);
        let Some(program) = workflow.expression(idx) else {
            break;
        };
        for op in program.ops.as_ref() {
            if let Some(feature) = unsupported_expr_feature(*op) {
                return Err(CodegenError::UnsupportedIr { feature });
            }
        }
        if expr_idx == u16::MAX {
            break;
        }
        expr_idx = expr_idx.saturating_add(1);
    }
    Ok(())
}

fn validate_generated_accessors(workflow: &CompiledWorkflow) -> CodegenResult<()> {
    let mut accessor_idx = 0u16;
    loop {
        let idx = vb_core::AccessorIdx::new(accessor_idx);
        let Some(accessor) = workflow.accessor(idx) else {
            break;
        };
        if !accessor.path.is_empty() {
            return Err(CodegenError::UnsupportedIr {
                feature: "accessor traversal",
            });
        }
        if accessor_idx == u16::MAX {
            break;
        }
        accessor_idx = accessor_idx.saturating_add(1);
    }
    Ok(())
}

fn unsupported_node_feature(kind: &CompiledNodeKind) -> Option<&'static str> {
    match kind {
        CompiledNodeKind::ForEachStart { .. } => Some("ForEachStart"),
        CompiledNodeKind::ForEachNext { .. } => Some("ForEachNext"),
        CompiledNodeKind::ForEachJoin { .. } => Some("ForEachJoin"),
        CompiledNodeKind::TogetherStart { .. } => Some("TogetherStart"),
        CompiledNodeKind::TogetherBranch { .. } => Some("TogetherBranch"),
        CompiledNodeKind::TogetherJoin { .. } => Some("TogetherJoin"),
        CompiledNodeKind::CollectStart { .. } => Some("CollectStart"),
        CompiledNodeKind::CollectPage { .. } => Some("CollectPage"),
        CompiledNodeKind::CollectNext { .. } => Some("CollectNext"),
        CompiledNodeKind::CollectFinish { .. } => Some("CollectFinish"),
        CompiledNodeKind::ReduceStart { .. } => Some("ReduceStart"),
        CompiledNodeKind::ReduceNext { .. } => Some("ReduceNext"),
        CompiledNodeKind::ReduceFinish { .. } => Some("ReduceFinish"),
        CompiledNodeKind::RepeatStart { .. } => Some("RepeatStart"),
        CompiledNodeKind::RepeatAttempt { .. } => Some("RepeatAttempt"),
        CompiledNodeKind::RepeatCheck { .. } => Some("RepeatCheck"),
        CompiledNodeKind::RepeatFinish { .. } => Some("RepeatFinish"),
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::Choose { .. }
        | CompiledNodeKind::ChooseSlot { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::ErrorHandler { .. }
        | CompiledNodeKind::RetryCheck { .. }
        | CompiledNodeKind::Jump { .. }
        | CompiledNodeKind::Finish { .. } => None,
    }
}

