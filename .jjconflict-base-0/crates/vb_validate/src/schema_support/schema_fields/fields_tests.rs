#![forbid(unsafe_code)]

use crate::schema_support::schema_doc::{FieldValue, StepDoc, WorkflowDoc};

#[path = "core.rs"]
mod core;
#[path = "ids.rs"]
mod ids;
#[path = "step.rs"]
mod step;
#[path = "trigger.rs"]
mod trigger;

fn make_workflow(fields: Vec<(&str, FieldValue)>) -> WorkflowDoc {
    WorkflowDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn make_step(fields: Vec<(&str, FieldValue)>) -> StepDoc {
    StepDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn valid_workflow_doc() -> WorkflowDoc {
    make_workflow(vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("test".to_owned())),
        (
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        ),
        (
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("step1".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ])
}

fn base_valid_fields() -> Vec<(&'static str, FieldValue)> {
    vec![
        (
            "version",
            FieldValue::String("velvet-ballistics/v1".to_owned()),
        ),
        ("name", FieldValue::String("test".to_owned())),
        (
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        ),
        (
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
            ])]),
        ),
    ]
}
