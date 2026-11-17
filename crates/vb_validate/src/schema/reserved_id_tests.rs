//! Public production-schema regressions, not the schema_support test model.
use crate::schema::{FieldValue, StepDoc, WorkflowDoc, validate_workflow_schema};
use crate::ValidationError;

fn step(id: &str) -> StepDoc {
    StepDoc::from_pairs(vec![
        ("id".into(), FieldValue::String(id.into())),
        ("set".into(), FieldValue::Empty),
    ])
}
fn workflow(name: &str, ids: &[&str]) -> WorkflowDoc {
    document("velvet-ballistics/v1", name, ids.iter().map(|id|step(id)).collect())
}
fn document(version: &str, name: &str, steps: Vec<StepDoc>) -> WorkflowDoc {
    WorkflowDoc::from_pairs(vec![
        ("version".into(),FieldValue::String(version.into())),
        ("name".into(),FieldValue::String(name.into())),
        ("when".into(),FieldValue::Mapping(vec![("manual".into(),FieldValue::Empty)])),
        ("steps".into(),FieldValue::Sequence(steps)),
    ])
}

#[test]
fn current_container_keywords_are_reserved_in_public_workflow_and_step_ids() {
    for reserved in ["together","reduce"] {
        assert_eq!(validate_workflow_schema(&workflow(reserved,&["valid_step"])),
            Err(ValidationError::ReservedId{id:format!("name: {reserved}")}));
        assert_eq!(validate_workflow_schema(&workflow("valid_workflow",&[reserved])),
            Err(ValidationError::ReservedId{id:reserved.into()}));
        assert_eq!(validate_workflow_schema(&workflow("valid_workflow",&["first",reserved,"last"])),
            Err(ValidationError::ReservedId{id:reserved.into()}));
    }
}

#[test]
fn existing_reserved_contract_members_remain_rejected_by_public_schema() {
    // Membership, not implementation table length: adding two names cannot hide deletion.
    for reserved in ["now","random","runtime","null","true","false","input","inputs",
        "vars","secrets","steps","error","attempt","total","result","when","item",
        "do","set","choose","for_each","collect","repeat","wait","ask","try_again",
        "on_error","then","finish"] {
        assert_eq!(validate_workflow_schema(&workflow(reserved,&["valid_step"])),
            Err(ValidationError::ReservedId{id:format!("name: {reserved}")}));
        assert_eq!(validate_workflow_schema(&workflow("valid_workflow",&[reserved])),
            Err(ValidationError::ReservedId{id:reserved.into()}));
    }
}

#[test]
fn nonreserved_neighbors_and_legacy_words_remain_valid_ids_not_new_primitives() {
    for allowed in ["together2","together_value","reduce2","reduce_value","parallel","aggregate","valid_workflow"] {
        assert_eq!(validate_workflow_schema(&workflow(allowed,&[allowed])),Ok(()));
    }
    // These are ID spellings only; each step remains a canonical Set primitive.
    assert_eq!(validate_workflow_schema(&workflow("workflow",&["parallel","aggregate","together2","reduce2"])),Ok(()));
    assert_eq!(validate_workflow_schema(&workflow("workflow",&["parallel","parallel"])),
        Err(ValidationError::DuplicateId{id:"parallel".into()}));
}

#[test]
fn reserved_diagnostics_preserve_schema_validation_order_and_field_identity() {
    assert_eq!(validate_workflow_schema(&workflow("together",&["reduce"])),
        Err(ValidationError::ReservedId{id:"name: together".into()}));
    assert_eq!(validate_workflow_schema(&workflow("workflow",&["reduce","together"])),
        Err(ValidationError::ReservedId{id:"reduce".into()}));
    assert_eq!(validate_workflow_schema(&workflow("workflow",&["together","together"])),
        Err(ValidationError::ReservedId{id:"together".into()}));
    assert_eq!(validate_workflow_schema(&document("bad-version","together",vec![step("reduce")])),
        Err(ValidationError::InvalidVersion{version:"bad-version".into()}));
    let malformed=StepDoc::from_pairs(vec![("id".into(),FieldValue::String("reduce".into()))]);
    assert_eq!(validate_workflow_schema(&document("velvet-ballistics/v1","workflow",vec![malformed])),
        Err(ValidationError::ReservedId{id:"reduce".into()}));
    assert_eq!(validate_workflow_schema(&workflow("Together",&["valid_step"])),
        Err(ValidationError::InvalidId{id:"name: Together".into()}));
    assert_eq!(validate_workflow_schema(&workflow("workflow",&["Reduce"])),
        Err(ValidationError::InvalidId{id:"Reduce".into()}));
}
