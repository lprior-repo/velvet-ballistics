use vb_compile::{CompileError, YamlError, compile_workflow, parse_workflow_source};
use vb_core::{InputDefaultValue, InputSlotKind};

const HEADER: &str = "version: velvet-ballistics/v1\nname: wait_context\nwhen: {manual: {}}\n";

#[test]
fn empty_event_spelling_in_comments_is_not_a_wait_declaration()
-> Result<(), Box<dyn std::error::Error>> {
    let source = format!(
        "{HEADER}# event: \"\"\nsteps:\n  - id: save_value\n    save: {{output: saved, value: '1'}}\n  - id: done\n    finish: {{result: saved}}\n"
    );
    let workflow = compile_workflow(source.as_bytes())?;
    assert_eq!(workflow.input_slots(), &[]);
    Ok(())
}

#[test]
fn empty_event_spelling_in_default_text_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let source = format!(
        "{HEADER}inputs:\n  message: {{is: text, default: 'event: \"\"'}}\nsteps:\n  - id: done\n    finish: {{result: '$input.message'}}\n"
    );
    let workflow = compile_workflow(source.as_bytes())?;
    let [input] = workflow.input_slots() else {
        return Err("expected exactly one input".into());
    };
    assert_eq!(input.kind, InputSlotKind::Symbol);
    assert_eq!(
        input.constraint.default_cold,
        Some(InputDefaultValue::Text("event: \"\"".into()))
    );
    Ok(())
}

#[test]
fn actual_empty_wait_events_are_rejected_independently_of_yaml_spelling() {
    for field in [
        "event: \"\"",
        "event: ''",
        "'event': \"\"",
        "event:    \"\"",
    ] {
        for nested in [false, true] {
            let steps = if nested {
                format!(
                    "  - id: retries\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: wait_for_signal\n          wait: {{{field}}}\n"
                )
            } else {
                format!("  - id: wait_for_signal\n    wait: {{{field}}}\n")
            };
            let source =
                format!("{HEADER}steps:\n{steps}  - id: done\n    finish: {{result: 1}}\n");
            assert_eq!(
                parse_workflow_source(&source).map(|_| ()),
                Err(YamlError::FieldShape {
                    field: "wait.event",
                    expected: "non-empty string",
                })
            );
            assert_eq!(
                compile_workflow(source.as_bytes()).map(|_| ()),
                Err(vb_compile::CompileErrors(vec![
                    CompileError::CanonicalYaml {
                        category: "field_shape",
                        message: "wait.event must be non-empty".into(),
                    }
                ]))
            );
        }
    }
}
