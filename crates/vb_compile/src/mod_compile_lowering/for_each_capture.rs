use crate::{CompileError, CompileErrors, StepAst, StepPrimitive};

pub(super) fn validate_for_each_capture(
    variable: &str,
    body: &[StepAst],
) -> Result<(), CompileErrors> {
    let enclosing = vec![variable.to_owned()];
    walk_body(body, &enclosing)
}

fn duplicate(name: &str) -> CompileErrors {
    CompileErrors(vec![CompileError::DuplicateOutputName {
        name: name.into(),
    }])
}

fn walk_body(body: &[StepAst], enclosing: &[String]) -> Result<(), CompileErrors> {
    for step in body {
        check_step(step, enclosing)?;
    }
    Ok(())
}

fn check_step(step: &StepAst, enclosing: &[String]) -> Result<(), CompileErrors> {
    if enclosing.iter().any(|item| item == &step.id) {
        return Err(duplicate(&step.id));
    }
    match &step.primitive {
        StepPrimitive::Set { output, .. } => check_binding(output, enclosing),
        StepPrimitive::ForEach { variable, body, .. } => {
            check_binding(variable, enclosing)?;
            descend(variable, body, enclosing)
        }
        StepPrimitive::Collect { variable, body, .. } => {
            check_binding(variable, enclosing)?;
            descend(variable, body, enclosing)
        }
        StepPrimitive::Repeat { body, .. } => walk_body(body, enclosing),
        StepPrimitive::Choose { branches, .. } => check_choose(branches, enclosing),
        StepPrimitive::Together { branches } => check_together(branches, enclosing),
        StepPrimitive::Aggregate { body, .. } => walk_body(body, enclosing),
        _ => Ok(()),
    }
}

fn check_binding(name: &str, enclosing: &[String]) -> Result<(), CompileErrors> {
    if enclosing.iter().any(|item| item == name) {
        Err(duplicate(name))
    } else {
        Ok(())
    }
}

fn descend(variable: &str, body: &[StepAst], enclosing: &[String]) -> Result<(), CompileErrors> {
    let mut nested = enclosing.to_vec();
    nested.push(variable.to_owned());
    walk_body(body, &nested)
}

fn check_choose(
    branches: &[crate::ChooseBranch],
    enclosing: &[String],
) -> Result<(), CompileErrors> {
    for branch in branches {
        walk_body(&branch.steps, enclosing)?;
    }
    Ok(())
}

fn check_together(
    branches: &[crate::TogetherBranch],
    enclosing: &[String],
) -> Result<(), CompileErrors> {
    for branch in branches {
        walk_body(&branch.steps, enclosing)?;
    }
    Ok(())
}
