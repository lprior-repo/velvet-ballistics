use vb_core::root_bundle::OwnedRootBundle;
use vb_core::workflow::source_metadata::{WorkflowSourceMetadata, seed_source_symbols};
use vb_core::{
    CompiledWorkflow, ObjectField, RunId, SlotValue, StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workflow() -> Result<CompiledWorkflow, vb_compile::CompileErrors> {
    vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: metadata\nwhen: {manual: {}}\ninputs: {payload: any}\nvars: {marker: compiled}\nsteps:\n  - id: echo\n    set: {output: echoed, value: '$input.payload'}\nresult: {value: '$echoed', marker: '$vars.marker'}\n")
}

#[test]
fn source_payload_decode_revalidates_private_invariants() -> TestResult {
    #[derive(serde::Serialize)]
    struct Wire {
        symbols: Box<[Box<str>]>,
        requirements: Box<[vb_core::workflow::source_metadata::CompiledSecretRequirement]>,
    }
    let invalid = Wire {
        symbols: Box::from([Box::from("duplicate"), Box::from("duplicate")]),
        requirements: Box::new([]),
    };
    assert_eq!(
        postcard::from_bytes::<WorkflowSourceMetadata>(&postcard::to_allocvec(&invalid)?),
        Err(postcard::Error::SerdeDeCustom)
    );
    let workflow = workflow()?;
    let metadata = workflow.source_metadata().ok_or("missing metadata")?;
    assert_eq!(
        postcard::from_bytes::<WorkflowSourceMetadata>(&postcard::to_allocvec(metadata)?)?,
        *metadata
    );
    let mut parts = workflow.to_parts();
    let actual = parts.symbols_count;
    parts.symbols_count = actual.checked_add(1).ok_or("count overflow")?;
    let declared = parts.symbols_count;
    let expected = vb_core::workflow::source_metadata::SourceMetadataError::SymbolCountMismatch {
        declared,
        actual,
    }
    .to_string();
    assert!(
        matches!(CompiledWorkflow::try_from_parts(parts), Err(vb_core::workflow::WorkflowError::SourceMetadata { detail }) if detail.as_ref() == expected)
    );
    Ok(())
}

#[test]
fn source_prefix_is_idempotent_and_rejects_conflicting_existing_arenas() -> TestResult {
    let workflow = workflow()?;
    let metadata = workflow.source_metadata().ok_or("missing metadata")?;
    let mut store = ValueStore::new();
    seed_source_symbols(metadata, &mut store)?;
    let checkpoint = store.checkpoint_v1();
    seed_source_symbols(metadata, &mut store)?;
    assert_eq!(store.checkpoint_v1(), checkpoint);
    let mut wrong = ValueStore::new();
    wrong.insert_symbol("wrong")?;
    assert!(matches!(
        seed_source_symbols(metadata, &mut wrong),
        Err(vb_core::CoreError::InternalInvariantViolation {
            reason: vb_core::InvariantKind::Unknown("compiled_source_symbol_prefix")
        })
    ));
    Ok(())
}

#[test]
fn owned_symbol_relocation_preserves_nested_values_and_static_payloads() -> TestResult {
    let workflow = workflow()?;
    let slot = workflow.input_slots().first().ok_or("missing input")?.slot;
    let mut caller = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let value = caller.insert_symbol("caller")?;
    let key = caller.insert_symbol("key")?;
    let object = caller.insert_object(Box::from([ObjectField::clean(
        key,
        SlotValue::Symbol(value),
    )]))?;
    let bundle = OwnedRootBundle::new(
        Box::from([(slot, SlotValue::Object(object), Taint::Clean)]),
        caller.checkpoint_v1(),
        &workflow,
    )?;
    let mut store = ValueStore::from_checkpoint_v1(bundle.checkpoint().clone())?;
    let mut frame = new_run_frame(RunId::new(1), &workflow)?;
    for (slot, value, taint) in bundle.roots() {
        frame.write_slot_with_taint(*slot, *value, *taint)?;
    }
    let fields = store.object(object)?;
    let field = fields.first().ok_or("missing nested field")?;
    assert_eq!(store.symbol(field.key)?, "key");
    let SlotValue::Symbol(value) = field.value else {
        return Err("wrong nested value".into());
    };
    assert_eq!(store.symbol(value)?, "caller");
    let mut evidence = EvidenceCollector::new();
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(100),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut CollectStates::new(),
        &vb_core::CapabilitySet::empty(),
        1,
    )?;
    let RuntimeSignal::Finished(SlotValue::Object(result)) = signal else {
        return Err(format!("{signal:?}").into());
    };
    for field in store.object(result)? {
        match store.symbol(field.key)? {
            "value" => assert_eq!(field.value, SlotValue::Object(object)),
            "marker" => {
                let SlotValue::Symbol(id) = field.value else {
                    return Err("missing marker".into());
                };
                assert_eq!(store.symbol(id)?, "compiled");
            }
            other => return Err(format!("unexpected field {other}").into()),
        }
    }
    let mut checkpoint = store.checkpoint_v1();
    if let Some(text) = checkpoint.symbols.first_mut() {
        *text = "corrupt".into();
    }
    assert!(matches!(
        vb_core::root_bundle::validate_runtime_store(&frame, &checkpoint, &workflow),
        Err(vb_core::root_bundle::OwnedRootBundleError::Checkpoint(
            vb_core::CoreError::InternalInvariantViolation {
                reason: vb_core::InvariantKind::Unknown("compiled_source_symbol_prefix")
            }
        ))
    ));
    Ok(())
}
