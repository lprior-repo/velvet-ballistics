//! Unsupported step emission functions.

use std::fmt::Write;
use crate::{fmt_err, CodegenResult};
use vb_core::CompiledNodeKind;

pub fn emit_unsupported_node_step(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    let name = match kind {
        CompiledNodeKind::BuildObject { .. } => "BuildObject",
        CompiledNodeKind::BuildList { .. } => "BuildList",
        CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        CompiledNodeKind::CollectStart { .. } => "CollectStart",
        CompiledNodeKind::CollectPage { .. } => "CollectPage",
        CompiledNodeKind::CollectNext { .. } => "CollectNext",
        CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
        _ => "UnsupportedStep",
    };
    emit_unsupported_step(out, name)
}

pub fn emit_unsupported_step(out: &mut String, primitive: &'static str) -> CodegenResult<()> {
    writeln!(
        out,
        "    Err(DriveError::UnsupportedPrimitive {{ primitive: \"{primitive}\" }})"
    )
    .map_err(fmt_err)
}

pub fn emit_unsupported_expr(out: &mut String, op: &'static str) -> CodegenResult<()> {
    writeln!(
        out,
        "    return Err(DriveError::UnsupportedExpressionOp {{ op: \"{op}\" }});"
    )
    .map_err(fmt_err)
}
