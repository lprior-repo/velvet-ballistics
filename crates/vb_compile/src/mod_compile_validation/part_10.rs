//! Iterative step-tree checks: empty choose bodies and constant faults.

use crate::mod_compile_errors::CompileError;

/// Iterative checks over a step and every nested body/branch (explicit heap
/// stack instead of recursion, Rule 1). Children are pushed in reverse so
/// errors land in the same depth-first pre-order the recursive version
/// produced. Step IDs and outputs are lexically scoped, so nested
/// duplicates are shadowing, not errors.
///
/// Termination: each step is visited exactly once, so iterations are
/// bounded by the AST node count and stack depth never exceeds nesting
/// depth.
pub(super) fn check_step_tree(step: &crate::StepAst, errors: &mut Vec<CompileError>) {
    let mut stack = vec![step];
    while let Some(step) = stack.pop() {
        match &step.primitive {
            crate::StepPrimitive::Set { value, .. } => {
                check_constant_fault(step, value, errors);
            }
            crate::StepPrimitive::Choose {
                branches,
                otherwise,
            } => {
                if branches.is_empty() && otherwise.is_none() {
                    errors.push(CompileError::StepFieldShape {
                        step: 0,
                        field: "choose.branches",
                        expected: "at least one branch or an otherwise target",
                    });
                }
                for branch in branches.iter().rev() {
                    for nested in branch.steps.iter().rev() {
                        stack.push(nested);
                    }
                }
            }
            crate::StepPrimitive::Together { branches } => {
                for branch in branches.iter().rev() {
                    for nested in branch.steps.iter().rev() {
                        stack.push(nested);
                    }
                }
            }
            crate::StepPrimitive::ForEach { body, .. }
            | crate::StepPrimitive::Collect { body, .. }
            | crate::StepPrimitive::Aggregate { body, .. }
            | crate::StepPrimitive::Repeat { body, .. } => {
                for nested in body.iter().rev() {
                    stack.push(nested);
                }
            }
            _ => {}
        }
    }
}

/// A `set` value with no references that is shaped like arithmetic is fully
/// evaluated at compile time; a fault (division by zero, overflow) is a
/// compile error instead of a deferred runtime failure.
fn check_constant_fault(step: &crate::StepAst, value: &str, errors: &mut Vec<CompileError>) {
    let trimmed = value.trim();
    if trimmed.contains('$') {
        return;
    }
    let arithmetic = trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '(' || c == '-')
        && trimmed.chars().any(|c| matches!(c, '+' | '*' | '/' | '%'))
        || trimmed.contains(" - ");
    if !arithmetic {
        return;
    }
    let resolver = |_: &str| None;
    let Ok((program, constants)) = crate::compile_expr(trimmed, &resolver) else {
        return;
    };
    if let Err(error) = crate::eval_expr_program(&program, &[], &constants) {
        errors.push(CompileError::ConstantFault {
            step: 0,
            reason: Box::from(format!("step {} value {trimmed:?}: {error}", step.id)),
        });
    }
}
