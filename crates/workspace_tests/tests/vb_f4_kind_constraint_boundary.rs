#![forbid(unsafe_code)]

use vb_core::workflow::{InputConstraintIssue, WorkflowError};
use vb_core::{CompiledInputConstraint, CompiledInputField, CompiledWorkflow, InputSlotKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parts() -> Result<vb_core::WorkflowParts, vb_compile::CompileErrors> {
    vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: kind_boundary\nwhen: {manual: {}}\ninputs: {value: any}\nsteps:\n  - id: seed\n    set: {output: answer, value: '42'}\n  - id: done\n    finish: {result: answer}\n").map(|workflow| workflow.to_parts())
}

#[test]
fn public_parts_reject_kind_incompatible_keywords_at_root_and_nested_fields() -> TestResult {
    let baseline = parts()?;
    for (kind, keyword) in [
        (InputSlotKind::Bool, "min"),
        (InputSlotKind::Bool, "max"),
        (InputSlotKind::I64, "min_length"),
        (InputSlotKind::I64, "max_length"),
        (InputSlotKind::Bool, "element"),
        (InputSlotKind::Bool, "allow_extra"),
        (InputSlotKind::Bool, "fields"),
        (InputSlotKind::Any, "fields"),
        (InputSlotKind::Any, "min"),
        (InputSlotKind::Any, "max_length"),
        (InputSlotKind::Symbol, "element"),
        (InputSlotKind::List, "allow_extra"),
        (InputSlotKind::F64, "min"),
        (InputSlotKind::Any, "allow_extra"),
    ] {
        for nested in [false, true] {
            let mut parts = baseline.clone();
            let mut constraint = CompiledInputConstraint::required();
            match keyword {
                "min" => constraint.min = Some(1),
                "max" => constraint.max = Some(2),
                "min_length" => constraint.min_length = Some(1),
                "max_length" => constraint.max_length = Some(2),
                "element" => constraint.element = Some(InputSlotKind::Bool),
                "allow_extra" => constraint.allow_extra = Some(false),
                "fields" => constraint.fields = Some(Box::from([])),
                _ => return Err("unknown fixture keyword".into()),
            }
            let input = parts.input_slots.first_mut().ok_or("input")?;
            if nested {
                input.kind = InputSlotKind::Object;
                input.constraint.fields = Some(Box::from([CompiledInputField {
                    name: "child".into(),
                    kind,
                    constraint,
                }]));
            } else {
                input.kind = kind;
                input.constraint = constraint;
            }
            let slot = input.slot;
            let expected = if keyword == "fields" {
                InputConstraintIssue::InvalidFieldSchema
            } else {
                InputConstraintIssue::KeywordKindMismatch
            };
            assert!(
                matches!(CompiledWorkflow::try_from_parts(parts),
                    Err(WorkflowError::InconsistentInputConstraint { slot: actual, issue }) if actual == slot && issue == expected
                ),
                "accepted {keyword} on {kind:?}; nested={nested}"
            );
        }
    }
    Ok(())
}

#[test]
fn public_parts_reject_negative_list_lengths_at_root_and_nested_fields() -> TestResult {
    for maximum in [false, true] {
        for nested in [false, true] {
            let mut parts = parts()?;
            let input = parts.input_slots.first_mut().ok_or("input")?;
            let mut constraint = CompiledInputConstraint::required();
            if maximum {
                constraint.max = Some(-1);
            } else {
                constraint.min = Some(-1);
            }
            if nested {
                input.kind = InputSlotKind::Object;
                input.constraint.fields = Some(Box::from([CompiledInputField {
                    name: "child".into(),
                    kind: InputSlotKind::List,
                    constraint,
                }]));
            } else {
                input.kind = InputSlotKind::List;
                input.constraint = constraint;
            }
            let slot = input.slot;
            assert!(matches!(CompiledWorkflow::try_from_parts(parts),
                Err(WorkflowError::InconsistentInputConstraint { slot: actual, issue: InputConstraintIssue::NegativeLengthBound }) if actual == slot
            ));
        }
    }
    Ok(())
}

#[test]
fn valid_keyword_kind_combinations_and_unconstrained_ir_kinds_remain_accepted() -> TestResult {
    for schema in [
        "{is: number, min: -4, max: 4}",
        "{is: text, min_length: 0, max_length: 4}",
        "{is: list, of: number, min: 0, max: 4}",
        "{is: object, extra: reject, fields: {child: boolean}}",
    ] {
        let mut text = String::from(
            "version: velvet-ballistics/v1\nname: valid_kind\nwhen: {manual: {}}\ninputs:\n  value: ",
        );
        text.push_str(schema);
        text.push_str("\nsteps:\n  - id: seed\n    set: {output: answer, value: '42'}\n  - id: done\n    finish: {result: answer}\n");
        let workflow = vb_compile::compile_workflow(text.as_bytes())?;
        assert_eq!(
            CompiledWorkflow::try_from_parts(workflow.to_parts())?.input_slots(),
            workflow.input_slots()
        );
    }
    for kind in [
        InputSlotKind::Any,
        InputSlotKind::Null,
        InputSlotKind::Bool,
        InputSlotKind::I64,
        InputSlotKind::F64,
        InputSlotKind::Symbol,
        InputSlotKind::List,
        InputSlotKind::Object,
        InputSlotKind::Blob,
    ] {
        let mut parts = parts()?;
        let input = parts.input_slots.first_mut().ok_or("input")?;
        input.kind = kind;
        input.constraint = CompiledInputConstraint::unconstrained();
        let workflow = CompiledWorkflow::try_from_parts(parts)?;
        assert_eq!(workflow.input_slots().first().ok_or("input")?.kind, kind);
    }
    Ok(())
}
