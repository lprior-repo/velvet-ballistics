#![forbid(unsafe_code)]
//! Reference-table data structures for workflow reference validation.
//!
//! [`RefTables`] is a lookup structure built from declared workflow names
//! (inputs, vars, secrets, step IDs, loop variables, and step outputs).
//! Downstream crates (e.g. `vb_compile`) build their own tables from AST
//! types and call [`validate_single_reference`] directly.
//!
//! [`WorkflowRefs`] is the input document model used by the top-level
//! [`validate_references`](super::validate_references) entry point.

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Workflow reference model
// ---------------------------------------------------------------------------

/// Workflow reference data used for reference validation.
#[derive(Debug, Clone, Default)]
pub struct WorkflowRefs {
    /// Declared input names.
    pub inputs: Vec<String>,
    /// Declared variable names.
    pub vars: Vec<String>,
    /// Declared secret names.
    pub secrets: Vec<String>,
    /// Declared step IDs (in order).
    pub step_ids: Vec<String>,
    /// All `$`-prefixed references found in the workflow.
    pub references: Vec<String>,
    /// Loop variable names in scope (from for_each, together, collect bodies).
    pub loop_variable_names: Vec<String>,
    /// Step IDs that produce a result output.
    ///
    /// When non-empty, [`RefTables::step_has_output`] uses this set to
    /// determine whether a `$steps.<step_id>.output` or `$step_id.output`
    /// reference is valid; references to steps not in this set produce
    /// [`crate::ValidationError::ResultReferenceMissing`]. When empty through
    /// this struct (the default), every step is treated as output-producing
    /// so the validator remains permissive for callers that have not yet
    /// wired output tracking. Call
    /// [`RefTables::from_slices_with_outputs`] to supply known output
    /// tracking where an empty slice means no steps produce output.
    pub step_outputs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Reference tables
// ---------------------------------------------------------------------------

/// Reference tables built from declared workflow names.
///
/// Public so that downstream crates (e.g. `vb_compile`) can build tables from
/// their own AST types and call [`super::validate_single_reference`] directly,
/// avoiding duplicate reference validation logic.
pub struct RefTables {
    pub(super) inputs: HashSet<String>,
    pub(super) vars: HashSet<String>,
    pub(super) secrets: HashSet<String>,
    step_ids: Vec<String>,
    step_ids_set: HashSet<String>,
    loop_variable_names: HashSet<String>,
    /// Set of step IDs that produce outputs. Used to validate that
    /// `$steps.<step_id>.output` references point to steps that actually
    /// produce an output. A step that does not produce an output is
    /// referenced at the user's peril: validation emits
    /// [`crate::ValidationError::ResultReferenceMissing`].
    pub(super) step_outputs: HashSet<String>,
    pub(super) step_outputs_known: bool,
}

impl RefTables {
    /// Builds reference tables from a [`WorkflowRefs`] document.
    pub fn build(workflow: &WorkflowRefs) -> Self {
        let step_ids = workflow.step_ids.clone();
        let step_ids_set = string_set(&workflow.step_ids);
        Self {
            inputs: string_set(&workflow.inputs),
            vars: string_set(&workflow.vars),
            secrets: string_set(&workflow.secrets),
            step_ids,
            step_ids_set,
            loop_variable_names: string_set(&workflow.loop_variable_names),
            step_outputs: string_set(&workflow.step_outputs),
            step_outputs_known: !workflow.step_outputs.is_empty(),
        }
    }

    /// Builds reference tables from individual name slices.
    ///
    /// This is the shared entry point used by `vb_compile` to avoid
    /// duplicating reference validation logic.
    pub fn from_slices(
        inputs: &[String],
        vars: &[String],
        secrets: &[String],
        step_ids: &[String],
    ) -> Self {
        Self::from_slices_with_loop_vars(inputs, vars, secrets, step_ids, &[])
    }

    /// Builds reference tables from individual name slices, including
    /// loop variable names that are in scope (for_each, together, collect).
    pub fn from_slices_with_loop_vars(
        inputs: &[String],
        vars: &[String],
        secrets: &[String],
        step_ids: &[String],
        loop_variable_names: &[String],
    ) -> Self {
        let step_ids_vec = step_ids.to_vec();
        let step_ids_set = string_set(step_ids);
        Self {
            inputs: string_set(inputs),
            vars: string_set(vars),
            secrets: string_set(secrets),
            step_ids: step_ids_vec,
            step_ids_set,
            loop_variable_names: string_set(loop_variable_names),
            step_outputs: HashSet::new(),
            step_outputs_known: false,
        }
    }

    /// Builds reference tables from individual name slices, loop variables,
    /// and the set of steps that can produce an `output` binding.
    pub fn from_slices_with_outputs(
        inputs: &[String],
        vars: &[String],
        secrets: &[String],
        step_ids: &[String],
        loop_variable_names: &[String],
        step_outputs: &[String],
    ) -> Self {
        let mut tables =
            Self::from_slices_with_loop_vars(inputs, vars, secrets, step_ids, loop_variable_names);
        tables.step_outputs = string_set(step_outputs);
        tables.step_outputs_known = true;
        tables
    }

    /// Returns whether the given name is a declared input.
    pub fn contains_input(&self, name: &str) -> bool {
        self.inputs.contains(name)
    }

    /// Returns whether the given name is a declared variable.
    pub fn contains_var(&self, name: &str) -> bool {
        self.vars.contains(name)
    }

    /// Returns whether the given name is a declared secret.
    pub fn contains_secret(&self, name: &str) -> bool {
        self.secrets.contains(name)
    }

    /// Returns whether the given name is a declared step ID.
    pub fn contains_step_id(&self, name: &str) -> bool {
        self.step_ids_set.contains(name)
    }

    /// Returns whether the given name is a loop variable in scope.
    pub fn contains_loop_variable(&self, name: &str) -> bool {
        self.loop_variable_names.contains(name)
    }

    /// Returns the index of the given step ID, or `None` if not found.
    pub fn step_index(&self, step_id: &str) -> Option<usize> {
        self.step_ids.iter().position(|id| id == step_id)
    }

    /// Returns whether the given step ID is declared to produce an output.
    ///
    /// When step-output tracking is supplied (via
    /// [`RefTables::from_slices_with_outputs`] or non-empty
    /// [`WorkflowRefs::step_outputs`]), this returns `true` only for steps
    /// that bind a result. A supplied-but-empty output set means no steps
    /// produce output. When tracking is not supplied, every step is treated as
    /// output-producing so callers that have not wired output tracking do not
    /// regress.
    pub fn step_has_output(&self, step_id: &str) -> bool {
        if !self.step_outputs_known {
            return true;
        }
        self.step_outputs.contains(step_id)
    }
}

fn string_set(names: &[String]) -> HashSet<String> {
    let mut set = HashSet::with_capacity(names.len());
    for name in names {
        set.insert(name.clone());
    }
    set
}
