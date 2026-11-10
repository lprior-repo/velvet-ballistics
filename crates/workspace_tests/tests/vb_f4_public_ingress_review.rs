#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{
    CapabilitySet, CompiledWorkflow, RunId, RuntimePolicy, SlotValue, Taint, ValueStore,
    WorkflowDigest,
};
use vb_runtime::{
    RuntimeError, journal::VolatileRuntimeJournal, runtime::Runtime, shard::ShardConfig,
};
use vb_storage::{FjallJournal, JournalError, RunInputsRecord, RunSubmission};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn source(schema: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: ingress_review\nwhen: {{manual: {{}}}}\ninputs:\n  value: {schema}\nsteps:\n  - id: first\n    set: {{output: answer, value: '42'}}\n  - id: done\n    finish: {{result: answer}}\n"
    )
}

fn workflow(schema: &str) -> TestResult<CompiledWorkflow> {
    Ok(vb_compile::compile_workflow(source(schema).as_bytes())?)
}

fn runtime() -> (Runtime, Arc<VolatileRuntimeJournal>) {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let config = ShardConfig {
        policy: RuntimePolicy::Relaxed,
        command_queue_capacity: 1,
        step_budget_per_tick: 1,
        ..ShardConfig::default()
    };
    (
        Runtime::new(NonZeroUsize::MIN, config, journal.clone()),
        journal,
    )
}

struct Resolver(CompiledWorkflow);

impl vb_ipc::server::WorkflowResolver for Resolver {
    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<CompiledWorkflow, vb_ipc::server::WorkflowResolutionError> {
        if digest == self.0.digest() {
            Ok(self.0.clone())
        } else {
            Err(vb_ipc::server::WorkflowResolutionError::NotFound)
        }
    }
}

fn ipc_submit(
    runtime: &mut Runtime,
    workflow: &CompiledWorkflow,
    input: Vec<u8>,
) -> vb_ipc::server::IpcResponse {
    vb_ipc::server::handlers::runs::submit_resolved_workflow(
        vb_ipc::IpcCommand::SubmitRunInline,
        vb_ipc::SubmitRunPayload {
            run_id: RunId::new(702),
            workflow: workflow.digest(),
            input,
        },
        runtime,
        Some(&mut Resolver(workflow.clone())),
    )
}

fn invalid_ipc() -> vb_ipc::server::IpcResponse {
    let error = vb_ipc::IpcError::PayloadDecodeFailed;
    vb_ipc::server::IpcResponse::PayloadError {
        diagnostic: error.diagnostic_code().code(),
        message: error.to_string(),
    }
}

#[test]
fn every_direct_runtime_submit_checks_inputs_before_using_queue_capacity() -> TestResult {
    let workflow = workflow("{is: number, min: 3, max: 5}")?;
    let slot = workflow.input_slots().first().ok_or("input")?.slot;
    for method in 0..8 {
        let (mut runtime, journal) = runtime();
        let run = RunId::new(702);
        let invalid = Box::from([(slot, SlotValue::I64(2))]);
        let result = match method {
            0 => runtime.submit_direct(run, workflow.clone()),
            1 => runtime.submit_direct_with_grants(run, workflow.clone(), CapabilitySet::empty()),
            2 => runtime.submit_compiled(run, workflow.clone()),
            3 => runtime.submit_compiled_with_grants(run, workflow.clone(), CapabilitySet::empty()),
            4 => runtime.submit_compiled_with_inputs(run, workflow.clone(), invalid),
            5 => runtime.submit_compiled_with_inputs_and_grants(
                run,
                workflow.clone(),
                invalid,
                CapabilitySet::empty(),
            ),
            6 => runtime.submit_direct_with_inputs_grants_and_contracts(
                run,
                workflow.clone(),
                invalid,
                CapabilitySet::empty(),
                Box::from([]),
            ),
            _ => runtime.submit_direct_with_grants_and_contracts(
                run,
                workflow.clone(),
                CapabilitySet::empty(),
                Box::from([]),
            ),
        };
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "input_schema_mismatch"
            })
        );
        assert_eq!(
            journal.snapshot()?,
            Vec::<vb_runtime::journal::RuntimeJournalEvent>::new()
        );
        runtime.tick_all()?;
        assert!(matches!(
            runtime.inspect_run_state(run),
            Err(RuntimeError::RunNotFound)
        ));
        assert_eq!(
            journal.snapshot()?,
            Vec::<vb_runtime::journal::RuntimeJournalEvent>::new()
        );
        // The sole queue slot must still accept a real caller's valid input.
        runtime.submit_compiled_with_inputs(
            run,
            workflow.clone(),
            Box::from([(slot, SlotValue::I64(3))]),
        )?;
        runtime.tick_all()?;
        assert_eq!(
            runtime.inspect_run_state(run)?.frame.read_slot(slot)?,
            &SlotValue::I64(3)
        );
    }
    Ok(())
}

#[test]
fn scalar_nullability_defaults_secrecy_and_bounds_reach_runtime_and_ipc_frames() -> TestResult {
    for (schema, supplied, expected, taint) in [
        (
            "{is: number, nullable: true}",
            Some(SlotValue::Null),
            SlotValue::Null,
            Taint::Clean,
        ),
        (
            "{is: number, default: 4, secret: true}",
            None,
            SlotValue::I64(4),
            Taint::Secret,
        ),
        (
            "{is: number, default: 4, secret: true}",
            Some(SlotValue::I64(5)),
            SlotValue::I64(5),
            Taint::Secret,
        ),
        (
            "{is: number, optional: true, default: 4, secret: true}",
            None,
            SlotValue::I64(4),
            Taint::Secret,
        ),
        (
            "{is: number, min: 3, max: 5}",
            Some(SlotValue::I64(3)),
            SlotValue::I64(3),
            Taint::Clean,
        ),
        (
            "{is: number, min: 3, max: 5}",
            Some(SlotValue::I64(5)),
            SlotValue::I64(5),
            Taint::Clean,
        ),
    ] {
        let workflow = workflow(schema)?;
        let slot = workflow.input_slots().first().ok_or("input")?.slot;
        for ipc in [false, true] {
            let (mut runtime, journal) = runtime();
            if ipc {
                let values = supplied.into_iter().collect::<Vec<_>>();
                assert_eq!(
                    ipc_submit(&mut runtime, &workflow, postcard::to_allocvec(&values)?),
                    vb_ipc::server::IpcResponse::AcceptedRun { run_id: 702 }
                );
            } else {
                runtime.submit_compiled_with_inputs(
                    RunId::new(702),
                    workflow.clone(),
                    supplied
                        .into_iter()
                        .map(|value| (slot, value))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )?;
            }
            assert_eq!(
                journal.snapshot()?,
                Vec::<vb_runtime::journal::RuntimeJournalEvent>::new()
            );
            runtime.tick_all()?;
            let state = runtime.inspect_run_state(RunId::new(702))?;
            assert_eq!(state.frame.read_slot(slot)?, &expected);
            assert_eq!(state.frame.read_taint(slot)?, taint);
        }
    }
    Ok(())
}

#[test]
fn malformed_ipc_inputs_leave_no_journal_state_or_queued_command() -> TestResult {
    let workflow = workflow("{is: number, min: 3, max: 5}")?;
    let slot = workflow.input_slots().first().ok_or("input")?.slot;
    let mut trailing = postcard::to_allocvec(&vec![SlotValue::I64(3)])?;
    trailing.push(0);
    for bytes in [
        Vec::new(),
        postcard::to_allocvec(&vec![SlotValue::I64(2)])?,
        postcard::to_allocvec(&vec![SlotValue::Bool(true)])?,
        postcard::to_allocvec(&vec![SlotValue::Null])?,
        postcard::to_allocvec(&usize::MAX)?,
        postcard::to_allocvec(&vec![SlotValue::I64(3), SlotValue::I64(3)])?,
        trailing,
    ] {
        let (mut runtime, journal) = runtime();
        assert_eq!(ipc_submit(&mut runtime, &workflow, bytes), invalid_ipc());
        assert_eq!(
            journal.snapshot()?,
            Vec::<vb_runtime::journal::RuntimeJournalEvent>::new()
        );
        runtime.tick_all()?;
        assert!(matches!(
            runtime.inspect_run_state(RunId::new(702)),
            Err(RuntimeError::RunNotFound)
        ));
        assert_eq!(
            journal.snapshot()?,
            Vec::<vb_runtime::journal::RuntimeJournalEvent>::new()
        );
        runtime.submit_compiled_with_inputs(
            RunId::new(702),
            workflow.clone(),
            Box::from([(slot, SlotValue::I64(3))]),
        )?;
        runtime.tick_all()?;
        assert_eq!(
            runtime
                .inspect_run_state(RunId::new(702))?
                .frame
                .read_slot(slot)?,
            &SlotValue::I64(3)
        );
    }
    Ok(())
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(32))]
    #[test]
    fn arbitrary_handle_and_declared_symbol_counts_cannot_admit_absent_payloads(count in proptest::prelude::any::<u32>(), handle in proptest::prelude::any::<u32>()) {
        let result = (|| -> TestResult {
            let mut parts = workflow("any")?.to_parts();
            parts.source_metadata = None;
            parts.symbols_count = count;
            let workflow = CompiledWorkflow::try_from_parts(parts)?;
            let slot = workflow.input_slots().first().ok_or("input")?.slot;
            let (mut runtime, journal) = runtime();
            for value in [SlotValue::Symbol(vb_core::SymbolId::new(handle)), SlotValue::Symbol(vb_core::SymbolId::new(count.saturating_sub(1))), SlotValue::List(vb_core::ListId::new(handle)), SlotValue::Object(vb_core::ObjectId::new(handle)), SlotValue::Blob(vb_core::BlobId::new(u64::from(handle)))] {
                assert!(matches!(RunInputsRecord::new(RunId::new(702), workflow.digest(), workflow.digest(), &workflow, Box::from([(slot, value)])), Err(JournalError::InputSchemaMismatch)));
                assert_eq!(runtime.submit_compiled_with_inputs(RunId::new(702), workflow.clone(), Box::from([(slot, value)])), Err(RuntimeError::UnsupportedOperation { operation: "input_schema_mismatch" }));
                assert_eq!(ipc_submit(&mut runtime, &workflow, postcard::to_allocvec(&vec![value])?), invalid_ipc());
            }
            assert_eq!(journal.snapshot()?, Vec::<vb_runtime::journal::RuntimeJournalEvent>::new());
            runtime.submit_compiled_with_inputs(RunId::new(702), workflow, Box::from([(slot, SlotValue::I64(3))]))?;
            runtime.tick_all()?;
            assert_eq!(runtime.inspect_run_state(RunId::new(702))?.frame.read_slot(slot)?, &SlotValue::I64(3));
            Ok(())
        })();
        proptest::prop_assert_eq!(result.map_err(|error| error.to_string()), Ok(()));
    }
}

#[test]
fn source_backed_symbols_use_actual_utf8_payload_bounds_at_every_scalar_ingress() -> TestResult {
    for (limit, accepted) in [(3, false), (4, true)] {
        let text = source(&format!("{{is: text, max_length: {limit}}}"))
            .replace("value: '42'", "value: '\"éé\"'");
        let workflow = vb_compile::compile_workflow(text.as_bytes())?;
        let slot = workflow.input_slots().first().ok_or("input")?.slot;
        let symbols = workflow
            .source_metadata()
            .ok_or("source metadata")?
            .symbols();
        let raw = symbols
            .iter()
            .position(|value| value.as_ref() == "éé")
            .ok_or("source text")?;
        let value = SlotValue::Symbol(vb_core::SymbolId::new(u32::try_from(raw)?));
        let retained = RunInputsRecord::new(
            RunId::new(702),
            workflow.digest(),
            workflow.digest(),
            &workflow,
            Box::from([(slot, value)]),
        );
        let (mut direct, direct_journal) = runtime();
        let (mut ipc, ipc_journal) = runtime();
        let submitted = direct.submit_compiled_with_inputs(
            RunId::new(702),
            workflow.clone(),
            Box::from([(slot, value)]),
        );
        let response = ipc_submit(&mut ipc, &workflow, postcard::to_allocvec(&vec![value])?);
        if accepted {
            assert_eq!(retained?.inputs(), &[(slot, value)]);
            submitted?;
            assert_eq!(
                response,
                vb_ipc::server::IpcResponse::AcceptedRun { run_id: 702 }
            );
            for runtime in [&mut direct, &mut ipc] {
                runtime.tick_all()?;
                let state = runtime.inspect_run_state(RunId::new(702))?;
                assert_eq!(state.frame.read_slot(slot)?, &value);
                assert_eq!(
                    state
                        .store
                        .symbol(vb_core::SymbolId::new(u32::try_from(raw)?))?,
                    "éé"
                );
            }
        } else {
            assert!(matches!(retained, Err(JournalError::InputSchemaMismatch)));
            assert_eq!(
                submitted,
                Err(RuntimeError::UnsupportedOperation {
                    operation: "input_schema_mismatch"
                })
            );
            assert_eq!(response, invalid_ipc());
            direct.tick_all()?;
            ipc.tick_all()?;
            assert_eq!(
                direct_journal.snapshot()?,
                Vec::<vb_runtime::journal::RuntimeJournalEvent>::new()
            );
            assert_eq!(
                ipc_journal.snapshot()?,
                Vec::<vb_runtime::journal::RuntimeJournalEvent>::new()
            );
        }
    }
    Ok(())
}

#[test]
fn recursive_owned_defaults_nulls_and_secret_payloads_survive_retention_and_runtime_activation()
-> TestResult {
    let schema = "{is: object, extra: reject, fields: {profile: {is: object, default: {}, extra: reject, fields: {name: {is: text, default: private, max_length: 7, secret: true}}}, enabled: {is: boolean, default: true, nullable: true}, labels: {is: list, of: text, default: [red, blue]}}}";
    let workflow = workflow(schema)?;
    let slot = workflow.input_slots().first().ok_or("input")?.slot;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let key = store.insert_symbol("enabled")?;
    let root = store.insert_object(Box::from([vb_core::ObjectField::clean(
        key,
        SlotValue::Null,
    )]))?;
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(
        Box::from([(slot, SlotValue::Object(root), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    assert_eq!(
        bundle.root(slot),
        Some((SlotValue::Object(root), Taint::Secret))
    );
    let revalidated = vb_core::root_bundle::OwnedRootBundle::new(
        bundle.roots().to_vec().into_boxed_slice(),
        bundle.checkpoint().clone(),
        &workflow,
    )?;
    assert_eq!(revalidated.roots(), bundle.roots());
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(source(schema).as_bytes()).into()),
        source: source(schema).into_bytes(),
    };
    let run = RunId::new(703);
    let record = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source.digest,
        &bundle,
        &workflow,
    )?;
    let input_digest = record.input_digest();
    let restored = record.restore(&workflow)?;
    assert_eq!(restored.roots(), bundle.roots());
    assert_eq!(restored.checkpoint(), bundle.checkpoint());
    let retained_again = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source.digest,
        &restored,
        &workflow,
    )?;
    let restored_again = retained_again.restore(&workflow)?;
    assert_eq!(retained_again.input_digest(), input_digest);
    assert_eq!(restored_again.roots(), bundle.roots());
    assert_eq!(restored_again.checkpoint(), bundle.checkpoint());
    reject_rehashed_noncanonical_checkpoint(&record, &workflow)?;
    let submission = RunSubmission::owned(
        source.clone(),
        artifact.clone(),
        record,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    assert_eq!(revalidated.checkpoint(), bundle.checkpoint());
    let directory = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    }
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let retained = journal
        .owned_run_inputs(run, artifact.digest, source.digest, &workflow)?
        .ok_or("retained input")?;
    assert_eq!(retained.input_digest(), input_digest);
    assert_eq!(retained.checkpoint(), bundle.checkpoint());
    let before = journal.events_for_run_full(run)?;
    let config = ShardConfig {
        command_queue_capacity: 1,
        step_budget_per_tick: 1,
        ..ShardConfig::default()
    };
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        config,
        vb_runtime::journal::StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    runtime.activate_owned_submission(run, source.digest)?;
    assert_eq!(journal.events_for_run_full(run)?, before);
    runtime.tick_all()?;
    let state = runtime.inspect_run_state(run)?;
    assert_eq!(state.frame.read_slot(slot)?, &SlotValue::Object(root));
    assert_eq!(state.frame.read_taint(slot)?, Taint::Secret);
    assert_eq!(state.store.checkpoint_v1(), *bundle.checkpoint());
    let fields = state.store.object(root)?;
    let enabled = fields
        .iter()
        .find(|field| state.store.symbol(field.key) == Ok("enabled"))
        .ok_or("enabled field")?;
    assert_eq!(enabled.value, SlotValue::Null);
    let profile = fields
        .iter()
        .find(|field| state.store.symbol(field.key) == Ok("profile"))
        .ok_or("profile field")?;
    let SlotValue::Object(profile) = profile.value else {
        return Err("profile object".into());
    };
    let name = state.store.object(profile)?.first().ok_or("name field")?;
    let SlotValue::Symbol(value) = name.value else {
        return Err("name text".into());
    };
    assert_eq!(state.store.symbol(value)?, "private");
    assert_eq!(name.taint, Taint::Secret);
    Ok(())
}

#[test]
fn retained_scalar_defaults_cannot_disappear_before_atomic_submission() -> TestResult {
    reject_omitted_retained_default(false)
}

#[test]
fn retained_optional_scalar_defaults_cannot_disappear_before_atomic_submission() -> TestResult {
    reject_omitted_retained_default(true)
}

fn reject_omitted_retained_default(optional: bool) -> TestResult {
    let schema = format!("{{is: number, optional: {optional}, default: 4, secret: true}}");
    let text = source(&schema);
    let workflow = workflow(&schema)?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into()),
        source: text.into_bytes(),
    };
    let run = RunId::new(701);
    let inputs = RunInputsRecord::new(
        run,
        artifact.digest,
        source.digest,
        &workflow,
        Box::from([]),
    )?;
    let slot = workflow.input_slots().first().ok_or("input missing")?.slot;
    assert_eq!(inputs.inputs(), &[(slot, SlotValue::I64(4))]);
    // Retained bytes can be tampered with independently of their executable.
    let mut wire = serde_json::to_value(&inputs)?;
    *wire.get_mut("inputs").ok_or("inputs wire field")? =
        serde_json::to_value(Vec::<(vb_core::SlotIdx, SlotValue)>::new())?;
    *wire
        .get_mut("input_digest")
        .ok_or("input digest wire field")? =
        serde_json::to_value(vb_storage::inputs::compute_input_digest(&[])?)?;
    let forged: RunInputsRecord = serde_json::from_value(wire)?;
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    let source_digest = source.digest;
    let result = RunSubmission::new(
        source,
        artifact,
        forged,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )
    .and_then(|submission| journal.commit_run_submission(&submission, RuntimePolicy::Strict));
    assert!(
        matches!(result, Err(JournalError::InputSchemaMismatch)),
        "default omission must reject before retention; got {result:?}; retained={:?}; events={:?}",
        journal.run_inputs(run)?,
        journal.events_for_run_full(run)?
    );
    assert_eq!(
        journal.events_for_run_full(run)?,
        Vec::<vb_storage::JournalEvent>::new()
    );
    assert_eq!(journal.run_inputs(run)?, None);
    assert_eq!(journal.run_header(run)?, None);
    assert_eq!(journal.workflow_source(source_digest)?, None);
    Ok(())
}

#[test]
fn aliased_multi_root_defaults_have_order_independent_idempotent_strong_taints() -> TestResult {
    let text = "version: velvet-ballistics/v1\nname: aliased_inputs\nwhen: {manual: {}}\ninputs:\n  first: {is: object, fields: {nested: {is: object, fields: {name: {is: text, optional: true}, enabled: {is: boolean, nullable: true, default: true}}}, sibling: {is: number, default: 4}}}\n  second: {is: object, fields: {nested: {is: object, fields: {name: {is: text, default: private, secret: true}, enabled: {is: boolean, nullable: true, default: true}}}, sibling: {is: number, default: 4}}}\nsteps:\n  - id: first\n    set: {output: answer, value: '42'}\n  - id: done\n    finish: {result: answer}\n";
    let workflow = vb_compile::compile_workflow(text.as_bytes())?;
    let first = workflow.input_slots().first().ok_or("first input")?.slot;
    let second = workflow.input_slots().get(1).ok_or("second input")?.slot;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into());
    for supplied in [Taint::Clean, Taint::Random, Taint::TimeDependent] {
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let enabled = store.insert_symbol("enabled")?;
        let nested = store.insert_symbol("nested")?;
        let leaf = store.insert_object(Box::from([vb_core::ObjectField::clean(
            enabled,
            SlotValue::Null,
        )]))?;
        let a = store.insert_object(Box::from([vb_core::ObjectField::clean(
            nested,
            SlotValue::Object(leaf),
        )]))?;
        let b = store.insert_object(Box::from([vb_core::ObjectField::clean(
            nested,
            SlotValue::Object(leaf),
        )]))?;
        let roots = [
            (first, SlotValue::Object(a), Taint::Clean),
            (second, SlotValue::Object(b), supplied),
        ];
        let bundle = vb_core::root_bundle::OwnedRootBundle::new(
            Box::from(roots),
            store.checkpoint_v1(),
            &workflow,
        )?;
        let [a_root, b_root] = roots;
        let reversed = vb_core::root_bundle::OwnedRootBundle::new(
            Box::from([b_root, a_root]),
            store.checkpoint_v1(),
            &workflow,
        )?;
        assert_eq!(reversed.roots(), bundle.roots());
        assert_eq!(reversed.checkpoint(), bundle.checkpoint());
        let expected = vb_core::join_taint(supplied, Taint::Secret);
        assert_eq!(bundle.root(first), Some((SlotValue::Object(a), expected)));
        assert_eq!(bundle.root(second), Some((SlotValue::Object(b), expected)));
        for object in &bundle.checkpoint().objects {
            for field in object.iter() {
                assert_eq!(field.taint, expected);
            }
        }
        assert_eq!(
            bundle
                .checkpoint()
                .symbols
                .iter()
                .filter(|symbol| symbol.as_ref() == "private")
                .count(),
            1
        );
        let mut restored = bundle;
        for _ in 0..2 {
            let record = vb_storage::OwnedRunInputsRecordV1::new(
                RunId::new(704),
                artifact.digest,
                source,
                &restored,
                &workflow,
            )?;
            let next = record.restore(&workflow)?;
            assert_eq!(next.roots(), restored.roots());
            assert_eq!(next.checkpoint(), restored.checkpoint());
            assert_eq!(record.checkpoint(), next.checkpoint());
            restored = next;
        }
    }
    Ok(())
}

fn reject_rehashed_noncanonical_checkpoint(
    record: &vb_storage::OwnedRunInputsRecordV1,
    workflow: &CompiledWorkflow,
) -> TestResult {
    // Postcard tuples preserve the exact ordered fields of the retained body.
    type Body = (
        u16,
        RunId,
        WorkflowDigest,
        WorkflowDigest,
        WorkflowDigest,
        u16,
        Box<[vb_core::workflow::CompiledInputSlot]>,
        vb_core::workflow::ResourceContract,
        Box<[(vb_core::SlotIdx, SlotValue, Taint)]>,
        vb_core::value_store::ValueStoreCheckpointV1,
    );
    let (body, _): (Body, WorkflowDigest) = postcard::from_bytes(&postcard::to_allocvec(record)?)?;
    let (
        version,
        run,
        artifact,
        workflow_digest,
        source,
        slots,
        inputs,
        resources,
        roots,
        mut checkpoint,
    ) = body;
    let field = checkpoint
        .objects
        .iter_mut()
        .flat_map(|fields| fields.iter_mut())
        .find(|field| field.value == SlotValue::Null)
        .ok_or("nullable field")?;
    assert_eq!(field.taint, Taint::Secret);
    field.taint = Taint::Clean;
    let body = (
        version,
        run,
        artifact,
        workflow_digest,
        source,
        slots,
        inputs,
        resources,
        roots,
        checkpoint,
    );
    let digest = WorkflowDigest::from_bytes(blake3::derive_key(
        "velvet-ballistics/owned-run-inputs/v1",
        &postcard::to_allocvec(&body)?,
    ));
    let payload = postcard::to_allocvec(&(body, digest))?;
    let header = vb_storage::encode_record_header(
        vb_storage::constants::MAGIC_INDEX_RECORD,
        vb_storage::RecordKind::RunOwnedInputs,
        run.get(),
        &payload,
        vb_storage::constants::MAX_RUN_INPUT_BYTES,
    )?;
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(&payload);
    assert!(matches!(
        vb_storage::OwnedRunInputsRecordV1::decode(&bytes, run, artifact, source, workflow),
        Err(JournalError::OwnedInputs(
            vb_storage::OwnedInputError::ProjectionMismatch
        ))
    ));
    Ok(())
}
