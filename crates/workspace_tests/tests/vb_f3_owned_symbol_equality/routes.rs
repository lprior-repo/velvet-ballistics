use super::*;

#[test]
fn mixed_scalar_equality_remains_variant_sensitive() -> TestResult {
    for (left, right, expected) in [
        (
            SlotValue::I64(0),
            SlotValue::F64(vb_core::FiniteF64::from_i64(0)),
            false,
        ),
        (SlotValue::Null, SlotValue::Bool(false), false),
        (SlotValue::Null, SlotValue::Null, true),
        (SlotValue::Bool(true), SlotValue::Bool(true), true),
    ] {
        for prefix in ["", "vars: {unused: marker}\n"] {
            let workflow = vb_compile::compile_workflow(
                source(
                    prefix,
                    "{left: any, right: any}",
                    "$input.left == $input.right",
                )
                .as_bytes(),
            )?;
            let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
            let roots = workflow
                .input_slots()
                .iter()
                .zip([left, right])
                .map(|(input, value)| (input.slot, value, Taint::Clean))
                .collect::<Vec<_>>();
            let bundle =
                OwnedRootBundle::new(roots.into_boxed_slice(), store.checkpoint_v1(), &workflow)?;
            assert_eq!(execute(&workflow, &bundle)?, SlotValue::Bool(expected));
        }
    }
    Ok(())
}

#[test]
fn duplicate_text_object_keys_have_the_same_rejection_with_or_without_source_symbols() -> TestResult
{
    for prefix in ["", "vars: {unused: marker}\n", "vars: {unused: same}\n"] {
        let source = source(
            prefix,
            "{left: object, right: {is: text, default: same}}",
            "length($input.left)",
        );
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let a = store.insert_symbol("same")?;
        let b = store.insert_symbol("same")?;
        let object = store.insert_object(Box::from([
            vb_core::ObjectField::clean(a, SlotValue::I64(1)),
            vb_core::ObjectField::clean(b, SlotValue::I64(2)),
        ]))?;
        let input = workflow.input_slots().first().ok_or("input")?.slot;
        let result = OwnedRootBundle::new(
            Box::from([(input, SlotValue::Object(object), Taint::Clean)]),
            store.checkpoint_v1(),
            &workflow,
        );
        let Err(vb_core::root_bundle::OwnedRootBundleError::GraphDuplicateObjectKey {
            object: failed,
            key,
        }) = result
        else {
            return Err("duplicate text keys were not rejected by graph validation".into());
        };
        assert_eq!(failed, object);
        let expected = workflow
            .source_metadata()
            .and_then(|m| m.symbols().iter().position(|s| s.as_ref() == "same"))
            .unwrap_or_else(|| workflow.source_metadata().map_or(0, |m| m.symbols().len()));
        assert_eq!(key, vb_core::SymbolId::new(u32::try_from(expected)?));
    }
    Ok(())
}

fn observed(runtime: &mut vb_runtime::runtime::Runtime, expected: bool) -> TestResult {
    use vb_runtime::runtime::completion_observation::{
        CompletionObservationSlot, CompletionValueObservation, OwnedCompletionScalar,
    };
    let mut slots = [CompletionObservationSlot::default()];
    let outcome = runtime
        .tick_all_observed(&mut slots)
        .map_err(|e| format!("observe: {e:?}"))?;
    outcome.execution?;
    outcome.observation.map_err(|e| format!("observe: {e:?}"))?;
    assert_eq!(
        slots
            .first()
            .and_then(|s| s.first)
            .ok_or("completion")?
            .result,
        CompletionValueObservation::Available(OwnedCompletionScalar::Bool(expected))
    );
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn retained_owned_text_reopens_with_canonical_roots_and_unchanged_receipt() -> TestResult {
    use std::{num::NonZeroUsize, sync::Arc};
    use vb_core::{RuntimePolicy, WorkflowDigest};
    use vb_storage::{
        FjallJournal, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission, WorkflowSourceRecord,
    };
    for prefix in ["", "vars: {unused: marker}\n", "vars: {unused: same}\n"] {
        let text = source(
            prefix,
            "{left: text, right: text}",
            "$input.left == $input.right",
        );
        let workflow = vb_compile::compile_workflow(text.as_bytes())?;
        let bundle = bundle(&workflow, "same", "same")?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[],
        )?;
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into()),
            source: text.into_bytes(),
        };
        let run = RunId::new(117);
        let record =
            OwnedRunInputsRecordV1::new(run, artifact.digest, source.digest, &bundle, &workflow)?;
        let digest = record.input_digest();
        let submission = RunSubmission::owned(
            source.clone(),
            artifact.clone(),
            record,
            RunHeaderRecord {
                run,
                workflow_id: vb_core::WorkflowId::new(0),
                compiled_digest: workflow.digest(),
                status: 0,
                accepted_at_ms: 1,
            },
        )?;
        let directory = tempfile::tempdir()?;
        {
            let journal = FjallJournal::open(directory.path(), None)?;
            journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        }
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        let loaded = journal
            .owned_run_inputs(run, artifact.digest, source.digest, &workflow)?
            .ok_or("retained inputs")?;
        assert_eq!(loaded.input_digest(), digest);
        let restored = loaded.restore(&workflow)?;
        assert_eq!(restored.roots(), bundle.roots());
        assert_eq!(restored.checkpoint(), bundle.checkpoint());
        assert_eq!(execute(&workflow, &restored)?, SlotValue::Bool(true));
        let mut runtime = vb_runtime::runtime::Runtime::new(
            NonZeroUsize::MIN,
            vb_runtime::shard::ShardConfig {
                policy: RuntimePolicy::Strict,
                ..Default::default()
            },
            vb_runtime::journal::StorageRuntimeJournal::shared_strict(journal),
        );
        runtime.recover_and_resume(run)?;
        observed(&mut runtime, true)?;
    }
    Ok(())
}

struct Resolver(CompiledWorkflow);
impl vb_ipc::server::WorkflowResolver for Resolver {
    fn resolve_workflow(
        &mut self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<CompiledWorkflow, vb_ipc::server::WorkflowResolutionError> {
        if digest == self.0.digest() {
            Ok(self.0.clone())
        } else {
            Err(vb_ipc::server::WorkflowResolutionError::NotFound)
        }
    }
}

#[test]
fn scalar_cold_ipc_preserves_source_symbol_identity_and_rejects_unbacked_handles() -> TestResult {
    use std::{num::NonZeroUsize, sync::Arc};
    for (declarations, values, expected) in [
        (
            "vars: {one: same, two: other}\n",
            vec![
                SlotValue::Symbol(vb_core::SymbolId::new(0)),
                SlotValue::Symbol(vb_core::SymbolId::new(0)),
            ],
            Some(true),
        ),
        (
            "vars: {one: same, two: other}\n",
            vec![
                SlotValue::Symbol(vb_core::SymbolId::new(0)),
                SlotValue::Symbol(vb_core::SymbolId::new(1)),
            ],
            Some(false),
        ),
        (
            "",
            vec![
                SlotValue::Symbol(vb_core::SymbolId::new(0)),
                SlotValue::Symbol(vb_core::SymbolId::new(0)),
            ],
            None,
        ),
    ] {
        let workflow = vb_compile::compile_workflow(
            source(
                declarations,
                "{left: text, right: text}",
                "$input.left == $input.right",
            )
            .as_bytes(),
        )?;
        let journal = Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
        let mut runtime = vb_runtime::runtime::Runtime::new(
            NonZeroUsize::MIN,
            vb_runtime::shard::ShardConfig {
                policy: vb_core::RuntimePolicy::Relaxed,
                ..Default::default()
            },
            journal,
        );
        let payload = vb_ipc::IpcPayload::SubmitRunInline(vb_ipc::SubmitRunPayload {
            run_id: RunId::new(117),
            workflow: workflow.digest(),
            input: postcard::to_allocvec(&values)?,
        });
        let bytes = postcard::to_allocvec(&payload)?;
        let response = vb_ipc::server::handlers::runs::handle_submit_run_inline(
            &bytes,
            &mut runtime,
            Some(&mut Resolver(workflow)),
        );
        if let Some(expected) = expected {
            assert_eq!(
                response,
                vb_ipc::server::IpcResponse::AcceptedRun { run_id: 117 }
            );
            observed(&mut runtime, expected)?;
        } else {
            let error = vb_ipc::IpcError::PayloadDecodeFailed;
            assert_eq!(
                response,
                vb_ipc::server::IpcResponse::PayloadError {
                    diagnostic: error.diagnostic_code().code(),
                    message: error.to_string()
                }
            );
        }
    }
    Ok(())
}
