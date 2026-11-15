//! Retains actual failure-issued checkpoint payloads, including their action registry.

use super::*;
use vb_core::action::{ActionFailure, ActionFailureCode, RetryPolicy};
use vb_core::value::Taint;
use vb_runtime::run_checkpoint::{RunCheckpointV1, decode_checkpoint_payload};
use vb_runtime::runtime::RegisteredExecutionError;
use vb_storage::EventSeq;

fn checkpoint_event(events: &[JournalEvent]) -> Result<&JournalEvent, Box<dyn std::error::Error>> {
    events
        .iter()
        .rev()
        .find(|event| matches!(event, JournalEvent::RetryCheckpoint { .. }))
        .ok_or_else(|| "missing issued retry checkpoint".into())
}

fn checkpoint(
    events: &[JournalEvent],
) -> Result<(RunCheckpointV1, EventSeq), Box<dyn std::error::Error>> {
    let JournalEvent::RetryCheckpoint {
        payload,
        payload_digest,
        seq,
        ..
    } = checkpoint_event(events)?
    else {
        return Err("missing checkpoint event".into());
    };
    assert_eq!(blake3::hash(payload).as_bytes(), payload_digest);
    let checkpoint = decode_checkpoint_payload(payload)?;
    assert_eq!(checkpoint.version, 3);
    assert_eq!(postcard::to_allocvec(&checkpoint)?, *payload);
    Ok((
        checkpoint,
        EventSeq::new(seq.get().checked_add(1).ok_or("sequence overflow")?),
    ))
}

fn issue(root: &Path, registered: bool) -> TestResult {
    std::fs::create_dir(root)?;
    let run = RunId::new(81_061);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let mut contract = external_contract(action)?;
    if !registered {
        contract.side_effect = SideEffect::None;
        contract.retry_safety = RetrySafety::Safe;
        contract.idempotency = Idempotency::DeterministicPure;
    }
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let journal = Arc::new(FjallJournal::open(
        &root.join("journal"),
        Some(FjallConfig::default()),
    )?);
    let artifact = if registered {
        let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
        write_value(root, "pack.postcard", &pack)?;
        catalog.submit_artifact(
            &journal,
            &workflow,
            RuntimePolicy::Strict,
            &[contract.clone()],
            &[pack],
        )?
    } else {
        vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            RuntimePolicy::Strict,
            &[contract.clone()],
        )?
    };
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage)?;
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42))]),
        action_grants(action),
        Box::from([contract.clone()]),
    )?;
    runtime.tick_all()?;
    let ticket = runtime
        .lookup_pending_action_ticket(run, 0)
        .ok_or("missing first ticket")?;
    if registered {
        // Preserve the provider-committed/runtime-uncompleted crash window.
        let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
        let mut register = FjallRegister::open(&root.join("register"), 8)?;
        let receipt = register.execute(&grant, ticket, 42)?;
        assert_eq!(register.effect_count()?, 1);
        write_value(root, "receipt-digest.postcard", &receipt.digest())?;
    }
    runtime.fail_action(
        ticket,
        ActionFailure {
            code: ActionFailureCode::Unknown,
            detail: None,
            retry_policy: RetryPolicy::NonRetryable,
            encoded_len: 0,
            taint: Taint::Clean,
        },
    )?;
    runtime.tick_all()?;
    let events = journal.events_for_run_full(run)?;
    let (captured, _) = checkpoint(&events)?;
    assert_eq!(captured.artifact_digest, artifact.digest);
    assert_eq!(captured.action_contracts.as_ref(), &[contract.clone()]);
    write_value(root, "artifact.postcard", &artifact)?;
    write_value(root, "contract.postcard", &contract)?;
    write_value(root, "failed-ticket.postcard", &ticket)?;
    write_value(root, "events.postcard", &events)?;
    write_value(
        root,
        "checkpoint-event.postcard",
        checkpoint_event(&events)?,
    )?;
    let JournalEvent::RetryCheckpoint { payload, .. } = checkpoint_event(&events)? else {
        return Err("missing payload".into());
    };
    std::fs::write(root.join("checkpoint-payload.postcard"), payload)?;
    Ok(())
}

fn replay_checkpoint(root: &Path, registered: bool) -> TestResult {
    let artifact: AcceptedArtifact = read_value(root, "artifact.postcard")?;
    let contract: ActionContract = read_value(root, "contract.postcard")?;
    let ticket: ActionTicket = read_value(root, "failed-ticket.postcard")?;
    let events: Vec<JournalEvent> = read_value(root, "events.postcard")?;
    let (captured, next_seq) = checkpoint(&events)?;
    assert_eq!(
        postcard::to_allocvec(&captured)?,
        std::fs::read(root.join("checkpoint-payload.postcard"))?
    );
    assert_eq!(
        postcard::to_allocvec(checkpoint_event(&events)?)?,
        std::fs::read(root.join("checkpoint-event.postcard"))?
    );
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = if registered {
        catalog.verify_artifact(&artifact, RuntimePolicy::Strict)?
    } else {
        vb_storage::admission::verify_accepted_artifact_for_policy(
            &artifact,
            RuntimePolicy::Strict,
        )?
    };
    captured.validate_against(&workflow, artifact.digest, &artifact.action_contracts)?;
    assert_eq!(captured.action_contracts.as_ref(), &[contract]);
    let journal = Arc::new(FjallJournal::open(
        &root.join("journal"),
        Some(FjallConfig::default()),
    )?);
    assert_eq!(
        journal
            .compiled_ir(artifact.digest)?
            .ok_or("missing artifact")?
            .ir,
        std::fs::read(root.join("artifact.postcard"))?
    );
    assert_eq!(journal.events_for_run_full(ticket.run)?, events);
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage)?;
    runtime.recover_retry(ticket.run, captured, next_seq)?;
    runtime.tick_all()?;
    let state = runtime.inspect_run_state(ticket.run)?;
    assert_eq!(state.action_contracts, artifact.action_contracts);
    assert_eq!(
        state
            .admission
            .as_ref()
            .ok_or("missing admission")?
            .artifact_digest(),
        artifact.digest
    );
    let retry_ticket = runtime
        .lookup_pending_action_ticket(ticket.run, 0)
        .ok_or("missing recovered retry ticket")?;
    assert_eq!(retry_ticket.run_attempt, 2);
    let after = journal.events_for_run_full(ticket.run)?;
    assert_eq!(after.get(..events.len()), Some(events.as_slice()));
    if registered {
        let mut register = FjallRegister::open(&root.join("register"), 8)?;
        let original_grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
        let receipt = register.reconcile(&original_grant, ticket, 42)?;
        assert_eq!(
            receipt.digest(),
            read_value(root, "receipt-digest.postcard")?
        );
        let retry_grant = catalog.issue_register_grant(&artifact, retry_ticket, scope())?;
        let (scoped, dispatch) = scoped_grant_fixture::issue_scoped(
            &catalog,
            &artifact,
            &retry_grant,
            retry_ticket,
            scope(),
            62,
        )?;
        let before = journal.events_for_run_full(ticket.run)?;
        let attempted = runtime.reconcile_registered_action(
            &mut register,
            ticket.run,
            0,
            &retry_grant,
            &scoped,
            &dispatch,
        );
        assert!(
            matches!(
                attempted,
                Err(RegisteredExecutionError::Executor(
                    ExecutorError::ReceiptMissing
                ))
            ),
            "unexpected recovered dispatch: {attempted:?}"
        );
        assert_eq!(journal.events_for_run_full(ticket.run)?, before);
        assert_eq!(register.effect_count()?, 1);
        assert_eq!(register.value(scope())?, Some(42));
    }
    Ok(())
}

#[test]
fn retained_retry_checkpoint_keeps_exact_action_contract_authority() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let source = match (
        std::env::var_os("VB_TAXONOMY_RETRY_ARCHIVE"),
        std::env::var_os("VB_TAXONOMY_RETRY_FREEZE"),
    ) {
        (Some(path), None) => {
            let path = std::path::PathBuf::from(path);
            if !path.is_dir() {
                return Err("retained retry archive must already exist".into());
            }
            path
        }
        (None, output) => {
            let path = output
                .map(std::path::PathBuf::from)
                .map_or_else(|| scratch.path().join("issued"), |path| path);
            std::fs::create_dir(&path)?;
            issue(&path.join("none-safe"), false)?;
            issue(&path.join("writes-key-required"), true)?;
            path
        }
        (Some(_), Some(_)) => return Err("select replay or freeze".into()),
    };
    let copy = scratch.path().join("replay");
    copy_tree(&source, &copy, 16)?;
    replay_checkpoint(&copy.join("none-safe"), false)?;
    replay_checkpoint(&copy.join("writes-key-required"), true)
}
