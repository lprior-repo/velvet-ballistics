use super::SlotCompiler;
use crate::{CompileError, CompileErrors};
use vb_core::{CompiledNodeKind as K, ExprOp, InputSlotKind, SlotIdx};

/// Bound only the latest known producer; unknown control/data origins use the store cap.
pub(super) fn input_limit(
    mut source: SlotIdx,
    builder: &SlotCompiler,
) -> Result<u32, CompileErrors> {
    let hard_max = u32::try_from(vb_core::limits::MAX_LIST_ITEMS_PER_VALUE).map_err(|_| {
        CompileErrors(vec![CompileError::InvalidInputSchema {
            field: "inputs.max",
            expected: "list maximum representable as u32",
        }])
    })?;
    for node in builder.nodes.iter().rev() {
        if node.output.slot() != Some(source) {
            continue;
        }
        match &node.kind {
            K::Copy { source: input } => source = *input,
            K::EvalExpr { expr } => {
                let ops = builder
                    .expressions
                    .get(expr.as_usize())
                    .map(|p| p.ops.as_ref());
                if let Some([ExprOp::LoadSlot(input)]) = ops {
                    source = *input;
                } else {
                    return Ok(hard_max);
                }
            }
            K::BuildList { items } => {
                return u32::try_from(items.len()).map_err(|_| {
                    CompileErrors(vec![CompileError::StepIndexOutOfRange {
                        value: items.len(),
                    }])
                });
            }
            _ => return Ok(hard_max),
        }
    }
    Ok(builder
        .input_slots
        .iter()
        .find(|input| input.slot == source && input.kind == InputSlotKind::List)
        .and_then(|input| input.constraint.max)
        .and_then(|max| u32::try_from(max).ok())
        .map_or(hard_max, |max| max.min(hard_max)))
}

/// A direct lexical slot needs no prelude; accessors/expressions are evaluated once.
pub(super) fn source_width(input: &str) -> usize {
    let input = input.trim();
    if !input.is_empty() && input.bytes().all(|byte| byte.is_ascii_digit()) {
        return 0;
    }
    let Some(reference) = input.strip_prefix('$') else {
        return 1;
    };
    let mut segments = reference.split('.');
    let root = segments.next().unwrap_or("");
    let name = if matches!(root, "input" | "vars" | "secrets" | "step" | "steps") {
        segments.next().unwrap_or("")
    } else {
        root
    };
    usize::from(
        name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || segments.next().is_some(),
    )
}

pub(super) fn item_type(
    mut source: SlotIdx,
    builder: &SlotCompiler,
    types: &std::collections::HashMap<SlotIdx, vb_expr::expr_typecheck::ExprType>,
) -> vb_expr::expr_typecheck::ExprType {
    use vb_expr::expr_typecheck::ExprType as T;
    for node in builder.nodes.iter().rev() {
        if node.output.slot() != Some(source) {
            continue;
        }
        match &node.kind {
            K::Copy { source: input } => source = *input,
            K::EvalExpr { expr } => {
                if let Some([ExprOp::LoadSlot(input)]) = builder
                    .expressions
                    .get(expr.as_usize())
                    .map(|p| p.ops.as_ref())
                {
                    source = *input;
                } else {
                    return T::Unknown;
                }
            }
            K::BuildList { items } => {
                let mut result = None;
                for item in items {
                    let ty = types
                        .get(item)
                        .copied()
                        .unwrap_or_else(|| constant_type(*item, builder));
                    result = Some(match result {
                        None => ty,
                        Some(previous) if previous == ty => ty,
                        Some(T::I64 | T::F64) if matches!(ty, T::I64 | T::F64) => T::F64,
                        _ => return T::Unknown,
                    });
                }
                return result.unwrap_or(T::Unknown);
            }
            _ => return T::Unknown,
        }
    }
    builder
        .input_slots
        .iter()
        .find(|input| input.slot == source)
        .and_then(|input| input.constraint.element)
        .map_or(T::Unknown, |kind| match kind {
            InputSlotKind::I64 => T::I64,
            InputSlotKind::F64 => T::F64,
            InputSlotKind::Bool => T::Bool,
            InputSlotKind::Symbol => T::Text,
            InputSlotKind::List => T::List,
            InputSlotKind::Object => T::Object,
            _ => T::Unknown,
        })
}

fn constant_type(source: SlotIdx, builder: &SlotCompiler) -> vb_expr::expr_typecheck::ExprType {
    use vb_expr::expr_typecheck::ExprType as T;
    let Some(node) = builder
        .nodes
        .iter()
        .rev()
        .find(|node| node.output.slot() == Some(source))
    else {
        return T::Unknown;
    };
    let K::SetConst { value } = node.kind else {
        return T::Unknown;
    };
    match builder.constants.get(value.as_usize()) {
        Some(vb_core::ConstValue::I64(_)) => T::I64,
        Some(vb_core::ConstValue::F64(_)) => T::F64,
        Some(vb_core::ConstValue::Bool(_)) => T::Bool,
        Some(vb_core::ConstValue::Symbol(_)) => T::Text,
        Some(vb_core::ConstValue::Null) => T::Null,
        _ => T::Unknown,
    }
}

#[cfg(test)]
#[path = "list_input_tests.rs"]
mod tests;
