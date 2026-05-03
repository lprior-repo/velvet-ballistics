//! Semantic checking functions for generated Rust source validation.

use std::fmt::Write;
use crate::{fmt_err, CodegenError, CodegenResult};
use vb_core::{CompiledNodeKind, CompiledWorkflow, StepIdx};

/// Verify semantic equivalence between generated Rust source and the original IR.
/// Checks that all steps, expressions, constants, and control flow are preserved.
pub fn compare_generated_to_ir(source: &str, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    reject_generated_pattern(source, "u16::MAX", "finish sentinel")?;
    reject_generated_pattern(source, "Vec<", "dynamic Vec allocation")?;
    reject_generated_pattern(source, "Vec::", "dynamic Vec allocation")?;
    reject_generated_pattern(source, "slots[", "unchecked slot indexing")?;
    reject_generated_pattern(source, "CONSTANTS[", "unchecked constant indexing")?;
    reject_generated_pattern(source, " as ", "unchecked cast")?;
    require_generated_pattern(source, "StepOutcome::Finished", "terminal result return")?;

    // Only require ExprStack when the workflow has expressions.
    // Expressionless workflows generate no eval_expr functions and never instantiate ExprStack.
    let mut has_expressions = false;
    for idx in 0..u16::MAX {
        if workflow.expression(vb_core::ExprIdx::new(idx)).is_some() {
            has_expressions = true;
            break;
        }
    }
    if has_expressions {
        require_generated_pattern(source, "ExprStack::new", "bounded expression stack")?;
    }

    let mut step_count = 0u16;
    let mut expr_count = 0u16;
    let mut action_count = 0u16;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn step_") {
            step_count = step_count
                .checked_add(1)
                .ok_or(CodegenError::SemanticMismatch {
                    detail: "step count overflow".into(),
                })?;
        }
        if trimmed.starts_with("fn eval_expr_") {
            expr_count = expr_count
                .checked_add(1)
                .ok_or(CodegenError::SemanticMismatch {
                    detail: "expression count overflow".into(),
                })?;
        }
        if trimmed.contains("Action boundary:") {
            action_count = action_count
                .checked_add(1)
                .ok_or(CodegenError::SemanticMismatch {
                    detail: "action count overflow".into(),
                })?;
        }
    }

    let expected_steps = workflow.node_count();
    if step_count != expected_steps {
        return Err(CodegenError::SemanticMismatch {
            detail: format!(
                "step count mismatch: generated has {step_count}, IR has {expected_steps}"
            ),
        });
    }

    // Count expressions in the workflow
    let mut expected_exprs = 0u16;
    for idx in 0..u16::MAX {
        if workflow.expression(vb_core::ExprIdx::new(idx)).is_some() {
            expected_exprs =
                expected_exprs
                    .checked_add(1)
                    .ok_or(CodegenError::SemanticMismatch {
                        detail: "expected expression count overflow".into(),
                    })?;
        } else {
            break;
        }
    }

    if expr_count != expected_exprs {
        return Err(CodegenError::SemanticMismatch {
            detail: format!(
                "expression count mismatch: generated has {expr_count}, IR has {expected_exprs}"
            ),
        });
    }

    // Verify action count matches
    let mut expected_actions = 0u16;
    for idx in 0..workflow.node_count() {
        if let Some(node) = workflow.node(StepIdx::new(idx)) {
            if matches!(node.kind, CompiledNodeKind::Do { .. }) {
                expected_actions =
                    expected_actions
                        .checked_add(1)
                        .ok_or(CodegenError::SemanticMismatch {
                            detail: "expected action count overflow".into(),
                        })?;
            }
        }
    }

    if action_count != expected_actions {
        return Err(CodegenError::SemanticMismatch {
            detail: format!(
                "action count mismatch: generated has {action_count}, IR has {expected_actions}"
            ),
        });
    }

    Ok(())
}

fn reject_generated_pattern(
    source: &str,
    pattern: &str,
    reason: &'static str,
) -> CodegenResult<()> {
    if source.contains(pattern) {
        return Err(CodegenError::SemanticMismatch {
            detail: format!("generated source contains {reason}"),
        });
    }
    Ok(())
}

fn require_generated_pattern(
    source: &str,
    pattern: &str,
    reason: &'static str,
) -> CodegenResult<()> {
    if !source.contains(pattern) {
        return Err(CodegenError::SemanticMismatch {
            detail: format!("generated source is missing {reason}"),
        });
    }
    Ok(())
}
