#![forbid(unsafe_code)]

use vb_core::input_validation::InputValueIssue;
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::{
    CompiledWorkflow, InputDefaultValue, ObjectField, ObjectId, SlotIdx, SlotValue, Taint,
    ValueStore,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn source(schema: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: recursive_schema\nwhen: {{manual: {{}}}}\ninputs:\n  value: {schema}\nsteps:\n  - id: seed\n    set: {{output: answer, value: '1'}}\n  - id: done\n    finish: {{result: answer}}\n"
    )
}

fn workflow(schema: &str) -> TestResult<CompiledWorkflow> {
    Ok(vb_compile::compile_workflow(source(schema).as_bytes())?)
}

fn slot(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    Ok(workflow
        .input_slots()
        .first()
        .ok_or("missing declaration")?
        .slot)
}

fn bundle(
    workflow: &CompiledWorkflow,
    store: &ValueStore,
    value: Option<SlotValue>,
) -> TestResult<OwnedRootBundle> {
    let roots = value.map(|value| (slot(workflow), value));
    let roots = match roots {
        Some((slot, value)) => vec![(slot?, value, Taint::Clean)],
        None => vec![],
    };
    Ok(OwnedRootBundle::new(
        roots.into_boxed_slice(),
        store.checkpoint_v1(),
        workflow,
    )?)
}

fn object(value: SlotValue) -> TestResult<ObjectId> {
    match value {
        SlotValue::Object(id) => Ok(id),
        _ => Err("expected object".into()),
    }
}

fn field(bundle: &OwnedRootBundle, id: ObjectId, name: &str) -> TestResult<(SlotValue, Taint)> {
    let checkpoint = bundle.checkpoint();
    let fields = checkpoint
        .objects
        .get(usize::try_from(id.get())?)
        .ok_or("object payload")?;
    for field in fields.iter() {
        if checkpoint
            .symbols
            .get(usize::try_from(field.key.get())?)
            .is_some_and(|key| key.as_ref() == name)
        {
            return Ok((field.value, field.taint));
        }
    }
    Err(format!("missing field {name}").into())
}

#[test]
fn recursive_schema_roundtrip_retains_field_kinds_and_defaults() -> TestResult {
    let workflow = workflow(
        "{is: object, fields: {profile: {is: object, fields: {name: {is: text, max_length: 9, default: guest}, flags: list<boolean>}}, enabled: {is: boolean, optional: true}}, extra: reject}",
    )?;
    let declaration = workflow.input_slots().first().ok_or("declaration")?;
    let fields = declaration.constraint.fields.as_deref().ok_or("fields")?;
    let profile = fields.first().ok_or("profile")?;
    assert_eq!(profile.kind, vb_core::InputSlotKind::Object);
    let children = profile.constraint.fields.as_deref().ok_or("children")?;
    assert_eq!(
        children.first().ok_or("name")?.kind,
        vb_core::InputSlotKind::Symbol
    );
    assert_eq!(
        children.first().ok_or("name")?.constraint.default_cold,
        Some(InputDefaultValue::Text("guest".into()))
    );
    assert_eq!(
        children.get(1).ok_or("flags")?.constraint.element,
        Some(vb_core::InputSlotKind::Bool)
    );
    let parts = workflow.to_parts();
    let bytes = postcard::to_allocvec(&parts)?;
    assert_eq!(
        postcard::from_bytes::<vb_core::WorkflowParts>(&bytes)?,
        parts
    );
    Ok(())
}

#[test]
fn owned_partial_objects_receive_recursive_defaults_and_preserve_overrides() -> TestResult {
    let workflow = workflow(
        "{is: object, extra: reject, fields: {profile: {is: object, default: {}, extra: reject, fields: {name: {is: text, default: guest}, enabled: {is: boolean, default: true}}}, count: {is: number, default: 3, min: 2}, tags: {is: list, of: text, default: [red, blue]}}}",
    )?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let count = store.insert_symbol("count")?;
    let root = store.insert_object(Box::from([ObjectField::clean(count, SlotValue::I64(8))]))?;
    let bundle = bundle(&workflow, &store, Some(SlotValue::Object(root)))?;
    assert_eq!(field(&bundle, root, "count")?.0, SlotValue::I64(8));
    let profile = object(field(&bundle, root, "profile")?.0)?;
    assert_eq!(field(&bundle, profile, "enabled")?.0, SlotValue::Bool(true));
    let SlotValue::Symbol(name) = field(&bundle, profile, "name")?.0 else {
        return Err("name".into());
    };
    assert_eq!(
        bundle
            .checkpoint()
            .symbols
            .get(usize::try_from(name.get())?)
            .map(Box::as_ref),
        Some("guest")
    );
    let SlotValue::List(tags) = field(&bundle, root, "tags")?.0 else {
        return Err("tags".into());
    };
    assert_eq!(
        bundle
            .checkpoint()
            .lists
            .get(usize::try_from(tags.get())?)
            .map(|items| items.len()),
        Some(2)
    );
    assert_eq!(bundle.checkpoint().objects.len(), 2);
    Ok(())
}

#[test]
fn recursive_fields_enforce_kind_bounds_required_and_closed_objects() -> TestResult {
    for (inner, expected) in [
        (Some(("enabled", SlotValue::I64(1))), InputValueIssue::Kind),
        (None, InputValueIssue::RequiredField),
        (
            Some(("unexpected", SlotValue::Bool(true))),
            InputValueIssue::RequiredField,
        ),
    ] {
        let workflow = workflow(
            "{is: object, fields: {profile: {is: object, extra: reject, fields: {enabled: boolean}}}}",
        )?;
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let mut fields = vec![];
        if let Some((name, value)) = inner {
            fields.push(ObjectField::clean(store.insert_symbol(name)?, value));
        }
        let inner = store.insert_object(fields.into_boxed_slice())?;
        let key = store.insert_symbol("profile")?;
        let root = store.insert_object(Box::from([ObjectField::clean(
            key,
            SlotValue::Object(inner),
        )]))?;
        assert!(
            matches!(OwnedRootBundle::new(Box::from([(slot(&workflow)?, SlotValue::Object(root), Taint::Clean)]), store.checkpoint_v1(), &workflow), Err(OwnedRootBundleError::RootConstraintMismatch { issue, .. }) if issue == expected)
        );
    }
    Ok(())
}

#[test]
fn defaults_must_satisfy_every_nested_constraint() {
    for schema in [
        "{is: text, max_length: 1, default: é}",
        "{is: list, of: number, default: [true]}",
        "{is: list, of: number, min: 2, default: [1]}",
        "{is: object, fields: {count: {is: number, min: 2}}, default: {count: 1}}",
        "{is: object, fields: {count: number}, default: {}}",
        "{is: object, fields: {}, extra: reject, default: {count: 1}}",
        "{is: object, fields: {count: {is: number, default: wrong}}}",
        "{is: boolean, default: null}",
    ] {
        let result = vb_compile::compile_workflow(source(schema).as_bytes());
        assert!(
            matches!(&result, Err(errors) if matches!(errors.0.as_slice(), [vb_compile::CompileError::InvalidInputSchema { field: "inputs.default", .. }])),
            "{schema}: {result:?}"
        );
    }
}

#[test]
fn text_list_object_and_any_defaults_materialize_without_foreign_handles() -> TestResult {
    for (schema, expected) in [
        (
            "{is: text, default: literal}",
            vb_core::InputSlotKind::Symbol,
        ),
        (
            "{is: list, of: text, default: [one, two]}",
            vb_core::InputSlotKind::List,
        ),
        (
            "{is: object, default: {enabled: true}}",
            vb_core::InputSlotKind::Object,
        ),
        (
            "{is: any, default: [{enabled: [true]}]}",
            vb_core::InputSlotKind::List,
        ),
        (
            "{is: text, optional: true, default: literal}",
            vb_core::InputSlotKind::Symbol,
        ),
    ] {
        let workflow = workflow(schema)?;
        let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let bundle = bundle(&workflow, &store, None)?;
        let value = bundle.root(slot(&workflow)?).ok_or("default root")?.0;
        assert_eq!(vb_core::input_validation::input_value_kind(value), expected);
        ValueStore::from_checkpoint_v1(bundle.checkpoint().clone())?;
    }
    Ok(())
}

#[test]
fn scalar_wires_reject_omitted_payload_defaults_including_optional_defaults() -> TestResult {
    for schema in [
        "{is: text, default: value}",
        "{is: text, optional: true, default: value}",
        "{is: list, of: number, default: [1]}",
    ] {
        let workflow = workflow(schema)?;
        assert!(matches!(
            vb_storage::RunInputsRecord::new(
                vb_core::RunId::new(1),
                vb_core::WorkflowDigest::from_bytes([1; 32]),
                vb_core::WorkflowDigest::from_bytes([2; 32]),
                &workflow,
                Box::from([])
            ),
            Err(vb_storage::JournalError::InputSchemaMismatch)
        ));
    }
    Ok(())
}

#[test]
fn explicit_null_is_preserved_and_nested_default_secrecy_propagates() -> TestResult {
    let workflow = workflow(
        "{is: object, default: {}, fields: {name: {is: text, default: private, secret: true}, enabled: {is: boolean, nullable: true, default: true}}}",
    )?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let key = store.insert_symbol("enabled")?;
    let root = store.insert_object(Box::from([ObjectField::clean(key, SlotValue::Null)]))?;
    let bundle = bundle(&workflow, &store, Some(SlotValue::Object(root)))?;
    assert_eq!(field(&bundle, root, "enabled")?.0, SlotValue::Null);
    assert_eq!(field(&bundle, root, "name")?.1, Taint::Secret);
    assert_eq!(
        bundle.root(slot(&workflow)?).map(|(_, taint)| taint),
        Some(Taint::Secret)
    );
    Ok(())
}

#[test]
fn materialization_enforces_arena_caps() -> TestResult {
    let workflow = workflow("{is: object, default: {name: text}}")?;
    let mut parts = workflow.to_parts();
    parts.resource_contract.max_slots = parts.slot_count;
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    assert!(matches!(
        OwnedRootBundle::new(Box::from([]), store.checkpoint_v1(), &workflow),
        Err(OwnedRootBundleError::Checkpoint(
            vb_core::CoreError::ResourceLimitExceeded { .. }
        ))
    ));
    Ok(())
}

#[test]
fn recursive_validator_rejects_cycles_and_hidden_dangling_payloads() -> TestResult {
    let workflow = workflow("any")?;
    let declaration = workflow.input_slots().first().ok_or("declaration")?;
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let mut checkpoint = store.checkpoint_v1();
    checkpoint
        .lists
        .push(Box::from([SlotValue::List(vb_core::ListId::new(0))]));
    assert_eq!(
        vb_core::input_validation::validate_input_value(
            declaration,
            SlotValue::List(vb_core::ListId::new(0)),
            0,
            Some(&checkpoint)
        ),
        Err(InputValueIssue::DepthExceeded)
    );
    *checkpoint.lists.first_mut().ok_or("list")? =
        Box::from([SlotValue::Symbol(vb_core::SymbolId::new(100))]);
    assert_eq!(
        vb_core::input_validation::validate_input_value(
            declaration,
            SlotValue::List(vb_core::ListId::new(0)),
            0,
            Some(&checkpoint)
        ),
        Err(InputValueIssue::Payload)
    );
    Ok(())
}

#[test]
fn nested_values_enforce_bounds_and_extra_fields_after_materialization() -> TestResult {
    for (schema, fields, expected) in [
        (
            "{is: object, fields: {count: {is: number, min: 2}}}",
            vec![("count", SlotValue::I64(1))],
            InputValueIssue::Bounds,
        ),
        (
            "{is: object, extra: reject, fields: {count: number}}",
            vec![
                ("count", SlotValue::I64(2)),
                ("extra", SlotValue::Bool(true)),
            ],
            InputValueIssue::ExtraField,
        ),
    ] {
        let workflow = workflow(schema)?;
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let fields = fields
            .into_iter()
            .map(|(name, value)| {
                store
                    .insert_symbol(name)
                    .map(|key| ObjectField::clean(key, value))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let root = store.insert_object(fields.into_boxed_slice())?;
        assert!(matches!(
            OwnedRootBundle::new(Box::from([(slot(&workflow)?, SlotValue::Object(root), Taint::Clean)]), store.checkpoint_v1(), &workflow),
            Err(OwnedRootBundleError::RootConstraintMismatch { issue, .. }) if issue == expected
        ));
    }
    Ok(())
}

#[test]
fn secret_schema_preserves_stronger_supplied_taints() -> TestResult {
    let workflow = workflow(
        "{is: object, secret: true, fields: {count: {is: number, default: 2, secret: true}}}",
    )?;
    for taint in [Taint::Random, Taint::TimeDependent] {
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let root = store.insert_object(Box::from([]))?;
        let bundle = OwnedRootBundle::new(
            Box::from([(slot(&workflow)?, SlotValue::Object(root), taint)]),
            store.checkpoint_v1(),
            &workflow,
        )?;
        assert_eq!(
            bundle.root(slot(&workflow)?).map(|(_, taint)| taint),
            Some(taint)
        );
        assert_eq!(field(&bundle, root, "count")?.1, taint);
    }
    Ok(())
}
