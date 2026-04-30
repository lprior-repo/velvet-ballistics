use crate::CompileError;
use crate::ast::{StepAst, StepKindAst, WorkflowAst};

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileError> {
    let table = StepTable::new(ast);
    validate_targets(&table)?;
    validate_reachability(&table)
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

fn validate_targets(table: &StepTable<'_>) -> Result<(), CompileError> {
    for (index, step) in table.steps.iter().enumerate() {
        if let StepKindAst::Choose {
            on_true, on_false, ..
        } = step.kind
        {
            validate_target(table, index, on_true.as_usize())?;
            validate_target(table, index, on_false.as_usize())?;
        }
    }
    Ok(())
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

fn validate_reachability(table: &StepTable<'_>) -> Result<(), CompileError> {
    if table.is_empty() {
        return Ok(());
    }

    let mut reachable = vec![false; table.len()];
    mark_reachable(table, &mut reachable)?;
    reject_unreachable(&reachable)
}

fn mark_reachable(table: &StepTable<'_>, reachable: &mut [bool]) -> Result<(), CompileError> {
    let mut stack = Vec::with_capacity(table.len());
    stack.push(0_usize);

    while let Some(index) = stack.pop() {
        if mark_seen(reachable, index)? {
            push_successors(table, index, &mut stack);
        }
    }

    Ok(())
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
        Some(StepKindAst::Save { .. }) => push_next(table, index, stack),
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
