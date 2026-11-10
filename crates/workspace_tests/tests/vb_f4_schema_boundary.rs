use vb_core::workflow::{InputConstraintIssue, WorkflowError};
use vb_core::{
    CompiledInputConstraint, CompiledInputField, CompiledWorkflow, InputDefaultValue, InputSlotKind,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parts() -> Result<vb_core::WorkflowParts, vb_compile::CompileErrors> {
    vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: schema_boundary\nwhen: {manual: {}}\ninputs: {value: any}\nsteps:\n  - id: seed\n    set: {output: answer, value: '42'}\n  - id: done\n    finish: {result: answer}\n").map(|workflow| workflow.to_parts())
}

fn constraint(depth: usize) -> CompiledInputConstraint {
    let mut constraint = CompiledInputConstraint::required();
    for _ in 0..depth {
        let mut parent = CompiledInputConstraint::required();
        parent.fields = Some(Box::from([CompiledInputField {
            name: "child".into(),
            kind: InputSlotKind::Object,
            constraint,
        }]));
        constraint = parent;
    }
    constraint
}

#[test]
fn binary_schema_depth_is_bounded_and_failure_resets_decode_context() -> TestResult {
    let valid = constraint(vb_core::input_schema::MAX_INPUT_SCHEMA_DEPTH);
    let bytes = postcard::to_allocvec(&valid)?;
    assert_eq!(
        postcard::from_bytes::<CompiledInputConstraint>(&bytes)?,
        valid
    );
    let invalid = constraint(
        vb_core::input_schema::MAX_INPUT_SCHEMA_DEPTH
            .checked_add(1)
            .ok_or("depth overflow")?,
    );
    assert_eq!(
        postcard::from_bytes::<CompiledInputConstraint>(&postcard::to_allocvec(&invalid)?),
        Err(postcard::Error::SerdeDeCustom)
    );
    assert_eq!(
        postcard::from_bytes::<CompiledInputConstraint>(&bytes)?,
        valid
    );
    Ok(())
}

#[test]
fn malformed_deep_default_bytes_fail_without_constructing_an_unbounded_tree() -> TestResult {
    // Postcard List variant (4), one item (1), repeated before the Null leaf.
    let mut bytes = [4u8, 1].repeat(10_000);
    bytes.push(0);
    assert_eq!(
        postcard::from_bytes::<InputDefaultValue>(&bytes),
        Err(postcard::Error::SerdeDeCustom)
    );
    assert_eq!(
        postcard::from_bytes::<InputDefaultValue>(&[0])?,
        InputDefaultValue::Null
    );
    let mut truncated = postcard::to_allocvec(&constraint(2))?;
    truncated.pop();
    assert_eq!(
        postcard::from_bytes::<CompiledInputConstraint>(&truncated),
        Err(postcard::Error::DeserializeUnexpectedEnd)
    );
    assert_eq!(
        postcard::from_bytes::<InputDefaultValue>(&[0])?,
        InputDefaultValue::Null
    );
    Ok(())
}

#[test]
fn untrusted_ir_rejects_nested_invalid_constraints_and_defaults() -> TestResult {
    for (schema, issue) in [
        (
            {
                let mut c = constraint(1);
                if let Some(field) = c.fields.as_mut().and_then(|fields| fields.first_mut()) {
                    field.constraint.min = Some(3);
                    field.constraint.max = Some(2);
                }
                c
            },
            InputConstraintIssue::MinExceedsMax,
        ),
        (
            {
                let mut c = constraint(1);
                if let Some(fields) = c.fields.as_mut() {
                    let mut duplicate = fields.to_vec();
                    duplicate.extend_from_slice(fields);
                    *fields = duplicate.into_boxed_slice();
                }
                c
            },
            InputConstraintIssue::InvalidFieldSchema,
        ),
        (
            {
                let mut c = CompiledInputConstraint::required();
                c.default_value = Some(vb_core::SlotValue::I64(3));
                c.default_cold = Some(InputDefaultValue::I64(4));
                c
            },
            InputConstraintIssue::ConflictingDefaults,
        ),
        (
            {
                let mut c = CompiledInputConstraint::required();
                c.default_cold = Some(InputDefaultValue::Text("wrong".into()));
                c
            },
            InputConstraintIssue::InvalidDefault,
        ),
        (constraint(65), InputConstraintIssue::SchemaCapacity),
        (
            {
                let mut c = CompiledInputConstraint::required();
                let count = vb_core::limits::MAX_LIST_ITEMS_PER_VALUE
                    .checked_add(1)
                    .ok_or("list bound overflow")?;
                c.default_cold = Some(InputDefaultValue::List(
                    vec![InputDefaultValue::Null; count].into_boxed_slice(),
                ));
                c
            },
            InputConstraintIssue::SchemaCapacity,
        ),
        (
            {
                let mut c = CompiledInputConstraint::required();
                let count = vb_core::limits::MAX_OBJECT_FIELDS_PER_VALUE
                    .checked_add(1)
                    .ok_or("object bound overflow")?;
                c.fields = Some(
                    vec![
                        CompiledInputField {
                            name: "child".into(),
                            kind: InputSlotKind::Any,
                            constraint: CompiledInputConstraint::required()
                        };
                        count
                    ]
                    .into_boxed_slice(),
                );
                c
            },
            InputConstraintIssue::SchemaCapacity,
        ),
    ] {
        let mut parts = parts()?;
        let input = parts.input_slots.first_mut().ok_or("input missing")?;
        input.kind = if issue == InputConstraintIssue::InvalidDefault {
            InputSlotKind::Bool
        } else {
            InputSlotKind::Object
        };
        input.constraint = schema;
        assert!(
            matches!(CompiledWorkflow::try_from_parts(parts), Err(WorkflowError::InconsistentInputConstraint { issue: actual, .. }) if actual == issue)
        );
    }
    Ok(())
}

#[test]
fn scalar_symbol_admission_requires_backing_payloads_even_with_a_declared_count() -> TestResult {
    let mut parts = parts()?;
    let input = parts.input_slots.first_mut().ok_or("input missing")?;
    input.kind = InputSlotKind::Symbol;
    let slot = input.slot;
    parts.symbols_count = 1;
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    assert!(matches!(
        vb_storage::inputs::prepare_scalar_inputs(
            &workflow,
            &[(slot, vb_core::SlotValue::Symbol(vb_core::SymbolId::new(0)))]
        ),
        Err(vb_storage::JournalError::InputSchemaMismatch)
    ));
    Ok(())
}
