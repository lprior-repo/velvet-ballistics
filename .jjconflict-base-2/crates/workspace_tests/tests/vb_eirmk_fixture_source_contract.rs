#![forbid(unsafe_code)]

use vb_yaml::{ScalarValue, StepPrimitive, TriggerAst, YamlError, load_fixture_source};

const VALID_SOURCE: &str = "version: velvet-ballistics/v1\nname: fixture_contract\nwhen:\n  manual: {}\nsteps:\n  - id: create_answer\n    set:\n      output: answer\n      value: \"42\"\n  - id: finish_work\n    finish:\n      result: answer\n";

fn loaded_workflow() -> Result<vb_yaml::WorkflowSource, String> {
    load_fixture_source(VALID_SOURCE).map_err(|error| error.to_string())
}

#[test]
fn scenario_loads_fixture_workflow_metadata_and_ordered_step_ids() -> Result<(), String> {
    // Given: canonical workflow YAML text is held by the caller.
    // When: the public fixture-source loader parses that text.
    let workflow = loaded_workflow()?;
    // Then: workflow metadata, trigger, and source step order match independently expected values.
    assert_eq!(workflow.version(), "velvet-ballistics/v1");
    assert_eq!(workflow.name(), "fixture_contract");
    assert_eq!(workflow.trigger(), &TriggerAst::Manual);
    assert_eq!(
        workflow
            .steps()
            .iter()
            .map(|step| step.id.as_str())
            .collect::<Vec<_>>(),
        ["create_answer", "finish_work"]
    );
    Ok(())
}

#[test]
fn scenario_loads_expected_set_step_output_and_literal() -> Result<(), String> {
    // Given: canonical workflow YAML contains a set step.
    // When: the public loader parses that fixture text.
    let workflow = loaded_workflow()?;
    let Some(set_step) = workflow.steps().first() else {
        return Err("set step missing".to_owned());
    };
    // Then: the AST preserves the step identifier and primitive fields.
    assert_eq!(set_step.id, "create_answer");
    assert_eq!(
        set_step.primitive,
        StepPrimitive::Set {
            output: "answer".to_owned(),
            value: "42".to_owned()
        }
    );
    Ok(())
}

#[test]
fn scenario_loads_expected_finish_step_literal() -> Result<(), String> {
    // Given: canonical workflow YAML contains a finish step.
    // When: the public loader parses that fixture text.
    let workflow = loaded_workflow()?;
    let Some(finish_step) = workflow.steps().get(1) else {
        return Err("finish step missing".to_owned());
    };
    // Then: the AST preserves the finish identifier and literal result.
    assert_eq!(finish_step.id, "finish_work");
    assert_eq!(
        finish_step.primitive,
        StepPrimitive::Finish {
            result: ScalarValue::String("answer".to_owned())
        }
    );
    Ok(())
}

#[test]
fn scenario_rejects_empty_and_non_workflow_sources_with_exact_errors() {
    // Given: empty input and a YAML scalar that cannot represent a workflow mapping.
    // When: each value is sent through the public loader.
    let empty = load_fixture_source("");
    let scalar = load_fixture_source("plain scalar");
    // Then: each input produces its documented typed error and exact payload.
    assert_eq!(empty, Err(YamlError::EmptySource));
    assert_eq!(
        scalar,
        Err(YamlError::FieldShape {
            field: "workflow",
            expected: "mapping"
        })
    );
}

#[test]
fn scenario_reports_malformed_yaml_with_location_and_reason() -> Result<(), String> {
    let source = "version: [unfinished\n";
    let result = load_fixture_source(source);
    let Err(YamlError::ParseError { line, reason }) = result else {
        return Err(format!("expected ParseError, received {result:?}"));
    };
    assert_eq!(line, 2);
    assert_eq!(
        reason.as_ref(),
        "while parsing a flow sequence, expected ',' or ']'"
    );
    Ok(())
}

#[test]
fn scenario_rejects_yaml_anchor_features_under_strict_profile() {
    // Given: a workflow-shaped document containing a YAML anchor.
    let source =
        "version: velvet-ballistics/v1\nname: anchor\nwhen:\n  manual: &trigger {}\nsteps: []\n";
    // When: the public loader validates and parses that source.
    let result = load_fixture_source(source);
    // Then: strict-profile validation returns the exact typed rejection.
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn scenario_repeated_loads_are_independent_and_leave_input_unchanged() -> Result<(), String> {
    // Given: caller-owned fixture text and a saved copy of its bytes.
    let source = String::from(VALID_SOURCE);
    let original = source.clone();
    // When: the same input is loaded twice.
    let first = load_fixture_source(&source).map_err(|error| error.to_string())?;
    let second = load_fixture_source(&source).map_err(|error| error.to_string())?;
    // Then: outputs agree and the caller's text is unchanged.
    assert_eq!(first, second);
    assert_eq!(source, original);
    assert_eq!(first.name(), "fixture_contract");
    Ok(())
}
