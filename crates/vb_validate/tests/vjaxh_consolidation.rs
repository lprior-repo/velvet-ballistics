#![forbid(unsafe_code)]

use vb_validate::ValidationError;
use vb_validate::references::{RefTables, validate_single_reference_with_context};
use vb_validate::schema::{
    FieldValue, StepDoc, WorkflowDoc, validate_step_fields, validate_trigger,
};
use vb_validate::type_taint::{
    ResourceLimits, StepKind, StepTypes, Taint, TypedValue, ValueType, WorkflowTypes,
    validate_resource_limits, validate_taint,
};

fn string_value(text: &str) -> FieldValue {
    FieldValue::String(text.to_owned())
}

fn step_doc(primitive: &str) -> StepDoc {
    StepDoc::from_pairs(vec![
        ("id".to_owned(), string_value("step_one")),
        (primitive.to_owned(), FieldValue::Empty),
    ])
}

fn steps_doc(steps: Vec<StepDoc>) -> WorkflowDoc {
    WorkflowDoc::from_pairs(vec![("steps".to_owned(), FieldValue::Sequence(steps))])
}

#[test]
fn reference_public_api_keeps_prior_step_context() {
    let empty: Vec<String> = Vec::new();
    let step_ids = vec!["build".to_owned(), "done".to_owned()];
    let tables = RefTables::from_slices(&empty, &empty, &empty, &step_ids);

    assert_eq!(
        validate_single_reference_with_context(
            "$steps.build.output",
            &tables,
            Some(1),
            false,
            false,
        ),
        Ok(())
    );
    assert_eq!(
        validate_single_reference_with_context(
            "$steps.done.output",
            &tables,
            Some(1),
            false,
            false,
        ),
        Err(ValidationError::FutureReference {
            reference: "$steps.done.output".to_owned(),
        })
    );
}

#[test]
fn schema_public_api_accepts_canonical_primitives() {
    let together = steps_doc(vec![step_doc("together")]);
    let reduce = steps_doc(vec![step_doc("reduce")]);

    assert_eq!(validate_step_fields(&together), Ok(()));
    assert_eq!(validate_step_fields(&reduce), Ok(()));
}

#[test]
fn schema_public_api_rejects_legacy_primitives_and_event_name() {
    let parallel = steps_doc(vec![step_doc("parallel")]);
    let aggregate = steps_doc(vec![step_doc("aggregate")]);
    let event_name = WorkflowDoc::from_pairs(vec![(
        "when".to_owned(),
        FieldValue::Mapping(vec![(
            "event".to_owned(),
            FieldValue::Mapping(vec![("name".to_owned(), string_value("job.created"))]),
        )]),
    )]);

    assert_eq!(
        validate_step_fields(&parallel),
        Err(ValidationError::UnknownStepField)
    );
    assert_eq!(
        validate_step_fields(&aggregate),
        Err(ValidationError::UnknownStepField)
    );
    assert_eq!(
        validate_trigger(&event_name),
        Err(ValidationError::UnsupportedTrigger {
            trigger: "event".to_owned(),
        })
    );
}

#[test]
fn type_taint_public_api_keeps_finish_secret_acceptance_and_limits() {
    let mut workflow = WorkflowTypes::default();
    workflow.secrets.push("token".to_owned());
    workflow.steps.push(StepTypes {
        id: "done".to_owned(),
        kind: StepKind::Finish {
            result: TypedValue::Reference("$secrets.token".to_owned()),
        },
    });

    assert_eq!(validate_taint(&workflow), Ok(()));
    assert_eq!(
        Taint::Clean.merge(Taint::DerivedFromSecret),
        Taint::DerivedFromSecret
    );
    assert_eq!(Taint::DerivedFromSecret.merge(Taint::Secret), Taint::Secret);

    let zero_step_limit = WorkflowTypes {
        resource_contract: ResourceLimits {
            max_steps: 0,
            ..ResourceLimits::default()
        },
        steps: vec![StepTypes {
            id: "done".to_owned(),
            kind: StepKind::Finish {
                result: TypedValue::Literal(ValueType::Null),
            },
        }],
        ..WorkflowTypes::default()
    };

    assert_eq!(
        validate_resource_limits(&zero_step_limit, &ResourceLimits::default()),
        Err(ValidationError::LimitRequired {
            resource: "max_steps".to_owned(),
        })
    );
}
