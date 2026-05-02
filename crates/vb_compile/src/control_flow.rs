//! Control-flow validation for compiled workflow ASTs.
//!
//! Validates choose branch targets, rejects backward branches (cycles), and
//! checks that all steps are reachable from the workflow entry.
//!
//! NOTE: This module performs the same *logical* checks as
//! `vb_validate::control_flow`, but operates on the compiler's `StepKindAst`
//! types with structured error diagnostics. The reference validation logic is
//! shared through `vb_validate::references::RefTables`; control-flow
//! validation remains compile-local because it needs structured step/target
//! indices that the standalone validator's string-based error model cannot
//! represent.

use crate::ast::{StepAst, StepKindAst, WorkflowAst};
use crate::{CompileError, CompileErrors, collect};

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors> {
    let table = StepTable::new(ast);
    let mut errors = Vec::new();
    validate_targets(&table, &mut errors);
    validate_reachability(&table, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

struct StepTable<'a> {
    steps: &'a [StepAst],
}

impl<'a> StepTable<'a> {
    fn new(ast: &'a WorkflowAst) -> Self {
        Self { steps: &ast.steps }
    }

    fn len(&self) -> usize {
        self.steps.len()
    }

    fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    fn kind(&self, index: usize) -> Option<&'a StepKindAst> {
        self.steps.get(index).map(|step| &step.kind)
    }

    fn contains_index(&self, index: usize) -> bool {
        index < self.steps.len()
    }
}

fn validate_targets(table: &StepTable<'_>, errors: &mut Vec<CompileError>) {
    for (index, step) in table.steps.iter().enumerate() {
        if let StepKindAst::Choose {
            on_true, on_false, ..
        } = step.kind
        {
            collect(errors, validate_target(table, index, on_true.as_usize()));
            collect(errors, validate_target(table, index, on_false.as_usize()));
        }
    }
}

fn validate_target(table: &StepTable<'_>, step: usize, target: usize) -> Result<(), CompileError> {
    if !table.contains_index(target) {
        return Err(CompileError::UnknownStepTarget { step, target });
    }
    if target <= step {
        return Err(CompileError::BackwardBranchTarget { step, target });
    }

    Ok(())
}

fn validate_reachability(table: &StepTable<'_>, errors: &mut Vec<CompileError>) {
    if table.is_empty() {
        return;
    }

    let mut reachable = vec![false; table.len()];
    mark_reachable(table, &mut reachable, errors);
    collect(errors, reject_unreachable(&reachable));
}

fn mark_reachable(table: &StepTable<'_>, reachable: &mut [bool], errors: &mut Vec<CompileError>) {
    let mut stack = Vec::with_capacity(table.len());
    stack.push(0_usize);

    while let Some(index) = stack.pop() {
        match mark_seen(reachable, index) {
            Ok(true) => push_successors(table, index, &mut stack),
            Ok(false) => {}
            Err(error) => errors.push(error),
        }
    }
}

fn mark_seen(reachable: &mut [bool], index: usize) -> Result<bool, CompileError> {
    let Some(seen) = reachable.get_mut(index) else {
        return Err(CompileError::UnknownStepTarget {
            step: index,
            target: index,
        });
    };
    if *seen {
        Ok(false)
    } else {
        *seen = true;
        Ok(true)
    }
}

fn push_successors(table: &StepTable<'_>, index: usize, stack: &mut Vec<usize>) {
    match table.kind(index) {
        Some(
            StepKindAst::Run { .. }
            | StepKindAst::Save { .. }
            | StepKindAst::ForEach { .. }
            | StepKindAst::Collect { .. }
            | StepKindAst::Reduce { .. }
            | StepKindAst::Repeat { .. }
            | StepKindAst::Wait { .. }
            | StepKindAst::Ask { .. },
        ) => {
            push_next(table, index, stack);
        }
        Some(StepKindAst::Together { branches }) => {
            push_next(table, index, stack);
            let mut branch_index = 0usize;
            while branch_index < branches.len() {
                if let Some(branch) = branches.get(branch_index) {
                    stack.push(branch.as_usize());
                }
                match branch_index.checked_add(1) {
                    Some(next) => branch_index = next,
                    None => return,
                }
            }
        }
        Some(StepKindAst::Choose {
            on_true, on_false, ..
        }) => {
            stack.push(on_false.as_usize());
            stack.push(on_true.as_usize());
        }
        Some(StepKindAst::Finish { .. }) | None => {}
    }
}

fn push_next(table: &StepTable<'_>, index: usize, stack: &mut Vec<usize>) {
    if let Some(next) = index.checked_add(1).filter(|next| *next < table.len()) {
        stack.push(next);
    }
}

fn reject_unreachable(reachable: &[bool]) -> Result<(), CompileError> {
    for (index, is_reachable) in reachable.iter().enumerate() {
        if !is_reachable {
            return Err(CompileError::UnreachableStep { step: index });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
