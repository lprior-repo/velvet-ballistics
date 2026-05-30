// Shared test helpers for digest behavior tests.
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// Provides reusable constructors for WorkflowSource values used
// across all digest behavior test files. Each function builds a
// minimal valid source with the specified primitive configuration.

#![forbid(unsafe_code)]
#![allow(dead_code)] // shared test helpers used across multiple test targets

use vb_yaml::ast::{
    ScalarValue, StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts,
};

/// Build a minimal WorkflowSource with a single Ask step and a Finish step.
///
/// The Finish step is required because valid workflow YAML always ends with
/// a terminating primitive (Finish is the simplest).
pub(crate) fn ask_source(prompt: &str, timeout: Option<&str>) -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_ask_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            StepAst {
                id: "ask_1".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Ask {
                    prompt: prompt.to_string(),
                    timeout: timeout.map(|s| s.to_string()),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
            StepAst {
                id: "finish_1".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Finish {
                    result: ScalarValue::String("done".to_string()),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
        ],
        result: None,
        examples: vec![],
    })
}

/// Build a WorkflowSource with only a Set step.
pub(crate) fn set_source(output: &str, value: &str) -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_set_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![StepAst {
            id: "set_1".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Set {
                output: output.to_string(),
                value: value.to_string(),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }],
        result: None,
        examples: vec![],
    })
}

/// Build a WorkflowSource with only a Finish step (String result).
pub(crate) fn finish_source_string(result: &str) -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_finish_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![StepAst {
            id: "finish_1".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Finish {
                result: ScalarValue::String(result.to_string()),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }],
        result: None,
        examples: vec![],
    })
}

/// Build a WorkflowSource with only a Finish step (Integer result).
pub(crate) fn finish_source_integer(value: i64) -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_finish_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![StepAst {
            id: "finish_1".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Finish {
                result: ScalarValue::Integer(value),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }],
        result: None,
        examples: vec![],
    })
}

/// Build a WorkflowSource with Set + Finish steps (regression test).
pub(crate) fn set_finish_source() -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_set_finish_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            StepAst {
                id: "set_1".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Set {
                    output: "x".to_string(),
                    value: "1".to_string(),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
            StepAst {
                id: "finish_1".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Finish {
                    result: ScalarValue::String("done".to_string()),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
        ],
        result: None,
        examples: vec![],
    })
}

/// Build a WorkflowSource with zero steps.
pub(crate) fn empty_source() -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_empty_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        result: None,
        examples: vec![],
    })
}

/// Build a WorkflowSource with a custom name (for name-sensitivity tests).
pub(crate) fn named_source(name: &str, steps: Vec<StepAst>) -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: name.to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        result: None,
        examples: vec![],
    })
}

/// Build a WorkflowSource with a custom trigger (for trigger-sensitivity tests).
pub(crate) fn triggered_source(trigger: TriggerAst, steps: Vec<StepAst>) -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        result: None,
        examples: vec![],
    })
}

/// Build a WorkflowSource with a custom version and name.
pub(crate) fn versioned_source(version: &str, name: &str, steps: Vec<StepAst>) -> WorkflowSource {
    WorkflowSource::new(WorkflowSourceParts {
        version: version.to_string(),
        name: name.to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        result: None,
        examples: vec![],
    })
}
