//! Materialize cold author values into numeric slots and immutable symbols.

use super::*;
use crate::{AuthorValue, CompileError, CompileErrors};
use vb_core::{CompiledNodeKind, ConstValue, SlotIdx, workflow::node};
use vb_expr::expr_typecheck::ExprType;

pub(super) fn width(value: &AuthorValue) -> Result<usize, CompileError> {
    let children = match value {
        AuthorValue::Sequence(items) => items.iter().map(width).collect::<Result<Vec<_>, _>>()?,
        AuthorValue::Mapping(fields) => fields
            .iter()
            .map(|field| width(&field.value))
            .collect::<Result<Vec<_>, _>>()?,
        _ => Vec::new(),
    };
    children.into_iter().try_fold(1usize, |total, child| {
        total
            .checked_add(child)
            .ok_or(CompileError::StepIndexOutOfRange { value: total })
    })
}

pub(super) fn emit(
    value: &AuthorValue,
    references: bool,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, ExprType), CompileErrors> {
    let (kind, inferred) = match value {
        AuthorValue::Sequence(items) => {
            let slots = items
                .iter()
                .map(|item| emit(item, references, scope, builder).map(|value| value.0))
                .collect::<Result<Vec<_>, _>>()?;
            (
                CompiledNodeKind::BuildList {
                    items: slots.into_boxed_slice(),
                },
                ExprType::List,
            )
        }
        AuthorValue::Mapping(entries) => {
            let mut fields = Vec::with_capacity(entries.len());
            for entry in entries {
                let key = builder
                    .intern(&entry.key)
                    .map_err(|e| CompileErrors(vec![e]))?;
                let (value, _) = emit(&entry.value, references, scope, builder)?;
                fields.push((key, value));
            }
            fields.sort_unstable_by_key(|(key, _)| *key);
            (
                CompiledNodeKind::BuildObject {
                    fields: fields.into_boxed_slice(),
                },
                ExprType::Object,
            )
        }
        AuthorValue::Text(text) if references && text.starts_with('$') => {
            let (expr, inferred) = compile_canonical_expression(text, scope, builder)?;
            (CompiledNodeKind::EvalExpr { expr }, inferred)
        }
        _ => {
            let (value, ty) = match value {
                AuthorValue::Null => (ConstValue::Null, ExprType::Null),
                AuthorValue::Bool(value) => (ConstValue::Bool(*value), ExprType::Bool),
                AuthorValue::I64(value) => (ConstValue::I64(*value), ExprType::I64),
                AuthorValue::Text(text) => (
                    ConstValue::Symbol(builder.intern(text).map_err(|e| CompileErrors(vec![e]))?),
                    ExprType::Text,
                ),
                _ => {
                    return Err(CompileErrors(vec![
                        CompileError::ExpressionLoweringUnsupported {
                            feature: "author value".into(),
                        },
                    ]));
                }
            };
            let value = builder
                .push_constant(value)
                .map_err(|e| CompileErrors(vec![e]))?;
            (CompiledNodeKind::SetConst { value }, ty)
        }
    };
    let slot = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let id = step_idx(builder.nodes.len()).map_err(|e| CompileErrors(vec![e]))?;
    let next = checked_step_offset(id, 1, "value", "next").map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node(id, kind, Some(slot), Some(next)));
    Ok((slot, inferred))
}
