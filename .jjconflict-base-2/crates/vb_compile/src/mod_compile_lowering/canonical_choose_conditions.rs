//! Typed expression and legacy numeric-slot Choose conditions.

use super::*;
use vb_core::workflow::node;

pub(super) fn choose_node(
    id: StepIdx,
    branches: &[crate::ChooseBranch],
    targets: &[StepIdx],
    otherwise: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<CompiledNode, CompileErrors> {
    if branches
        .iter()
        .all(|branch| branch.when.parse::<i64>().is_ok())
    {
        let slots = branches
            .iter()
            .zip(targets)
            .map(|(branch, target)| {
                let condition =
                    slot_from_text(&branch.when, id.as_usize(), "choose.branches[].when")?;
                require_boolean_type(scope.get_type(condition))?;
                Ok(SlotBranch {
                    condition,
                    target: *target,
                })
            })
            .collect::<Result<Vec<_>, CompileErrors>>()?;
        return lower_choose(id, slots, otherwise, builder).map_err(|e| CompileErrors(vec![e]));
    }
    let mut expressions = Vec::with_capacity(branches.len());
    for (branch, target) in branches.iter().zip(targets) {
        let condition = choose_condition(&branch.when, scope, builder)?;
        expressions.push(vb_core::ExprBranch {
            condition,
            target: *target,
        });
    }
    Ok(node(
        id,
        CompiledNodeKind::Choose {
            branches: expressions.into_boxed_slice(),
            otherwise: otherwise.into(),
        },
        None,
        None,
    ))
}

fn choose_condition(
    value: &str,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<vb_core::ExprIdx, CompileErrors> {
    // Preserve the existing numeric-slot dialect even in a mixed branch table.
    if value.parse::<i64>().is_ok() {
        let slot = slot_from_text(value, 0, "choose.branches[].when")?;
        require_boolean_type(scope.get_type(slot))?;
        builder.record_slot(slot);
        let program = vb_core::ExprProgram::try_from_ops(
            vec![vb_core::ExprOp::LoadSlot(slot)].into_boxed_slice(),
        )
        .map_err(|e| CompileErrors(vec![CompileError::Workflow(WorkflowError::Expression(e))]))?;
        return builder
            .push_expression(program)
            .map_err(|e| CompileErrors(vec![e]));
    }
    let (expr, ty) = compile_canonical_expression(value, scope, builder)?;
    require_boolean_type(Some(ty))?;
    Ok(expr)
}

fn require_boolean_type(
    ty: Option<vb_expr::expr_typecheck::ExprType>,
) -> Result<(), CompileErrors> {
    if !matches!(
        ty,
        None | Some(
            vb_expr::expr_typecheck::ExprType::Bool | vb_expr::expr_typecheck::ExprType::Unknown
        )
    ) {
        return Err(CompileErrors(vec![CompileError::TypeMismatch {
            field: "choose.branches[].when",
            expected: "boolean",
            found: "non-boolean expression",
        }]));
    }
    Ok(())
}
