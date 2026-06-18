//! Reference table construction from workflow AST.
//!
//! Extracts declared names from inputs, vars, secrets, steps, and output-producing
//! steps into the shared `RefTables` used by the validator.

use crate::ast::{AstMapEntry, StepAst, WorkflowAst};
use crate::ast::StepKindAst;
use vb_validate::references::RefTables;

/// Builds reference lookup tables from a workflow AST.
pub(super) fn build_ref_tables(ast: &WorkflowAst) -> RefTables {
    let inputs = entry_names_owned(&ast.inputs);
    let vars = entry_names_owned(&ast.vars);
    let secrets = secret_names_owned(&ast.secrets);
    let step_ids = step_names_owned(&ast.steps);
    let step_outputs = output_step_names_owned(&ast.steps);
    RefTables::from_slices_with_outputs(&inputs, &vars, &secrets, &step_ids, &[], &step_outputs)
}

fn entry_names_owned<T>(entries: &[AstMapEntry<T>]) -> Vec<String> {
    let mut names = Vec::with_capacity(entries.len());
    for entry in entries {
        names.push(entry.name.as_ref().to_owned());
    }
    names
}

fn secret_names_owned(entries: &[AstMapEntry<Box<str>>]) -> Vec<String> {
    let mut names = Vec::with_capacity(entries.len());
    for entry in entries {
        names.push(entry.name.as_ref().to_owned());
    }
    names
}

fn step_names_owned(steps: &[StepAst]) -> Vec<String> {
    let mut names = Vec::with_capacity(steps.len());
    for step in steps {
        names.push(step.id.as_ref().to_owned());
    }
    names
}

fn output_step_names_owned(steps: &[StepAst]) -> Vec<String> {
    let mut names = Vec::with_capacity(steps.len());
    for step in steps {
        if step_kind_produces_output(&step.kind) {
            names.push(step.id.as_ref().to_owned());
        }
    }
    names
}

pub(super) fn step_kind_produces_output(kind: &StepKindAst) -> bool {
    matches!(
        kind,
        StepKindAst::Run { .. }
            | StepKindAst::Save { .. }
            | StepKindAst::ForEach { .. }
            | StepKindAst::Together { .. }
            | StepKindAst::Collect { .. }
            | StepKindAst::Reduce { .. }
            | StepKindAst::Repeat { .. }
            | StepKindAst::Ask { .. }
    )
}
