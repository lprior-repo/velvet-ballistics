#![forbid(unsafe_code)]
#[path = "vb_f1_secret_references/boundaries.rs"]
mod boundaries;
#[path = "vb_f1_secret_references/owned.rs"]
mod owned;
#[path = "vb_f1_secret_references/recovery.rs"]
mod recovery;
#[path = "vb_f1_secret_references/support.rs"]
mod support;
use support::*;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, RunId, SlotValue, Taint, ValueStore,
};
use vb_runtime::RuntimeError;
use vb_runtime::journal::RuntimeJournalEvent;

#[test]
fn missing_host_requirement_rejects_every_public_submit_before_journal() -> TestResult {
    assert_eq!(
        RuntimeError::SecretUnavailable.symbolic_code().as_str(),
        "SECRET_UNAVAILABLE"
    );
    let workflow = workflow()?;
    let (runtime, journal) = runtime();
    let run = RunId::new(1);
    for result in [
        runtime.submit_compiled(run, workflow.clone()),
        runtime.submit_direct(run, workflow.clone()),
        runtime.submit_compiled_with_inputs(run, workflow.clone(), Box::new([])),
        runtime.submit_direct_with_grants_and_contracts(
            run,
            workflow.clone(),
            CapabilitySet::empty(),
            contracts()?,
        ),
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow.clone(),
            Box::new([]),
            CapabilitySet::empty(),
            contracts()?,
        ),
    ] {
        assert_eq!(result, Err(RuntimeError::SecretUnavailable));
    }
    assert_eq!(journal.snapshot()?, vec![]);
    Ok(())
}

#[test]
fn available_opaque_reference_reaches_nested_action_payload_with_secret_taint() -> TestResult {
    let workflow = workflow()?;
    let metadata = workflow.source_metadata().ok_or("metadata")?;
    let binding = *metadata.secret_requirements().first().ok_or("binding")?;
    assert_eq!(metadata.symbol(binding.alias()), Some("token"));
    assert_eq!(metadata.symbol(binding.requirement()), Some("TOKEN"));
    let root = binding.slot().ok_or("binding slot")?;
    let (mut runtime, journal) = runtime();
    runtime.set_available_secret_names(Box::new([Box::from("TOKEN")]));
    for number in [2, 3] {
        let run = RunId::new(number);
        submit(&mut runtime, run, workflow.clone())?;
        let state = runtime.inspect_run_state(run).map_err(|error| {
            format!("run {number}: {error:?}; journal={:?}", journal.snapshot())
        })?;
        assert_eq!(
            *state.frame.read_slot(root)?,
            SlotValue::Symbol(binding.requirement())
        );
        assert_eq!(state.frame.read_taint(root)?, Taint::Secret);
        let node = workflow
            .to_parts()
            .nodes
            .into_iter()
            .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
            .ok_or("Do")?;
        let CompiledNodeKind::Do { input, .. } = node.kind else {
            return Err("Do kind".into());
        };
        assert_eq!(state.frame.read_taint(input)?, Taint::Secret);
        let SlotValue::Object(object) = *state.frame.read_slot(input)? else {
            return Err("object".into());
        };
        assert_eq!(state.store.object(object)?.len(), 2);
        for field in state.store.object(object)? {
            assert_eq!(field.taint, Taint::Secret);
            match state.store.symbol(field.key)? {
                "auth" => assert_eq!(field.value, SlotValue::Symbol(binding.requirement())),
                "nested" => {
                    let SlotValue::Object(nested) = field.value else {
                        return Err("nested object".into());
                    };
                    let fields = state.store.object(nested)?;
                    assert_eq!(fields.len(), 1);
                    assert_eq!(
                        fields.first().ok_or("auth")?.value,
                        SlotValue::Symbol(binding.requirement())
                    );
                    assert_eq!(fields.first().ok_or("auth")?.taint, Taint::Secret);
                }
                _ => return Err("unexpected action input field".into()),
            }
        }
        assert_eq!(state.store.symbol(binding.requirement())?, "TOKEN");
        let ticket = runtime
            .lookup_pending_action_ticket(run, u64::from(node.id.get()))
            .ok_or("action ticket")?;
        let value = SlotValue::Symbol(binding.requirement());
        runtime.complete_action_with_output(
            ticket,
            vb_core::ActionOutputReady {
                output_slot: node.output.slot().ok_or("output")?,
                value,
                encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
                taint: Taint::Secret,
            },
        )?;
        runtime.tick_all()?;
    }
    let events = journal.snapshot()?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, RuntimeJournalEvent::RunFinished { .. }))
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, RuntimeJournalEvent::RunFailed { .. }))
            .count(),
        0
    );
    Ok(())
}

#[test]
fn secret_binding_survives_postcard_and_rejects_foreign_writers_and_defaults() -> TestResult {
    let workflow = workflow()?;
    let bytes = postcard::to_allocvec(&workflow.to_parts())?;
    let restored = CompiledWorkflow::try_from_parts(postcard::from_bytes(&bytes)?)?;
    assert_eq!(restored.to_parts(), workflow.to_parts());
    let binding = *workflow
        .source_metadata()
        .ok_or("metadata")?
        .secret_requirements()
        .first()
        .ok_or("binding")?;
    let slot = binding.slot().ok_or("slot")?;
    for change in 0..3 {
        let mut parts = workflow.to_parts();
        if change == 2 {
            parts.nodes.first_mut().ok_or("node")?.output =
                vb_core::workflow::NodeOutput::Slot(slot);
        } else {
            let input = parts
                .input_slots
                .iter_mut()
                .find(|input| input.slot == slot)
                .ok_or("input")?;
            if change == 0 {
                input.constraint.secret = false;
            } else {
                input.constraint.default_cold =
                    Some(vb_core::InputDefaultValue::Text("token".into()));
            }
        }
        assert_eq!(
            CompiledWorkflow::try_from_parts(parts).map(|_| ()),
            Err(vb_core::WorkflowError::SourceMetadata {
                detail: "invalid immutable secret input binding".into()
            })
        );
    }
    Ok(())
}

#[test]
fn scalar_and_owned_roots_cannot_substitute_secret_handles_or_downgrade_taint() -> TestResult {
    let workflow = workflow()?;
    let metadata = workflow.source_metadata().ok_or("metadata")?;
    let binding = *metadata.secret_requirements().first().ok_or("binding")?;
    let slot = binding.slot().ok_or("slot")?;
    let Err(vb_storage::JournalError::InputSchemaMismatch) =
        vb_storage::inputs::prepare_scalar_inputs(
            &workflow,
            &[(slot, SlotValue::Symbol(binding.alias()))],
        )
    else {
        return Err("forged scalar reference accepted or wrong rejection".into());
    };
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
    let bundle =
        vb_core::root_bundle::OwnedRootBundle::new(Box::new([]), store.checkpoint_v1(), &workflow)?;
    assert_eq!(
        bundle.roots(),
        &[(
            slot,
            SlotValue::Symbol(binding.requirement()),
            Taint::Secret
        )]
    );
    for (value, taint) in [
        (SlotValue::Symbol(binding.alias()), Taint::Secret),
        (SlotValue::Symbol(binding.requirement()), Taint::Clean),
    ] {
        let result = vb_core::root_bundle::OwnedRootBundle::new(
            Box::new([(slot, value, taint)]),
            store.checkpoint_v1(),
            &workflow,
        );
        let Err(vb_core::root_bundle::OwnedRootBundleError::Checkpoint(error)) = result else {
            return Err("forged owned reference accepted or wrong rejection".into());
        };
        assert_eq!(
            error,
            vb_core::CoreError::InternalInvariantViolation {
                reason: "secret_input_binding"
            }
        );
    }
    Ok(())
}

#[test]
fn availability_is_requirement_name_presence_and_defaults_to_empty_on_restart() -> TestResult {
    let workflow = workflow()?;
    let (mut first, _) = runtime();
    first.set_available_secret_names(Box::new([Box::from("token")]));
    assert_eq!(
        first.submit_compiled(RunId::new(4), workflow.clone()),
        Err(RuntimeError::SecretUnavailable)
    );
    first.set_available_secret_names(Box::new([Box::from("TOKEN")]));
    submit(&mut first, RunId::new(4), workflow.clone())?;
    let (second, journal) = runtime();
    let restored = CompiledWorkflow::try_from_parts(postcard::from_bytes(
        &postcard::to_allocvec(&workflow.to_parts())?,
    )?)?;
    assert_eq!(
        second.submit_compiled(RunId::new(5), restored),
        Err(RuntimeError::SecretUnavailable)
    );
    assert_eq!(journal.snapshot()?, vec![]);
    Ok(())
}
