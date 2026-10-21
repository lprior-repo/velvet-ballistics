#![forbid(unsafe_code)]
//! Top-level workflow AST data.

use super::{ExampleAst, InputField, ResultMapping, SecretField, StepAst, TriggerAst, VarField};

/// Top-level workflow AST produced by parsing a workflow YAML document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSource {
    /// Language version string (e.g. "velvet-ballistics/v1").
    pub(crate) version: String,
    /// Workflow name.
    pub(crate) name: String,
    /// Trigger declaration.
    pub(crate) trigger: TriggerAst,
    /// Declared input fields.
    pub(crate) inputs: Vec<InputField>,
    /// Declared workflow-level variables.
    pub(crate) vars: Vec<VarField>,
    /// Declared secret references.
    pub(crate) secrets: Vec<SecretField>,
    /// Ordered step list.
    pub(crate) steps: Vec<StepAst>,
    /// Optional result mapping.
    pub(crate) result: Option<ResultMapping>,
    /// Inline examples / test cases.
    pub(crate) examples: Vec<ExampleAst>,
}

impl WorkflowSource {
    /// Production-visible constructor: restricted to crate-internal use.
    #[doc(hidden)]
    #[cfg(not(any(test, feature = "test-util")))]
    pub(crate) fn new(parts: WorkflowSourceParts) -> Self {
        Self::from_parts(parts)
    }

    /// Test-visible constructor: publicly exported when the `test-util`
    /// feature is active or `cfg(test)` is enabled.
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-util"))]
    pub fn new(parts: WorkflowSourceParts) -> Self {
        Self::from_parts(parts)
    }

    fn from_parts(parts: WorkflowSourceParts) -> Self {
        Self {
            version: parts.version,
            name: parts.name,
            trigger: parts.trigger,
            inputs: parts.inputs,
            vars: parts.vars,
            secrets: parts.secrets,
            steps: parts.steps,
            result: parts.result,
            examples: parts.examples,
        }
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn trigger(&self) -> &TriggerAst {
        &self.trigger
    }

    #[must_use]
    pub fn inputs(&self) -> &[InputField] {
        &self.inputs
    }

    #[must_use]
    pub fn vars(&self) -> &[VarField] {
        &self.vars
    }

    #[must_use]
    pub fn secrets(&self) -> &[SecretField] {
        &self.secrets
    }

    #[must_use]
    pub fn steps(&self) -> &[StepAst] {
        &self.steps
    }

    #[must_use]
    pub fn result(&self) -> Option<&ResultMapping> {
        self.result.as_ref()
    }

    #[must_use]
    pub fn examples(&self) -> &[ExampleAst] {
        &self.examples
    }
}

/// Parts bundle for constructing a [`WorkflowSource`].
#[doc(hidden)]
#[cfg(not(any(test, feature = "test-util")))]
pub(crate) struct WorkflowSourceParts {
    /// Language version string.
    pub(crate) version: String,
    /// Workflow name.
    pub(crate) name: String,
    /// Trigger declaration.
    pub(crate) trigger: TriggerAst,
    /// Declared input fields.
    pub(crate) inputs: Vec<InputField>,
    /// Declared workflow-level variables.
    pub(crate) vars: Vec<VarField>,
    /// Declared secret references.
    pub(crate) secrets: Vec<SecretField>,
    /// Ordered step list.
    pub(crate) steps: Vec<StepAst>,
    /// Optional result mapping.
    pub(crate) result: Option<ResultMapping>,
    /// Inline examples / test cases.
    pub(crate) examples: Vec<ExampleAst>,
}

#[doc(hidden)]
#[cfg(any(test, feature = "test-util"))]
pub struct WorkflowSourceParts {
    /// Language version string.
    pub version: String,
    /// Workflow name.
    pub name: String,
    /// Trigger declaration.
    pub trigger: TriggerAst,
    /// Declared input fields.
    pub inputs: Vec<InputField>,
    /// Declared workflow-level variables.
    pub vars: Vec<VarField>,
    /// Declared secret references.
    pub secrets: Vec<SecretField>,
    /// Ordered step list.
    pub steps: Vec<StepAst>,
    /// Optional result mapping.
    pub result: Option<ResultMapping>,
    /// Inline examples / test cases.
    pub examples: Vec<ExampleAst>,
}
