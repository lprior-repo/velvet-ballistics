//! Public production-schema regressions, not the schema_support test model.
use crate::ValidationError;
use crate::schema::{FieldValue, StepDoc, WorkflowDoc, validate_workflow_schema};

fn step(id: &str) -> StepDoc {
    StepDoc::from_pairs(vec![
        ("id".into(), FieldValue::String(id.into())),
        ("set".into(), FieldValue::Empty),
    ])
}
fn workflow(name: &str, ids: &[&str]) -> WorkflowDoc {
    document(
        "velvet-ballistics/v1",
        name,
        ids.iter().map(|id| step(id)).collect(),
    )
}
fn document(version: &str, name: &str, steps: Vec<StepDoc>) -> WorkflowDoc {
    WorkflowDoc::from_pairs(vec![
        ("version".into(), FieldValue::String(version.into())),
        ("name".into(), FieldValue::String(name.into())),
        (
            "when".into(),
            FieldValue::Mapping(vec![("manual".into(), FieldValue::Empty)]),
        ),
        ("steps".into(), FieldValue::Sequence(steps)),
    ])
}

fn primitive_step(fields: &[&str]) -> StepDoc {
    let mut pairs = vec![("id".into(), FieldValue::String("valid_step".into()))];
    pairs.extend(
        fields
            .iter()
            .map(|field| ((*field).into(), FieldValue::Empty)),
    );
    StepDoc::from_pairs(pairs)
}

#[test]
fn public_schema_accepts_each_canonical_primitive_field() {
    // Schema checks primitive membership, not the executable contents of these bodies.
    // Literal contract members deliberately do not come from implementation tables.
    for primitive in [
        "set", "do", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait",
        "ask", "finish",
    ] {
        let candidate = primitive_step(&[primitive]);
        assert_eq!(
            crate::schema::validate_single_primitive(&candidate),
            Ok(()),
            "{primitive}"
        );
        assert_eq!(
            validate_workflow_schema(&document(
                "velvet-ballistics/v1",
                "workflow",
                vec![candidate]
            )),
            Ok(()),
            "{primitive}",
        );
    }
}

#[test]
fn public_schema_rejects_legacy_primitive_aliases_not_their_id_spellings() {
    for alias in ["parallel", "aggregate"] {
        let candidate = primitive_step(&[alias]);
        assert_eq!(
            crate::schema::validate_single_primitive(&candidate),
            Err(ValidationError::MissingStepPrimitive)
        );
        assert_eq!(
            validate_workflow_schema(&document(
                "velvet-ballistics/v1",
                "workflow",
                vec![candidate]
            )),
            Err(ValidationError::UnknownStepField),
        );
        // An otherwise recognized primitive cannot hide an unknown alias field.
        let mixed = primitive_step(&["set", alias]);
        assert_eq!(crate::schema::validate_single_primitive(&mixed), Ok(()));
        assert_eq!(
            validate_workflow_schema(&document("velvet-ballistics/v1", "workflow", vec![mixed])),
            Err(ValidationError::UnknownStepField),
        );
        assert_eq!(validate_workflow_schema(&workflow(alias, &[alias])), Ok(()));
    }
}

#[test]
fn public_schema_counts_current_container_primitives_and_rejects_missing_primitive() {
    for fields in [
        ["together", "reduce"],
        ["reduce", "together"],
        ["set", "together"],
        ["reduce", "do"],
    ] {
        let candidate = primitive_step(&fields);
        assert_eq!(
            crate::schema::validate_single_primitive(&candidate),
            Err(ValidationError::MultipleStepPrimitives)
        );
        assert_eq!(
            validate_workflow_schema(&document(
                "velvet-ballistics/v1",
                "workflow",
                vec![candidate]
            )),
            Err(ValidationError::MultipleStepPrimitives),
        );
    }
    let missing = primitive_step(&[]);
    assert_eq!(
        crate::schema::validate_single_primitive(&missing),
        Err(ValidationError::MissingStepPrimitive)
    );
    assert_eq!(
        validate_workflow_schema(&document("velvet-ballistics/v1", "workflow", vec![missing])),
        Err(ValidationError::MissingStepPrimitive),
    );
}
