//! Characterization archive issued by the current storage/runtime authorities.

#[path = "retained_retry.rs"]
mod retained_retry;

use super::*;
use std::path::Path;
use vb_core::action::ActionTicket;
use vb_storage::{AcceptedArtifact, JournalEvent};

fn write_value<T: serde::Serialize>(root: &Path, name: &str, value: &T) -> TestResult {
    std::fs::write(root.join(name), postcard::to_allocvec(value)?)?;
    Ok(())
}

fn read_value<T: serde::de::DeserializeOwned>(
    root: &Path,
    name: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    Ok(postcard::from_bytes(&std::fs::read(root.join(name))?)?)
}

fn issue_pending(root: &Path, registered: bool) -> TestResult {
    std::fs::create_dir(root)?;
    let run = RunId::new(81_060);
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
        vb_storage::admission::submit_artifact_with_contracts(
            &journal,
            &workflow,
            RuntimePolicy::Strict,
            &[contract.clone()],
        )?
    };
    write_value(root, "artifact.postcard", &artifact)?;
    write_value(root, "contract.postcard", &contract)?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage)?;
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([0x60; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;
    let ticket = runtime
        .lookup_pending_action_ticket(run, 0)
        .ok_or("missing issued ticket")?;
    let events = journal.events_for_run_full(run)?;
    let scheduled = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::ActionScheduledTicket {
                ticket: stored,
                action_abi_digest,
                ..
            } if *stored == ticket => Some(*action_abi_digest),
            _ => None,
        })
        .ok_or("missing issued schedule")?;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, JournalEvent::RunAdmissionBudget { .. }))
    );
    write_value(root, "ticket.postcard", &ticket)?;
    write_value(root, "scheduled-abi.postcard", &scheduled)?;
    write_value(root, "events-before-dispatch.postcard", &events)?;
    if registered {
        let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
        let (scoped, dispatch) =
            scoped_grant_fixture::issue_scoped(&catalog, &artifact, &grant, ticket, scope(), 60)?;
        let mut adapter = FjallRegister::open(&root.join("register"), 8)?;
        let receipt = runtime.execute_registered_action(
            &mut adapter,
            run,
            0,
            &grant,
            vb_core::ScopedDispatch::new(&scoped, &dispatch, &[]),
        )?;
        assert_eq!(adapter.effect_count()?, 1);
        catalog.validate_receipt(&artifact, ticket, 42, &receipt)?;
        write_value(root, "receipt-digest.postcard", &receipt.digest())?;
        write_value(
            root,
            "receipt-observation.postcard",
            &(receipt.value(), receipt.effect_ordinal()),
        )?;
        drop(adapter);
        let database = fjall::Database::builder(root.join("register")).open()?;
        let receipts = database.keyspace(
            "register_receipts_v1",
            fjall::KeyspaceCreateOptions::default,
        )?;
        let rows = receipts
            .iter()
            .map(|guard| {
                let (key, value) = guard.into_inner()?;
                Ok((key.to_vec(), value.to_vec()))
            })
            .collect::<Result<Vec<_>, fjall::Error>>()?;
        assert_eq!(rows.len(), 1);
        write_value(root, "raw-receipt-rows.postcard", &rows)?;
    }
    write_value(root, "events.postcard", &journal.events_for_run_full(run)?)
}

fn issue_cold(root: &Path, name: &str, side_effect: SideEffect) -> TestResult {
    let path = root.join(name);
    std::fs::create_dir(&path)?;
    let mut contract = external_contract(ActionId::new(0))?;
    contract.side_effect = side_effect;
    let journal = FjallJournal::open(&path.join("journal"), Some(FjallConfig::default()))?;
    let artifact = vb_storage::admission::submit_artifact_with_contracts(
        &journal,
        &action_then_finish_workflow()?,
        RuntimePolicy::Journaled,
        &[contract.clone()],
    )?;
    write_value(&path, "contract.postcard", &contract)?;
    write_value(&path, "artifact.postcard", &artifact)
}

fn produce(root: &Path) -> TestResult {
    std::fs::create_dir(root)?;
    issue_pending(&root.join("none-safe"), false)?;
    issue_pending(&root.join("writes-key-required"), true)?;
    [
        ("sends", SideEffect::Sends),
        ("creates", SideEffect::Creates),
        ("destroys", SideEffect::Destroys),
    ]
    .into_iter()
    .try_for_each(|(name, effect)| issue_cold(root, name, effect))
}

fn copy_tree(source: &Path, destination: &Path, remaining: u8) -> TestResult {
    let next = remaining.checked_sub(1).ok_or("fixture nesting limit")?;
    std::fs::create_dir(destination)?;
    std::fs::read_dir(source)?.try_for_each(|entry| -> TestResult {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, next)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
        Ok(())
    })
}

fn replay(root: &Path, registered: bool) -> TestResult {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let artifact: AcceptedArtifact = read_value(root, "artifact.postcard")?;
    let ticket: ActionTicket = read_value(root, "ticket.postcard")?;
    let digest: WorkflowDigest = read_value(root, "scheduled-abi.postcard")?;
    let contract: ActionContract = read_value(root, "contract.postcard")?;
    assert_eq!(
        WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&contract)?).into()),
        digest
    );
    let journal = Arc::new(FjallJournal::open(
        &root.join("journal"),
        Some(FjallConfig::default()),
    )?);
    let stored = journal
        .compiled_ir(artifact.digest)?
        .ok_or("missing retained artifact")?;
    assert_eq!(stored.ir, std::fs::read(root.join("artifact.postcard"))?);
    assert_eq!(
        journal.events_for_run_full(ticket.run)?,
        read_value::<Vec<JournalEvent>>(root, "events.postcard")?
    );
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage)?;
    runtime.recover_and_resume(ticket.run)?;
    runtime.tick_all()?;
    assert_eq!(
        runtime.lookup_pending_action_ticket(ticket.run, 0),
        Some(ticket)
    );
    if registered {
        catalog.verify_artifact(&artifact, RuntimePolicy::Strict)?;
        let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
        let (scoped, dispatch) =
            scoped_grant_fixture::issue_scoped(&catalog, &artifact, &grant, ticket, scope(), 61)?;
        let mut adapter = FjallRegister::open(&root.join("register"), 8)?;
        let receipt = runtime.reconcile_registered_action(
            &mut adapter,
            ticket.run,
            0,
            &grant,
            &scoped,
            &dispatch,
        )?;
        assert_eq!(
            receipt.digest(),
            read_value(root, "receipt-digest.postcard")?
        );
        assert_eq!(
            (receipt.value(), receipt.effect_ordinal()),
            read_value(root, "receipt-observation.postcard")?
        );
        assert_eq!(adapter.effect_count()?, 1);
        assert_eq!(adapter.value(scope())?, Some(42));
    }
    Ok(())
}

#[test]
fn retained_taxonomy_authority_characterization_archive() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let archive = match std::env::var_os("VB_TAXONOMY_ARCHIVE") {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            if !path.is_dir() {
                return Err("retained archive must already exist".into());
            }
            path
        }
        None => {
            let path = scratch.path().join("issued");
            produce(&path)?;
            path
        }
    };
    let replay_path = scratch.path().join("replay");
    copy_tree(&archive, &replay_path, 16)?;
    replay(&replay_path.join("none-safe"), false)?;
    replay(&replay_path.join("writes-key-required"), true)?;
    ["sends", "creates", "destroys"]
        .into_iter()
        .try_for_each(|name| -> TestResult {
            let path = replay_path.join(name);
            let artifact: AcceptedArtifact = read_value(&path, "artifact.postcard")?;
            let journal = FjallJournal::open(&path.join("journal"), Some(FjallConfig::default()))?;
            assert_eq!(
                journal
                    .compiled_ir(artifact.digest)?
                    .ok_or("missing cold artifact")?
                    .ir,
                std::fs::read(path.join("artifact.postcard"))?
            );
            vb_storage::admission::verify_accepted_artifact_for_policy(
                &artifact,
                RuntimePolicy::Journaled,
            )?;
            Ok(())
        })
}
