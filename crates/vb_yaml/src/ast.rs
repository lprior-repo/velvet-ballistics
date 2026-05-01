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
        Some(v) => {
            v.as_integer()
                .and_then(|i| u16::try_from(i).ok())
                .ok_or(YamlError::FieldShape {
                    field,
                    expected: "u16 integer",
                })
        }
    }
}

// ---------------------------------------------------------------------------
// Trigger
// ---------------------------------------------------------------------------

fn parse_trigger(node: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    if let Some(when_val) = lookup(node, "when") {
        return parse_when_trigger(when_val);
    }
    Err(YamlError::MissingField { field: "when" })
}

fn parse_when_trigger(when_val: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    if let Some(manual_val) = lookup(when_val, "manual") {
        if manual_val.is_mapping() {
            return Ok(TriggerAst::Manual);
        }
        return Err(YamlError::FieldShape {
            field: "when.manual",
            expected: "mapping",
        });
    }

    if let Some(ipc_val) = lookup(when_val, "ipc") {
        let name = require_str_in(ipc_val, "name", "when.ipc.name")?;
        return Ok(TriggerAst::Ipc { name });
    }

    if lookup(when_val, "http").is_some() {
        return Err(YamlError::UnsupportedFeature {
            feature: "http trigger",
        });
    }

    Err(YamlError::FieldShape {
        field: "when",
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

    fn assertion_failed(message: std::fmt::Arguments<'_>) -> bool {
        let _ = message;
        false
    }

    macro_rules! fail_assert {
        ($($arg:tt)*) => {
            assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
        };
    }

    macro_rules! parse_ok {
        ($yaml:expr) => {
            match parse_workflow_ast($yaml) {
                Ok(value) => value,
                Err(error) => {
                    fail_assert!("parse failed: {error}");
                    return;
                }
            }
        };
    }

    macro_rules! first_item {
        ($values:expr, $label:expr) => {
            match $values.first() {
                Some(value) => value,
                None => {
                    fail_assert!("missing {}", $label);
                    return;
                }
            }
        };
    }

    #[test]
    fn parse_minimal_workflow() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: minimal
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"42\"
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.version, "velvet-ballastics/v1");
        assert_eq!(wf.name, "minimal");
        assert_eq!(wf.trigger, TriggerAst::Manual);
        assert_eq!(wf.steps.len(), 1);
        let first_step = first_item!(wf.steps, "step");
        assert_eq!(first_step.id, "s1");
        assert!(matches!(
            &first_step.primitive,
            StepPrimitive::Set { output, value } if output == "x" && value == "42"
        ));
    }

    #[test]
    fn parse_ipc_trigger() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ipc-test
            when:
              ipc:
                name: my-channel
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(
            wf.trigger,
            TriggerAst::Ipc {
                name: "my-channel".to_string()
            }
        );
    }

    #[test]
    fn parse_canonical_when_manual_trigger() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: manual-test
            when:
              manual: {}
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Ok(WorkflowSource {
                trigger: TriggerAst::Manual,
                ..
            })
        ));
    }

    #[test]
    fn parse_canonical_when_ipc_trigger() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ipc-test
            when:
              ipc:
                name: issue_triage
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Ok(WorkflowSource {
                trigger: TriggerAst::Ipc { name },
                ..
            }) if name == "issue_triage"
        ));
    }

    #[test]
    fn canonical_when_http_trigger_is_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: http-test
            when:
              http: {}
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert!(
            matches!(result, Err(YamlError::UnsupportedFeature { feature }) if feature == "http trigger")
        );
    }

    #[test]
    fn parse_inputs_vars_secrets() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: full
            when:
              manual: {}
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
        let wf = parse_ok!(yaml);
        assert_eq!(wf.inputs.len(), 1);
        let first_input = first_item!(wf.inputs, "input");
        assert_eq!(first_input.name, "count");
        assert_eq!(first_input.field_type.as_deref(), Some("u32"));
        assert_eq!(first_input.default.as_deref(), Some("10"));

        assert_eq!(wf.vars.len(), 1);
        let first_var = first_item!(wf.vars, "var");
        assert_eq!(first_var.name, "acc");

        assert_eq!(wf.secrets.len(), 1);
        let first_secret = first_item!(wf.secrets, "secret");
        assert_eq!(first_secret.name, "api_key");
        assert_eq!(first_secret.key.as_deref(), Some("vault/api_key"));
    }

    #[test]
    fn parse_do_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: do-test
            when:
              manual: {}
            steps:
              - id: do1
                do:
                  action: http.get
                  input: '\"https://example.com\"'
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        assert!(matches!(
            &first_step.primitive,
            StepPrimitive::Do { action, input }
            if action == "http.get" && input == "\"https://example.com\""
        ));
    }

    #[test]
    fn parse_choose_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: choose-test
            when:
              manual: {}
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
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Choose {
                branches,
                otherwise,
            } => {
                assert_eq!(branches.len(), 1);
                let first_branch = first_item!(branches, "branch");
                assert_eq!(first_branch.when, "x > 0");
                assert_eq!(first_branch.steps.len(), 1);
                assert_eq!(otherwise.as_deref(), Some("handle_zero"));
            }
            other => fail_assert!("expected Choose, got {other:?}"),
        }
    }

    #[test]
    fn parse_foreach_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: foreach-test
            when:
              manual: {}
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
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
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
            other => fail_assert!("expected ForEach, got {other:?}"),
        }
    }

    #[test]
    fn parse_together_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: together-test
            when:
              manual: {}
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
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Together { branches } => {
                assert_eq!(branches.len(), 2);
                let first_branch = first_item!(branches, "branch");
                assert_eq!(first_branch.label, "a");
                assert_eq!(first_branch.steps.len(), 1);
                let Some(second_branch) = branches.get(1) else {
                    fail_assert!("missing second branch");
                    return;
                };
                assert_eq!(second_branch.label, "b");
            }
            other => fail_assert!("expected Together, got {other:?}"),
        }
    }

    #[test]
    fn parse_collect_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: collect-test
            when:
              manual: {}
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
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
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
            other => fail_assert!("expected Collect, got {other:?}"),
        }
    }

    #[test]
    fn parse_reduce_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: reduce-test
            when:
              manual: {}
            steps:
              - id: r1
                reduce:
                  variable: acc
                  input: items
                  initial: \"0\"
                  steps: []
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
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
            other => fail_assert!("expected Reduce, got {other:?}"),
        }
    }

    #[test]
    fn parse_repeat_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: repeat-test
            when:
              manual: {}
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
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Repeat { max_attempts, body } => {
                assert_eq!(*max_attempts, 3);
                assert_eq!(body.len(), 1);
            }
            other => fail_assert!("expected Repeat, got {other:?}"),
        }
    }

    #[test]
    fn parse_wait_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: wait-test
            when:
              manual: {}
            steps:
              - id: w1
                wait:
                  event: approval
                  timeout: 30s
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Wait { event, timeout } => {
                assert_eq!(event.as_deref(), Some("approval"));
                assert_eq!(timeout.as_deref(), Some("30s"));
            }
            other => fail_assert!("expected Wait, got {other:?}"),
        }
    }

    #[test]
    fn parse_ask_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ask-test
            when:
              manual: {}
            steps:
              - id: a1
                ask:
                  prompt: Continue?
                  timeout: 60s
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Ask { prompt, timeout } => {
                assert_eq!(prompt, "Continue?");
                assert_eq!(timeout.as_deref(), Some("60s"));
            }
            other => fail_assert!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn parse_finish_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: finish-test
            when:
              manual: {}
            steps:
              - id: f1
                finish:
                  result: output
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Finish { result } => {
                assert_eq!(result, "output");
            }
            other => fail_assert!("expected Finish, got {other:?}"),
        }
    }

    #[test]
    fn parse_step_with_metadata() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: meta-test
            when:
              manual: {}
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
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        assert_eq!(step.name.as_deref(), Some("My Step"));
        assert_eq!(step.condition.as_deref(), Some("x > 0"));
        assert_eq!(step.with.as_deref(), Some("http_connector"));
        assert_eq!(step.then.as_deref(), Some("next_step"));

        let Some(retry) = step.retry.as_ref() else {
            fail_assert!("missing retry");
            return;
        };
        assert_eq!(retry.max_attempts, 3);
        assert_eq!(retry.delay.as_deref(), Some("1s"));

        let Some(on_error) = step.on_error.as_ref() else {
            fail_assert!("missing on_error");
            return;
        };
        assert_eq!(on_error.handler, "fallback");
    }

    #[test]
    fn parse_result_and_examples() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: result-test
            when:
              manual: {}
            steps: []
            result:
              value: final_output
            examples:
              - description: basic test
                input: '{\"x\": 1}'
                expected: \"2\"
        "};
        let wf = parse_ok!(yaml);
        let Some(result) = wf.result.as_ref() else {
            fail_assert!("missing result");
            return;
        };
        assert_eq!(result.value, "final_output");

        assert_eq!(wf.examples.len(), 1);
        let first_example = first_item!(wf.examples, "example");
        assert_eq!(first_example.description.as_deref(), Some("basic test"));
        assert_eq!(first_example.input.as_deref(), Some("{\"x\": 1}"));
        assert_eq!(first_example.expected.as_deref(), Some("2"));
    }

    #[test]
    fn missing_version_is_error() {
        let yaml = "name: test\nwhen:\n  manual: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn missing_step_primitive_is_error() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: test
            when:
              manual: {}
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

    // -----------------------------------------------------------------------
    // AST BDD tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_workflow_ast_produces_typed_nodes_for_valid_input() {
        // Given: valid workflow YAML
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: typed
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
        "};
        // When: parsing the AST
        let result = parse_workflow_ast(yaml);
        // Then: Ok with correct structure
        match result {
            Ok(wf) => {
                assert_eq!(wf.version, "velvet-ballastics/v1");
                assert_eq!(wf.name, "typed");
                assert_eq!(wf.steps.len(), 1);
            }
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn parse_workflow_ast_returns_scalar_kind_for_scalar_nodes() {
        // Given: workflow with a set step producing a string value
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: scalar-test
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"hello\"
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: the Set primitive has the exact value
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Set { output, value } => {
                assert_eq!(output, "x");
                assert_eq!(value, "hello");
            }
            other => fail_assert!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn parse_workflow_ast_returns_mapping_for_mapping_nodes() {
        // Given: workflow with nested mapping (retry, on_error)
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: mapping-test
            when:
              manual: {}
            steps:
              - id: s1
                retry:
                  max_attempts: 5
                set:
                  output: x
                  value: \"1\"
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: retry is parsed as a mapping with correct fields
        let first_step = first_item!(wf.steps, "step");
        let Some(retry) = first_step.retry.as_ref() else {
            fail_assert!("missing retry");
            return;
        };
        assert_eq!(retry.max_attempts, 5);
        assert_eq!(retry.delay, None);
    }

    #[test]
    fn parse_workflow_ast_returns_sequence_for_sequence_nodes() {
        // Given: workflow with a sequence of steps
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: seq-test
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
              - id: s2
                set:
                  output: y
                  value: \"2\"
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: steps is a sequence with correct length and IDs
        assert_eq!(wf.steps.len(), 2);
        assert_eq!(wf.steps.first().map(|step| step.id.as_str()), Some("s1"));
        assert_eq!(wf.steps.get(1).map(|step| step.id.as_str()), Some("s2"));
    }

    #[test]
    fn parse_preserves_span_information_in_nodes() {
        // Given: a source map built from valid workflow YAML
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: span-test
            when:
              manual: {}
            steps: []
        "};
        // When: building source map
        let result = crate::source_map::build_source_map(yaml);
        // Then: Ok with non-empty map containing valid spans
        match result {
            Ok(map) => {
                assert!(!map.is_empty());
                let first_span = map.span_for_node(0);
                let Some(span) = first_span else {
                    fail_assert!("expected Some span for node 0");
                    return;
                };
                assert!(span.start_line > 0);
            }
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn missing_version_returns_missing_field_exact() {
        // Given: YAML without version
        let yaml = "name: test\nwhen:\n  manual: {}\nsteps: []\n";
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "version" })
        assert_eq!(result, Err(YamlError::MissingField { field: "version" }));
    }

    #[test]
    fn missing_name_returns_missing_field_exact() {
        // Given: YAML without name
        let yaml = "version: velvet-ballastics/v1\nwhen:\n  manual: {}\nsteps: []\n";
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "name" })
        assert_eq!(result, Err(YamlError::MissingField { field: "name" }));
    }

    #[test]
    fn missing_when_returns_missing_field_exact() {
        // Given: YAML without when
        let yaml = "version: velvet-ballastics/v1\nname: test\nsteps: []\n";
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "when" })
        assert_eq!(result, Err(YamlError::MissingField { field: "when" }));
    }

    #[test]
    fn missing_step_primitive_returns_error_exact() {
        // Given: step without a primitive
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: test
            when:
              manual: {}
            steps:
              - id: s1
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField) for step primitive
        match result {
            Err(YamlError::MissingField { field }) => {
                assert!(
                    field.contains("step primitive"),
                    "expected step primitive field, got: {field}"
                );
            }
            other => fail_assert!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn empty_version_returns_field_shape_error() {
        // Given: YAML with empty version string
        let yaml = "version: ''\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape { field: "version", expected: "non-empty string" })
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "version",
                expected: "non-empty string"
            })
        );
    }

    #[test]
    fn empty_name_returns_field_shape_error() {
        // Given: YAML with empty name string
        let yaml = "version: velvet-ballastics/v1\nname: ''\nwhen:\n  manual: {}\nsteps: []\n";
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape { field: "name", expected: "non-empty string" })
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "name",
                expected: "non-empty string"
            })
        );
    }

    #[test]
    fn non_mapping_root_returns_field_shape_error() {
        // Given: YAML with scalar root
        let yaml = "just a string\n";
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape { field: "workflow", expected: "mapping" })
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "workflow",
                expected: "mapping"
            })
        );
    }

    #[test]
    fn http_trigger_returns_unsupported_feature_exact() {
        // Given: YAML with http trigger
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: t
            when:
              http: {}
            steps: []
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::UnsupportedFeature { feature: "http trigger" })
        assert_eq!(
            result,
            Err(YamlError::UnsupportedFeature {
                feature: "http trigger"
            })
        );
    }

    #[test]
    fn parse_wait_step_with_only_timeout() {
        // Given: wait step with only timeout
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: wait-only
            when:
              manual: {}
            steps:
              - id: w1
                wait:
                  timeout: 10s
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        // Then: Wait with event=None, timeout=Some("10s")
        match &first_step.primitive {
            StepPrimitive::Wait { event, timeout } => {
                assert_eq!(*event, None);
                assert_eq!(timeout.as_deref(), Some("10s"));
            }
            other => fail_assert!("expected Wait, got {other:?}"),
        }
    }

    #[test]
    fn parse_ask_step_without_timeout() {
        // Given: ask step without timeout
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ask-simple
            when:
              manual: {}
            steps:
              - id: a1
                ask:
                  prompt: What?
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        // Then: Ask with prompt="What?", timeout=None
        match &first_step.primitive {
            StepPrimitive::Ask { prompt, timeout } => {
                assert_eq!(prompt, "What?");
                assert_eq!(*timeout, None);
            }
            other => fail_assert!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn parse_step_with_condition() {
        // Given: step with an "if" condition
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: cond
            when:
              manual: {}
            steps:
              - id: s1
                if: x > 10
                set:
                  output: y
                  value: \"1\"
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        // Then: condition is Some("x > 10")
        assert_eq!(step.condition.as_deref(), Some("x > 10"));
    }

    #[test]
    fn parse_step_with_then() {
        // Given: step with a "then" field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: then-test
            when:
              manual: {}
            steps:
              - id: s1
                then: next_step
                set:
                  output: y
                  value: \"1\"
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        // Then: then is Some("next_step")
        assert_eq!(step.then.as_deref(), Some("next_step"));
    }

    #[test]
    fn parse_workflow_with_inputs_and_defaults() {
        // Given: workflow with input having type and default
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: inputs-test
            when:
              manual: {}
            inputs:
              - name: count
                type: u32
                default: \"10\"
              - name: name
                type: string
            steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: two inputs with correct fields
        assert_eq!(wf.inputs.len(), 2);
        assert_eq!(
            wf.inputs.first().map(|input| input.name.as_str()),
            Some("count")
        );
        assert_eq!(
            wf.inputs
                .first()
                .and_then(|input| input.field_type.as_deref()),
            Some("u32")
        );
        assert_eq!(
            wf.inputs.first().and_then(|input| input.default.as_deref()),
            Some("10")
        );
        assert_eq!(
            wf.inputs.get(1).map(|input| input.name.as_str()),
            Some("name")
        );
        assert_eq!(
            wf.inputs
                .get(1)
                .and_then(|input| input.field_type.as_deref()),
            Some("string")
        );
        assert_eq!(
            wf.inputs.get(1).and_then(|input| input.default.as_ref()),
            None
        );
    }

    #[test]
    fn parse_workflow_with_vars() {
        // Given: workflow with vars
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: vars-test
            when:
              manual: {}
            vars:
              - name: acc
                value: \"0\"
              - name: buf
            steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: two vars with correct fields
        assert_eq!(wf.vars.len(), 2);
        assert_eq!(wf.vars.first().map(|var| var.name.as_str()), Some("acc"));
        assert_eq!(
            wf.vars.first().and_then(|var| var.value.as_deref()),
            Some("0")
        );
        assert_eq!(wf.vars.get(1).map(|var| var.name.as_str()), Some("buf"));
        assert_eq!(wf.vars.get(1).and_then(|var| var.value.as_ref()), None);
    }

    #[test]
    fn parse_workflow_with_secrets() {
        // Given: workflow with secrets
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: secrets-test
            when:
              manual: {}
            secrets:
              - name: api_key
                key: vault/api_key
              - name: db_pass
            steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: two secrets with correct fields
        assert_eq!(wf.secrets.len(), 2);
        assert_eq!(
            wf.secrets.first().map(|secret| secret.name.as_str()),
            Some("api_key")
        );
        assert_eq!(
            wf.secrets.first().and_then(|secret| secret.key.as_deref()),
            Some("vault/api_key")
        );
        assert_eq!(
            wf.secrets.get(1).map(|secret| secret.name.as_str()),
            Some("db_pass")
        );
        assert_eq!(
            wf.secrets.get(1).and_then(|secret| secret.key.as_ref()),
            None
        );
    }

    #[test]
    fn parse_workflow_with_result() {
        // Given: workflow with result mapping
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: result-test
            when:
              manual: {}
            steps: []
            result:
              value: final_output
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: result is Some with exact value
        let Some(ref result) = wf.result else {
            fail_assert!("missing result");
            return;
        };
        assert_eq!(result.value, "final_output");
    }

    #[test]
    fn parse_workflow_without_result() {
        // Given: workflow without result mapping
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-result
            when:
              manual: {}
            steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: result is None
        assert_eq!(wf.result, None);
    }

    #[test]
    fn parse_workflow_with_examples() {
        // Given: workflow with examples
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ex-test
            when:
              manual: {}
            steps: []
            examples:
              - description: basic
                input: '{\"x\": 1}'
                expected: \"2\"
              - description: empty
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: two examples with correct fields
        assert_eq!(wf.examples.len(), 2);
        assert_eq!(
            wf.examples
                .first()
                .and_then(|example| example.description.as_deref()),
            Some("basic")
        );
        assert_eq!(
            wf.examples
                .first()
                .and_then(|example| example.input.as_deref()),
            Some("{\"x\": 1}")
        );
        assert_eq!(
            wf.examples
                .first()
                .and_then(|example| example.expected.as_deref()),
            Some("2")
        );
        assert_eq!(
            wf.examples
                .get(1)
                .and_then(|example| example.description.as_deref()),
            Some("empty")
        );
        assert_eq!(
            wf.examples
                .get(1)
                .and_then(|example| example.input.as_ref()),
            None
        );
        assert_eq!(
            wf.examples
                .get(1)
                .and_then(|example| example.expected.as_ref()),
            None
        );
    }

    #[test]
    fn parse_workflow_without_examples() {
        // Given: workflow without examples
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-ex
            when:
              manual: {}
            steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: examples is empty
        assert!(wf.examples.is_empty());
    }

    #[test]
    fn parse_foreach_without_at_once() {
        // Given: foreach step without at_once
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: foreach-simple
            when:
              manual: {}
            steps:
              - id: fe1
                foreach:
                  variable: item
                  input: items
                  steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::ForEach {
                variable,
                input,
                at_once,
                body,
            } => {
                assert_eq!(variable, "item");
                assert_eq!(input, "items");
                assert_eq!(*at_once, None);
                assert!(body.is_empty());
            }
            other => fail_assert!("expected ForEach, got {other:?}"),
        }
    }

    #[test]
    fn parse_collect_without_optional_fields() {
        // Given: collect step without pages/items
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: collect-simple
            when:
              manual: {}
            steps:
              - id: c1
                collect:
                  variable: page
                  source: api.list
                  steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Collect {
                variable,
                source,
                pages,
                items,
                body,
            } => {
                assert_eq!(variable, "page");
                assert_eq!(source, "api.list");
                assert_eq!(*pages, None);
                assert_eq!(*items, None);
                assert!(body.is_empty());
            }
            other => fail_assert!("expected Collect, got {other:?}"),
        }
    }

    #[test]
    fn parse_repeat_with_max_attempts() {
        // Given: repeat step with max_attempts
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: repeat-simple
            when:
              manual: {}
            steps:
              - id: r1
                repeat:
                  max_attempts: 5
                  steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Repeat { max_attempts, body } => {
                assert_eq!(*max_attempts, 5);
                assert!(body.is_empty());
            }
            other => fail_assert!("expected Repeat, got {other:?}"),
        }
    }

    #[test]
    fn parse_do_step_with_input() {
        // Given: do step with action and input
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: do-test
            when:
              manual: {}
            steps:
              - id: d1
                do:
                  action: http.post
                  input: payload
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Do { action, input } => {
                assert_eq!(action, "http.post");
                assert_eq!(input, "payload");
            }
            other => fail_assert!("expected Do, got {other:?}"),
        }
    }

    #[test]
    fn parse_finish_step_with_result() {
        // Given: finish step with result expression
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: finish-simple
            when:
              manual: {}
            steps:
              - id: f1
                finish:
                  result: done
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Finish { result } => {
                assert_eq!(result, "done");
            }
            other => fail_assert!("expected Finish, got {other:?}"),
        }
    }

    #[test]
    fn parse_choose_with_multiple_branches() {
        // Given: choose with two branches
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: choose-multi
            when:
              manual: {}
            steps:
              - id: c1
                choose:
                  branches:
                    - when: x > 0
                      steps: []
                    - when: x < 0
                      steps: []
                  otherwise: zero
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Choose {
                branches,
                otherwise,
            } => {
                assert_eq!(branches.len(), 2);
                assert_eq!(
                    branches.first().map(|branch| branch.when.as_str()),
                    Some("x > 0")
                );
                assert_eq!(
                    branches.get(1).map(|branch| branch.when.as_str()),
                    Some("x < 0")
                );
                assert_eq!(otherwise.as_deref(), Some("zero"));
            }
            other => fail_assert!("expected Choose, got {other:?}"),
        }
    }

    #[test]
    fn parse_together_with_multiple_branches() {
        // Given: together with two branches
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: together-multi
            when:
              manual: {}
            steps:
              - id: t1
                together:
                  branches:
                    - label: first
                      steps: []
                    - label: second
                      steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Together { branches } => {
                assert_eq!(branches.len(), 2);
                assert_eq!(
                    branches.first().map(|branch| branch.label.as_str()),
                    Some("first")
                );
                assert_eq!(
                    branches.get(1).map(|branch| branch.label.as_str()),
                    Some("second")
                );
            }
            other => fail_assert!("expected Together, got {other:?}"),
        }
    }

    #[test]
    fn parse_step_with_on_error_handler() {
        // Given: step with on_error handler
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: error-handler
            when:
              manual: {}
            steps:
              - id: s1
                on_error:
                  handler: fallback
                set:
                  output: x
                  value: \"1\"
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        let Some(ref on_error) = step.on_error else {
            fail_assert!("missing on_error");
            return;
        };
        assert_eq!(on_error.handler, "fallback");
    }

    #[test]
    fn parse_step_without_optional_fields() {
        // Given: minimal step with no optional fields
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: minimal-step
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        // Then: all optional fields are None
        assert_eq!(step.name, None);
        assert_eq!(step.condition, None);
        assert_eq!(step.with, None);
        assert_eq!(step.retry, None);
        assert_eq!(step.on_error, None);
        assert_eq!(step.then, None);
    }

    #[test]
    fn parse_ipc_trigger_exact_fields() {
        // Given: IPC trigger with specific channel name
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ipc-exact
            when:
              ipc:
                name: my-channel
            steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: TriggerAst::Ipc with exact name
        assert_eq!(
            wf.trigger,
            TriggerAst::Ipc {
                name: "my-channel".to_string()
            }
        );
    }

    #[test]
    fn parse_empty_steps_list() {
        // Given: workflow with empty steps
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: empty-steps
            when:
              manual: {}
            steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: steps is empty vec
        assert!(wf.steps.is_empty());
    }

    // -----------------------------------------------------------------------
    // Adversarial BDD tests - AST layer attack vectors
    // -----------------------------------------------------------------------

    #[test]
    fn adversarial_ast_non_mapping_step_rejected() {
        // Given: workflow with a step that is a scalar, not a mapping
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-step
            when:
              manual: {}
            steps:
              - just_a_string
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape)
        assert!(
            matches!(result, Err(YamlError::FieldShape { field, .. }) if field == "step"),
            "expected FieldShape(step), got: {result:?}"
        );
    }

    #[test]
    fn adversarial_ast_step_missing_id_rejected() {
        // Given: workflow with a step that has no id field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-id
            when:
              manual: {}
            steps:
              - set:
                  output: x
                  value: \"1\"
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "step.id" })
        assert_eq!(result, Err(YamlError::MissingField { field: "step.id" }));
    }

    #[test]
    fn adversarial_ast_empty_step_id_rejected() {
        // Given: workflow with a step whose id is empty string
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: empty-id
            when:
              manual: {}
            steps:
              - id: ''
                set:
                  output: x
                  value: \"1\"
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape) - empty id is not a non-empty string
        assert!(
            matches!(result, Err(YamlError::FieldShape { field, .. }) if field == "step.id"),
            "expected FieldShape for empty id, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_ast_set_missing_output_rejected() {
        // Given: set step missing output field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-output
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  value: \"1\"
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "set.output" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "set.output"
            })
        );
    }

    #[test]
    fn adversarial_ast_do_missing_action_rejected() {
        // Given: do step missing action field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-action
            when:
              manual: {}
            steps:
              - id: s1
                do:
                  input: payload
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "do.action" })
        assert_eq!(result, Err(YamlError::MissingField { field: "do.action" }));
    }

    #[test]
    fn adversarial_ast_ask_missing_prompt_rejected() {
        // Given: ask step missing prompt field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-prompt
            when:
              manual: {}
            steps:
              - id: s1
                ask:
                  timeout: 10s
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "ask.prompt" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "ask.prompt"
            })
        );
    }

    #[test]
    fn adversarial_ast_repeat_missing_max_attempts_rejected() {
        // Given: repeat step missing max_attempts
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-max
            when:
              manual: {}
            steps:
              - id: s1
                repeat:
                  steps: []
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "max_attempts" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "max_attempts"
            })
        );
    }

    #[test]
    fn adversarial_ast_reduce_missing_initial_rejected() {
        // Given: reduce step missing initial field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-init
            when:
              manual: {}
            steps:
              - id: s1
                reduce:
                  variable: acc
                  input: items
                  steps: []
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "reduce.initial" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "reduce.initial"
            })
        );
    }

    #[test]
    fn adversarial_ast_collect_missing_source_rejected() {
        // Given: collect step missing source field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-source
            when:
              manual: {}
            steps:
              - id: s1
                collect:
                  variable: page
                  steps: []
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "collect.source" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "collect.source"
            })
        );
    }

    #[test]
    fn adversarial_ast_finish_missing_result_rejected() {
        // Given: finish step missing result field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-result
            when:
              manual: {}
            steps:
              - id: s1
                finish: {}
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "finish.result" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "finish.result"
            })
        );
    }

    #[test]
    fn adversarial_ast_invalid_input_type_rejected() {
        // Given: inputs field is a string, not a sequence
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-inputs
            when:
              manual: {}
            inputs: not_a_list
            steps: []
        "};
        // When: parsing
        let wf = parse_ok!(yaml);
        // Then: inputs is treated as empty (opt-in parsing returns empty)
        assert!(
            wf.inputs.is_empty(),
            "non-sequence inputs should be treated as empty"
        );
    }

    #[test]
    fn adversarial_ast_non_mapping_input_item_rejected() {
        // Given: inputs with a scalar item instead of mapping
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-input-item
            when:
              manual: {}
            inputs:
              - just_a_string
            steps: []
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape { field: "inputs", expected: "mapping" })
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "inputs",
                expected: "mapping"
            })
        );
    }

    #[test]
    fn adversarial_ast_http_trigger_rejected_by_ast_layer() {
        // Given: YAML with http trigger
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: http-trigger
            when:
              http: {}
            steps: []
        "};
        // When: parsing AST directly (bypassing profile)
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::UnsupportedFeature { feature: "http trigger" })
        assert_eq!(
            result,
            Err(YamlError::UnsupportedFeature {
                feature: "http trigger"
            })
        );
    }

    #[test]
    fn adversarial_ast_scalar_root_rejected() {
        // Given: YAML whose root is a plain scalar
        let yaml = "42\n";
        // When: parsing AST
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape { field: "workflow", expected: "mapping" })
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "workflow",
                expected: "mapping"
            })
        );
    }

    #[test]
    fn adversarial_ast_sequence_root_rejected() {
        // Given: YAML whose root is a sequence
        let yaml = "- a\n- b\n";
        // When: parsing AST
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape { field: "workflow", expected: "mapping" })
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "workflow",
                expected: "mapping"
            })
        );
    }

    #[test]
    fn adversarial_ast_together_branch_missing_label_rejected() {
        // Given: together branch without a label
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-label
            when:
              manual: {}
            steps:
              - id: t1
                together:
                  branches:
                    - steps: []
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "together.branches[].label" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "together.branches[].label"
            })
        );
    }

    #[test]
    fn adversarial_ast_choose_branch_missing_when_rejected() {
        // Given: choose branch without a when condition
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-when
            when:
              manual: {}
            steps:
              - id: c1
                choose:
                  branches:
                    - steps: []
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "choose.branches[].when" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "choose.branches[].when"
            })
        );
    }

    #[test]
    fn adversarial_ast_ipc_trigger_missing_name_rejected() {
        // Given: IPC trigger without a name field
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-ipc-name
            when:
              ipc: {}
            steps: []
        "};
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::MissingField { field: "when.ipc.name" })
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "when.ipc.name"
            })
        );
    }

    #[test]
    fn adversarial_ast_when_with_empty_mapping_rejected() {
        // Given: when field with empty mapping (no manual or ipc)
        let yaml = "version: velvet-ballastics/v1\nname: bad\nwhen: {}\nsteps: []\n";
        // When: parsing
        let result = parse_workflow_ast(yaml);
        // Then: Err(YamlError::FieldShape) - empty when has no recognized trigger
        assert!(
            matches!(result, Err(YamlError::FieldShape { field, .. }) if field == "when"),
            "expected FieldShape for empty when, got: {result:?}"
        );
    }
}
