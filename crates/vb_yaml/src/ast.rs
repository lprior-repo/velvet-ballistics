//! Typed AST for the workflow definition language.
//!
//! This module provides [`WorkflowSource`] and its supporting types, representing
//! a fully-parsed workflow YAML document. The [`parse_workflow_ast`] function
//! converts raw YAML text into this typed structure after profile validation.

use saphyr::LoadableYamlNode;

use crate::{YamlError, YamlResult};

// ---------------------------------------------------------------------------
// Top-level workflow AST
// ---------------------------------------------------------------------------

/// Top-level workflow AST produced by parsing a workflow YAML document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSource {
    /// Language version string (e.g. "velvet-ballastics/v1").
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

// ---------------------------------------------------------------------------
// Trigger
// ---------------------------------------------------------------------------

/// Trigger declaration: manual invocation or IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerAst {
    /// Manual trigger (default).
    Manual,
    /// IPC trigger with a named channel.
    Ipc {
        /// Channel name.
        name: String,
    },
}

// ---------------------------------------------------------------------------
// Step
// ---------------------------------------------------------------------------

/// A single workflow step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepAst {
    /// Unique step identifier.
    pub id: String,
    /// Human-readable name (optional).
    pub name: Option<String>,
    /// Condition expression for conditional execution.
    pub condition: Option<String>,
    /// The primitive operation.
    pub primitive: StepPrimitive,
    /// Resource / connector reference (optional).
    pub with: Option<String>,
    /// Retry policy (optional).
    pub retry: Option<RetryPolicy>,
    /// Error handler (optional).
    pub on_error: Option<ErrorHandlerAst>,
    /// Next-step label for explicit flow control (optional).
    pub then: Option<String>,
}

/// The concrete primitive operation for a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepPrimitive {
    /// Set a variable to a value.
    Set {
        /// Target variable name.
        output: String,
        /// Value expression.
        value: String,
    },
    /// Execute an action.
    Do {
        /// Action identifier.
        action: String,
        /// Input expression.
        input: String,
    },
    /// Branching construct.
    Choose {
        /// Branch list.
        branches: Vec<ChooseBranch>,
        /// Default branch label (optional).
        otherwise: Option<String>,
    },
    /// Parallel fan-out.
    ForEach {
        /// Loop variable name.
        variable: String,
        /// Input collection expression.
        input: String,
        /// Maximum concurrency (optional).
        at_once: Option<u32>,
        /// Body steps.
        body: Vec<StepAst>,
    },
    /// Concurrent branches that run together.
    Together {
        /// Branch list.
        branches: Vec<TogetherBranch>,
    },
    /// Paginated collection loop.
    Collect {
        /// Loop variable name.
        variable: String,
        /// Source expression.
        source: String,
        /// Maximum pages (optional).
        pages: Option<u32>,
        /// Items per page (optional).
        items: Option<u32>,
        /// Body steps.
        body: Vec<StepAst>,
    },
    /// Left-fold reduction.
    Reduce {
        /// Accumulator variable name.
        variable: String,
        /// Input collection expression.
        input: String,
        /// Initial value expression.
        initial: String,
        /// Body steps.
        body: Vec<StepAst>,
    },
    /// Retry loop.
    Repeat {
        /// Maximum retry attempts.
        max_attempts: u16,
        /// Body steps.
        body: Vec<StepAst>,
    },
    /// Wait for an event or timeout.
    Wait {
        /// Event expression (optional).
        event: Option<String>,
        /// Timeout expression (optional).
        timeout: Option<String>,
    },
    /// Ask for human input.
    Ask {
        /// Prompt text.
        prompt: String,
        /// Timeout expression (optional).
        timeout: Option<String>,
    },
    /// Terminate the workflow with a result.
    Finish {
        /// Result expression.
        result: String,
    },
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// A branch inside a `Choose` primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooseBranch {
    /// Condition label (the "when" field).
    pub when: String,
    /// Steps to execute when the condition matches.
    pub steps: Vec<StepAst>,
}

/// A branch inside a `Together` primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TogetherBranch {
    /// Branch label.
    pub label: String,
    /// Steps to execute in this branch.
    pub steps: Vec<StepAst>,
}

/// Retry policy for a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum retry attempts.
    pub max_attempts: u16,
    /// Delay between retries (expression or duration string).
    pub delay: Option<String>,
}

/// Error handler attached to a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorHandlerAst {
    /// Handler label or step reference.
    pub handler: String,
}

/// An input field declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputField {
    /// Field name.
    pub name: String,
    /// Field type annotation (optional).
    pub field_type: Option<String>,
    /// Default value expression (optional).
    pub default: Option<String>,
}

/// A variable field declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarField {
    /// Variable name.
    pub name: String,
    /// Initial value expression.
    pub value: Option<String>,
}

/// A secret reference declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretField {
    /// Secret name.
    pub name: String,
    /// External key path (optional).
    pub key: Option<String>,
}

/// Result mapping at the end of a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultMapping {
    /// Result expression.
    pub value: String,
}

/// An inline example / test case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleAst {
    /// Example description.
    pub description: Option<String>,
    /// Input bindings for the example.
    pub input: Option<String>,
    /// Expected result expression.
    pub expected: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse YAML text into a [`WorkflowSource`] AST.
///
/// This is a low-level function. Prefer [`crate::parse_workflow_source`] which
/// runs profile validation first.
pub fn parse_workflow_ast(text: &str) -> YamlResult<WorkflowSource> {
    let docs = saphyr::Yaml::load_from_str(text).map_err(|e| YamlError::ParseError {
        line: e.marker().line(),
        reason: e.info().into(),
    })?;

    let root = docs.into_iter().next().ok_or(YamlError::EmptySource)?;
    parse_workflow_from_yaml(&root)
}

/// Parse a single workflow document from a loaded saphyr Yaml node.
fn parse_workflow_from_yaml(root: &saphyr::Yaml<'_>) -> YamlResult<WorkflowSource> {
    if !root.is_mapping() {
        return Err(YamlError::FieldShape {
            field: "workflow",
            expected: "mapping",
        });
    }

    let version = require_str(root, "version")?;
    let name = require_str(root, "name")?;
    let trigger = parse_trigger(root)?;
    let inputs = parse_inputs(root)?;
    let vars = parse_vars(root)?;
    let secrets = parse_secrets(root)?;
    let steps = parse_steps(root)?;
    let result = parse_result(root)?;
    let examples = parse_examples(root)?;

    Ok(WorkflowSource {
        version,
        name,
        trigger,
        inputs,
        vars,
        secrets,
        steps,
        result,
        examples,
    })
}

// ---------------------------------------------------------------------------
// Helpers that work on &Yaml<'_> using as_mapping_get
//
// The `Yaml` type provides `as_mapping_get(&str)` which returns
// `Option<&Yaml>` and handles lifetime bridging internally. Missing keys
// yield `None`.
// ---------------------------------------------------------------------------

/// Look up a key in a mapping node. Returns `None` for absent keys.
fn lookup<'a>(node: &'a saphyr::Yaml<'_>, key: &str) -> Option<&'a saphyr::Yaml<'a>> {
    node.as_mapping_get(key)
}

/// Require a non-empty string field.
fn require_str(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<String> {
    match lookup(node, field) {
        None => Err(YamlError::MissingField { field }),
        Some(v) => match v.as_str() {
            Some(s) if !s.is_empty() => Ok(s.to_string()),
            _ => Err(YamlError::FieldShape {
                field,
                expected: "non-empty string",
            }),
        },
    }
}

/// Require a non-empty string from a sub-node, with a context label.
fn require_str_in(
    node: &saphyr::Yaml<'_>,
    field: &str,
    context: &'static str,
) -> YamlResult<String> {
    match lookup(node, field) {
        None => Err(YamlError::MissingField { field: context }),
        Some(v) => match v.as_str() {
            Some(s) if !s.is_empty() => Ok(s.to_string()),
            _ => Err(YamlError::FieldShape {
                field: context,
                expected: "non-empty string",
            }),
        },
    }
}

/// Optional string field.
fn opt_str(node: &saphyr::Yaml<'_>, field: &str) -> Option<String> {
    lookup(node, field).and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Optional u32 field.
fn opt_u32(node: &saphyr::Yaml<'_>, field: &str) -> Option<u32> {
    lookup(node, field).and_then(|v| v.as_integer().and_then(|i| u32::try_from(i).ok()))
}

/// Require a u16 field.
fn require_u16(node: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<u16> {
    match lookup(node, field) {
        None => Err(YamlError::MissingField { field }),
        Some(v) => v
            .as_integer()
            .and_then(|i| u16::try_from(i).ok())
            .ok_or(YamlError::FieldShape {
                field,
                expected: "u16 integer",
            }),
    }
}

// ---------------------------------------------------------------------------
// Trigger
// ---------------------------------------------------------------------------

fn parse_trigger(node: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    let Some(trigger_val) = lookup(node, "trigger") else {
        return Ok(TriggerAst::Manual);
    };

    if trigger_val.as_str() == Some("manual") {
        return Ok(TriggerAst::Manual);
    }

    if let Some(ipc_val) = lookup(trigger_val, "ipc") {
        let name = ipc_val
            .as_str()
            .ok_or(YamlError::FieldShape {
                field: "trigger.ipc",
                expected: "string",
            })?
            .to_string();
        return Ok(TriggerAst::Ipc { name });
    }

    Err(YamlError::FieldShape {
        field: "trigger",
        expected: "manual or ipc mapping",
    })
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

fn parse_inputs(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<InputField>> {
    let Some(seq) = lookup(node, "inputs").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut inputs = Vec::new();
    for item in seq.iter() {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "inputs",
                expected: "mapping",
            });
        }
        let name = require_str_in(item, "name", "inputs[].name")?;
        let field_type = opt_str(item, "type");
        let default = opt_str(item, "default");
        inputs.push(InputField {
            name,
            field_type,
            default,
        });
    }
    Ok(inputs)
}

// ---------------------------------------------------------------------------
// Vars
// ---------------------------------------------------------------------------

fn parse_vars(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<VarField>> {
    let Some(seq) = lookup(node, "vars").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut vars = Vec::new();
    for item in seq.iter() {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "vars",
                expected: "mapping",
            });
        }
        let name = require_str_in(item, "name", "vars[].name")?;
        let value = opt_str(item, "value");
        vars.push(VarField { name, value });
    }
    Ok(vars)
}

// ---------------------------------------------------------------------------
// Secrets
// ---------------------------------------------------------------------------

fn parse_secrets(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<SecretField>> {
    let Some(seq) = lookup(node, "secrets").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut secrets = Vec::new();
    for item in seq.iter() {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "secrets",
                expected: "mapping",
            });
        }
        let name = require_str_in(item, "name", "secrets[].name")?;
        let key = opt_str(item, "key");
        secrets.push(SecretField { name, key });
    }
    Ok(secrets)
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

fn parse_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(seq) = lookup(node, "steps").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut steps = Vec::new();
    for item in seq.iter() {
        steps.push(parse_step(item)?);
    }
    Ok(steps)
}

fn parse_step(yaml: &saphyr::Yaml<'_>) -> YamlResult<StepAst> {
    if !yaml.is_mapping() {
        return Err(YamlError::FieldShape {
            field: "step",
            expected: "mapping",
        });
    }

    let id = require_str_in(yaml, "id", "step.id")?;
    let name = opt_str(yaml, "name");
    let condition = opt_str(yaml, "if");
    let primitive = parse_step_primitive(yaml)?;
    let with = opt_str(yaml, "with");
    let retry = parse_retry(yaml)?;
    let on_error = parse_error_handler(yaml)?;
    let then = opt_str(yaml, "then");

    Ok(StepAst {
        id,
        name,
        condition,
        primitive,
        with,
        retry,
        on_error,
        then,
    })
}

fn parse_step_primitive(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    // Set
    if let Some(sub) = lookup(node, "set")
        && sub.is_mapping()
    {
        let output = require_str_in(sub, "output", "set.output")?;
        let value = require_str_in(sub, "value", "set.value")?;
        return Ok(StepPrimitive::Set { output, value });
    }

    // Do
    if let Some(sub) = lookup(node, "do")
        && sub.is_mapping()
    {
        let action = require_str_in(sub, "action", "do.action")?;
        let input = require_str_in(sub, "input", "do.input")?;
        return Ok(StepPrimitive::Do { action, input });
    }

    // Choose
    if let Some(sub) = lookup(node, "choose")
        && sub.is_mapping()
    {
        return parse_choose(sub);
    }

    // ForEach
    if let Some(sub) = lookup(node, "foreach")
        && sub.is_mapping()
    {
        return parse_foreach(sub);
    }

    // Together
    if let Some(sub) = lookup(node, "together")
        && sub.is_mapping()
    {
        return parse_together(sub);
    }

    // Collect
    if let Some(sub) = lookup(node, "collect")
        && sub.is_mapping()
    {
        return parse_collect(sub);
    }

    // Reduce
    if let Some(sub) = lookup(node, "reduce")
        && sub.is_mapping()
    {
        return parse_reduce(sub);
    }

    // Repeat
    if let Some(sub) = lookup(node, "repeat")
        && sub.is_mapping()
    {
        return parse_repeat(sub);
    }

    // Wait
    if let Some(sub) = lookup(node, "wait")
        && sub.is_mapping()
    {
        let event = opt_str(sub, "event");
        let timeout = opt_str(sub, "timeout");
        return Ok(StepPrimitive::Wait { event, timeout });
    }

    // Ask
    if let Some(sub) = lookup(node, "ask")
        && sub.is_mapping()
    {
        let prompt = require_str_in(sub, "prompt", "ask.prompt")?;
        let timeout = opt_str(sub, "timeout");
        return Ok(StepPrimitive::Ask { prompt, timeout });
    }

    // Finish
    if let Some(sub) = lookup(node, "finish")
        && sub.is_mapping()
    {
        let result = require_str_in(sub, "result", "finish.result")?;
        return Ok(StepPrimitive::Finish { result });
    }

    Err(YamlError::MissingField {
        field: "step primitive (set/do/choose/foreach/together/collect/reduce/repeat/wait/ask/finish)",
    })
}

fn parse_choose(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let mut branches = Vec::new();

    if let Some(seq) = lookup(node, "branches").and_then(|v| v.as_vec()) {
        for item in seq.iter() {
            if !item.is_mapping() {
                return Err(YamlError::FieldShape {
                    field: "choose.branches[]",
                    expected: "mapping",
                });
            }
            let when = require_str_in(item, "when", "choose.branches[].when")?;
            let steps = parse_body_steps(item)?;
            branches.push(ChooseBranch { when, steps });
        }
    }

    let otherwise = opt_str(node, "otherwise");

    Ok(StepPrimitive::Choose {
        branches,
        otherwise,
    })
}

fn parse_foreach(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let variable = require_str_in(node, "variable", "foreach.variable")?;
    let input = require_str_in(node, "input", "foreach.input")?;
    let at_once = opt_u32(node, "at_once");
    let body = parse_body_steps(node)?;

    Ok(StepPrimitive::ForEach {
        variable,
        input,
        at_once,
        body,
    })
}

fn parse_together(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let mut branches = Vec::new();

    if let Some(seq) = lookup(node, "branches").and_then(|v| v.as_vec()) {
        for item in seq.iter() {
            if !item.is_mapping() {
                return Err(YamlError::FieldShape {
                    field: "together.branches[]",
                    expected: "mapping",
                });
            }
            let label = require_str_in(item, "label", "together.branches[].label")?;
            let steps = parse_body_steps(item)?;
            branches.push(TogetherBranch { label, steps });
        }
    }

    Ok(StepPrimitive::Together { branches })
}

fn parse_collect(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let variable = require_str_in(node, "variable", "collect.variable")?;
    let source = require_str_in(node, "source", "collect.source")?;
    let pages = opt_u32(node, "pages");
    let items = opt_u32(node, "items");
    let body = parse_body_steps(node)?;

    Ok(StepPrimitive::Collect {
        variable,
        source,
        pages,
        items,
        body,
    })
}

fn parse_reduce(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let variable = require_str_in(node, "variable", "reduce.variable")?;
    let input = require_str_in(node, "input", "reduce.input")?;
    let initial = require_str_in(node, "initial", "reduce.initial")?;
    let body = parse_body_steps(node)?;

    Ok(StepPrimitive::Reduce {
        variable,
        input,
        initial,
        body,
    })
}

fn parse_repeat(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let max_attempts = require_u16(node, "max_attempts")?;
    let body = parse_body_steps(node)?;

    Ok(StepPrimitive::Repeat { max_attempts, body })
}

/// Parse the "steps" sub-sequence from a node.
fn parse_body_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(seq) = lookup(node, "steps").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut steps = Vec::new();
    for item in seq.iter() {
        steps.push(parse_step(item)?);
    }
    Ok(steps)
}

// ---------------------------------------------------------------------------
// Retry
// ---------------------------------------------------------------------------

fn parse_retry(node: &saphyr::Yaml<'_>) -> YamlResult<Option<RetryPolicy>> {
    let Some(sub) = lookup(node, "retry") else {
        return Ok(None);
    };
    if !sub.is_mapping() {
        return Ok(None);
    }

    let max_attempts = require_u16(sub, "max_attempts")?;
    let delay = opt_str(sub, "delay");

    Ok(Some(RetryPolicy {
        max_attempts,
        delay,
    }))
}

// ---------------------------------------------------------------------------
// Error handler
// ---------------------------------------------------------------------------

fn parse_error_handler(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ErrorHandlerAst>> {
    let Some(sub) = lookup(node, "on_error") else {
        return Ok(None);
    };
    if !sub.is_mapping() {
        return Ok(None);
    }

    let handler = require_str_in(sub, "handler", "on_error.handler")?;
    Ok(Some(ErrorHandlerAst { handler }))
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

fn parse_result(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ResultMapping>> {
    let Some(sub) = lookup(node, "result") else {
        return Ok(None);
    };
    if !sub.is_mapping() {
        return Ok(None);
    }

    let value = require_str_in(sub, "value", "result.value")?;
    Ok(Some(ResultMapping { value }))
}

// ---------------------------------------------------------------------------
// Examples
// ---------------------------------------------------------------------------

fn parse_examples(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<ExampleAst>> {
    let Some(seq) = lookup(node, "examples").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut examples = Vec::new();
    for item in seq.iter() {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "examples",
                expected: "mapping",
            });
        }
        let description = opt_str(item, "description");
        let input = opt_str(item, "input");
        let expected = opt_str(item, "expected");
        examples.push(ExampleAst {
            description,
            input,
            expected,
        });
    }
    Ok(examples)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_workflow() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: minimal
            trigger: manual
            steps:
              - id: s1
                set:
                  output: x
                  value: \"42\"
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        assert_eq!(wf.version, "velvet-ballastics/v1");
        assert_eq!(wf.name, "minimal");
        assert_eq!(wf.trigger, TriggerAst::Manual);
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(wf.steps[0].id, "s1");
        assert!(matches!(
            &wf.steps[0].primitive,
            StepPrimitive::Set { output, value } if output == "x" && value == "42"
        ));
    }

    #[test]
    fn parse_ipc_trigger() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ipc-test
            trigger:
              ipc: my-channel
            steps: []
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        assert_eq!(
            wf.trigger,
            TriggerAst::Ipc {
                name: "my-channel".to_string()
            }
        );
    }

    #[test]
    fn parse_inputs_vars_secrets() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: full
            trigger: manual
            inputs:
              - name: count
                type: u32
                default: \"10\"
            vars:
              - name: acc
                value: \"0\"
            secrets:
              - name: api_key
                key: vault/api_key
            steps: []
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        assert_eq!(wf.inputs.len(), 1);
        assert_eq!(wf.inputs[0].name, "count");
        assert_eq!(wf.inputs[0].field_type.as_deref(), Some("u32"));
        assert_eq!(wf.inputs[0].default.as_deref(), Some("10"));

        assert_eq!(wf.vars.len(), 1);
        assert_eq!(wf.vars[0].name, "acc");

        assert_eq!(wf.secrets.len(), 1);
        assert_eq!(wf.secrets[0].name, "api_key");
        assert_eq!(wf.secrets[0].key.as_deref(), Some("vault/api_key"));
    }

    #[test]
    fn parse_do_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: do-test
            trigger: manual
            steps:
              - id: do1
                do:
                  action: http.get
                  input: '\"https://example.com\"'
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        assert!(matches!(
            &wf.steps[0].primitive,
            StepPrimitive::Do { action, input }
            if action == "http.get" && input == "\"https://example.com\""
        ));
    }

    #[test]
    fn parse_choose_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: choose-test
            trigger: manual
            steps:
              - id: c1
                choose:
                  branches:
                    - when: x > 0
                      steps:
                        - id: pos
                          set:
                            output: sign
                            value: \"1\"
                  otherwise: handle_zero
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::Choose {
                branches,
                otherwise,
            } => {
                assert_eq!(branches.len(), 1);
                assert_eq!(branches[0].when, "x > 0");
                assert_eq!(branches[0].steps.len(), 1);
                assert_eq!(otherwise.as_deref(), Some("handle_zero"));
            }
            other => panic!("expected Choose, got {other:?}"),
        }
    }

    #[test]
    fn parse_foreach_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: foreach-test
            trigger: manual
            steps:
              - id: fe1
                foreach:
                  variable: item
                  input: items
                  at_once: 5
                  steps:
                    - id: inner
                      set:
                        output: out
                        value: item
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::ForEach {
                variable,
                input,
                at_once,
                body,
            } => {
                assert_eq!(variable, "item");
                assert_eq!(input, "items");
                assert_eq!(*at_once, Some(5));
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected ForEach, got {other:?}"),
        }
    }

    #[test]
    fn parse_together_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: together-test
            trigger: manual
            steps:
              - id: t1
                together:
                  branches:
                    - label: a
                      steps:
                        - id: sa
                          set:
                            output: x
                            value: \"1\"
                    - label: b
                      steps: []
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::Together { branches } => {
                assert_eq!(branches.len(), 2);
                assert_eq!(branches[0].label, "a");
                assert_eq!(branches[0].steps.len(), 1);
                assert_eq!(branches[1].label, "b");
            }
            other => panic!("expected Together, got {other:?}"),
        }
    }

    #[test]
    fn parse_collect_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: collect-test
            trigger: manual
            steps:
              - id: col1
                collect:
                  variable: page
                  source: api.list
                  pages: 10
                  items: 50
                  steps:
                    - id: process
                      set:
                        output: buf
                        value: page
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::Collect {
                variable,
                source,
                pages,
                items,
                body,
            } => {
                assert_eq!(variable, "page");
                assert_eq!(source, "api.list");
                assert_eq!(*pages, Some(10));
                assert_eq!(*items, Some(50));
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected Collect, got {other:?}"),
        }
    }

    #[test]
    fn parse_reduce_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: reduce-test
            trigger: manual
            steps:
              - id: r1
                reduce:
                  variable: acc
                  input: items
                  initial: \"0\"
                  steps: []
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::Reduce {
                variable,
                input,
                initial,
                body,
            } => {
                assert_eq!(variable, "acc");
                assert_eq!(input, "items");
                assert_eq!(initial, "0");
                assert!(body.is_empty());
            }
            other => panic!("expected Reduce, got {other:?}"),
        }
    }

    #[test]
    fn parse_repeat_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: repeat-test
            trigger: manual
            steps:
              - id: rp1
                repeat:
                  max_attempts: 3
                  steps:
                    - id: attempt
                      do:
                        action: http.post
                        input: body
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::Repeat {
                max_attempts,
                body,
            } => {
                assert_eq!(*max_attempts, 3);
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected Repeat, got {other:?}"),
        }
    }

    #[test]
    fn parse_wait_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: wait-test
            trigger: manual
            steps:
              - id: w1
                wait:
                  event: approval
                  timeout: 30s
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::Wait { event, timeout } => {
                assert_eq!(event.as_deref(), Some("approval"));
                assert_eq!(timeout.as_deref(), Some("30s"));
            }
            other => panic!("expected Wait, got {other:?}"),
        }
    }

    #[test]
    fn parse_ask_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ask-test
            trigger: manual
            steps:
              - id: a1
                ask:
                  prompt: Continue?
                  timeout: 60s
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::Ask { prompt, timeout } => {
                assert_eq!(prompt, "Continue?");
                assert_eq!(timeout.as_deref(), Some("60s"));
            }
            other => panic!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn parse_finish_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: finish-test
            trigger: manual
            steps:
              - id: f1
                finish:
                  result: output
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        match &wf.steps[0].primitive {
            StepPrimitive::Finish { result } => {
                assert_eq!(result, "output");
            }
            other => panic!("expected Finish, got {other:?}"),
        }
    }

    #[test]
    fn parse_step_with_metadata() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: meta-test
            trigger: manual
            steps:
              - id: s1
                name: My Step
                if: x > 0
                with: http_connector
                retry:
                  max_attempts: 3
                  delay: 1s
                on_error:
                  handler: fallback
                then: next_step
                set:
                  output: y
                  value: \"hello\"
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        let step = &wf.steps[0];
        assert_eq!(step.name.as_deref(), Some("My Step"));
        assert_eq!(step.condition.as_deref(), Some("x > 0"));
        assert_eq!(step.with.as_deref(), Some("http_connector"));
        assert_eq!(step.then.as_deref(), Some("next_step"));

        let retry = step.retry.as_ref().unwrap();
        assert_eq!(retry.max_attempts, 3);
        assert_eq!(retry.delay.as_deref(), Some("1s"));

        let on_error = step.on_error.as_ref().unwrap();
        assert_eq!(on_error.handler, "fallback");
    }

    #[test]
    fn parse_result_and_examples() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: result-test
            trigger: manual
            steps: []
            result:
              value: final_output
            examples:
              - description: basic test
                input: '{\"x\": 1}'
                expected: \"2\"
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        let result = wf.result.as_ref().unwrap();
        assert_eq!(result.value, "final_output");

        assert_eq!(wf.examples.len(), 1);
        assert_eq!(wf.examples[0].description.as_deref(), Some("basic test"));
        assert_eq!(wf.examples[0].input.as_deref(), Some("{\"x\": 1}"));
        assert_eq!(wf.examples[0].expected.as_deref(), Some("2"));
    }

    #[test]
    fn missing_version_is_error() {
        let yaml = "name: test\ntrigger: manual\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn missing_step_primitive_is_error() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: test
            trigger: manual
            steps:
              - id: s1
        "};
        let result = parse_workflow_ast(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn empty_source_is_error() {
        let result = parse_workflow_ast("");
        assert!(result.is_err());
    }
}
