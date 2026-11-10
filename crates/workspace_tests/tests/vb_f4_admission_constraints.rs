#![forbid(unsafe_code)]

use vb_core::input_validation::InputValueIssue;
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::{
    CompiledWorkflow, ObjectField, RunId, SlotIdx, SlotValue, Taint, ValueStore, WorkflowDigest,
};
use vb_storage::{JournalError, RunInputsRecord};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn workflow(schema: &str) -> TestResult<CompiledWorkflow> {
    let source = format!(
        "version: velvet-ballistics/v1\nname: input_contract\nwhen: {{manual: {{}}}}\ninputs:\n  value: {schema}\nsteps:\n  - id: first\n    set: {{output: answer, value: '1'}}\n  - id: done\n    finish: {{result: answer}}\n"
    );
    Ok(vb_compile::compile_workflow(source.as_bytes())?)
}

fn slot(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    Ok(workflow
        .input_slots()
        .first()
        .ok_or("missing input declaration")?
        .slot)
}

fn record(
    workflow: &CompiledWorkflow,
    inputs: Box<[(SlotIdx, SlotValue)]>,
) -> Result<RunInputsRecord, JournalError> {
    RunInputsRecord::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([1; 32]),
        WorkflowDigest::from_bytes([2; 32]),
        workflow,
        inputs,
    )
}

#[test]
fn nullable_scalar_is_accepted_by_scalar_and_owned_admission() -> TestResult {
    let workflow = workflow("{is: number, nullable: true}")?;
    let slot = slot(&workflow)?;
    let scalar = record(&workflow, Box::from([(slot, SlotValue::Null)]))?;
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let owned = OwnedRootBundle::new(
        Box::from([(slot, SlotValue::Null, Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    assert_eq!(scalar.inputs(), &[(slot, SlotValue::Null)]);
    assert_eq!(owned.root(slot), Some((SlotValue::Null, Taint::Clean)));
    Ok(())
}

#[test]
fn required_optional_and_scalar_defaults_match_across_admission_paths() -> TestResult {
    let required = workflow("number")?;
    let store = ValueStore::with_max_slots(required.resource_contract().max_slots);
    assert!(matches!(
        record(&required, Box::from([])),
        Err(JournalError::InputSchemaMismatch)
    ));
    assert!(
        matches!(OwnedRootBundle::new(Box::from([]), store.checkpoint_v1(), &required), Err(OwnedRootBundleError::RootSlotMissing { slot: missing }) if missing == slot(&required)?)
    );
    for (schema, expected) in [
        ("{is: number, optional: true}", None),
        ("{is: number, default: 7}", Some(SlotValue::I64(7))),
    ] {
        let workflow = workflow(schema)?;
        let slot = slot(&workflow)?;
        let scalar = record(&workflow, Box::from([]))?;
        let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let owned = OwnedRootBundle::new(Box::from([]), store.checkpoint_v1(), &workflow)?;
        assert_eq!(scalar.inputs().first().map(|(_, value)| *value), expected);
        assert_eq!(owned.root(slot).map(|(value, _)| value), expected);
    }
    Ok(())
}

#[test]
fn numeric_bounds_are_inclusive_and_reject_before_retention() -> TestResult {
    let workflow = workflow("{is: number, min: 1, max: 10}")?;
    let slot = slot(&workflow)?;
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    for value in [1, 10] {
        record(&workflow, Box::from([(slot, SlotValue::I64(value))]))?;
        OwnedRootBundle::new(
            Box::from([(slot, SlotValue::I64(value), Taint::Clean)]),
            store.checkpoint_v1(),
            &workflow,
        )?;
    }
    for value in [0, 11] {
        assert!(matches!(
            record(&workflow, Box::from([(slot, SlotValue::I64(value))])),
            Err(JournalError::InputSchemaMismatch)
        ));
        assert!(
            matches!(OwnedRootBundle::new(Box::from([(slot, SlotValue::I64(value), Taint::Clean)]), store.checkpoint_v1(), &workflow), Err(OwnedRootBundleError::RootConstraintMismatch { slot: failed, issue: InputValueIssue::Bounds }) if failed == slot)
        );
    }
    Ok(())
}

#[test]
fn owned_text_bounds_measure_utf8_bytes() -> TestResult {
    for (schema, accepted) in [
        ("{is: text, min_length: 4, max_length: 4}", true),
        ("{is: text, max_length: 3}", false),
        ("{is: text, min_length: 5}", false),
    ] {
        let workflow = workflow(schema)?;
        let slot = slot(&workflow)?;
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let value = SlotValue::Symbol(store.insert_symbol("éé")?);
        let result = OwnedRootBundle::new(
            Box::from([(slot, value, Taint::Clean)]),
            store.checkpoint_v1(),
            &workflow,
        );
        if accepted {
            result?;
        } else {
            assert!(
                matches!(result, Err(OwnedRootBundleError::RootConstraintMismatch { slot: failed, issue: InputValueIssue::Bounds }) if failed == slot)
            );
        }
    }
    Ok(())
}

#[test]
fn owned_list_checks_length_and_element_kind() -> TestResult {
    let workflow = workflow("{is: list, of: number, min: 1, max: 2}")?;
    let slot = slot(&workflow)?;
    for (values, issue) in [
        (vec![SlotValue::I64(1)], None),
        (vec![SlotValue::I64(1), SlotValue::I64(2)], None),
        (vec![], Some(InputValueIssue::Bounds)),
        (
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
            Some(InputValueIssue::Bounds),
        ),
        (vec![SlotValue::Bool(true)], Some(InputValueIssue::Element)),
    ] {
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let value = SlotValue::List(store.insert_list(values.into_boxed_slice())?);
        let result = OwnedRootBundle::new(
            Box::from([(slot, value, Taint::Clean)]),
            store.checkpoint_v1(),
            &workflow,
        );
        if let Some(expected) = issue {
            assert!(
                matches!(result, Err(OwnedRootBundleError::RootConstraintMismatch { slot: failed, issue }) if failed == slot && issue == expected)
            );
        } else {
            result?;
        }
    }
    Ok(())
}

#[test]
fn closed_empty_object_schema_rejects_extra_fields() -> TestResult {
    let workflow = workflow("{is: object, fields: {}, extra: reject}")?;
    let slot = slot(&workflow)?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let key = store.insert_symbol("unexpected")?;
    let object = store.insert_object(Box::from([ObjectField::clean(key, SlotValue::I64(1))]))?;
    assert!(
        matches!(OwnedRootBundle::new(Box::from([(slot, SlotValue::Object(object), Taint::Clean)]), store.checkpoint_v1(), &workflow), Err(OwnedRootBundleError::RootConstraintMismatch { slot: failed, issue: InputValueIssue::ExtraField }) if failed == slot)
    );
    Ok(())
}

#[test]
fn owned_projection_binding_compares_retained_constraints() -> TestResult {
    let workflow = workflow("{is: number, min: 1, max: 10}")?;
    let slot = slot(&workflow)?;
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let owned = OwnedRootBundle::new(
        Box::from([(slot, SlotValue::I64(3), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let mut parts = workflow.to_parts();
    parts
        .input_slots
        .first_mut()
        .ok_or("missing input declaration")?
        .constraint
        .max = Some(9);
    let changed = CompiledWorkflow::try_from_parts(parts)?;
    assert!(!owned.matches_workflow(&changed));
    assert!(owned.matches_workflow(&workflow));
    Ok(())
}
