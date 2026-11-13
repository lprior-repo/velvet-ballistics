#![forbid(unsafe_code)]

use super::*;
use crate::journal::StorageRuntimeJournal;
use crate::runtime::Runtime;
use crate::shard::ShardConfig;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::Arc;
use vb_core::RuntimePolicy;
use vb_core::ids::RunId;
use vb_core::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn waiting_workflow() -> TestResult<CompiledWorkflow> {
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("lifetime-cap-wait"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            vb_core::node(
                StepIdx::ZERO,
                CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
                },
                Some(SlotIdx::ZERO),
                Some(StepIdx::new(1)),
            ),
            vb_core::node(
                StepIdx::new(1),
                CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::ZERO,
                },
                None,
                Some(StepIdx::new(2)),
            ),
            vb_core::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
                None,
                None,
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(9_000_000_000_000_000)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    parts.digest =
        WorkflowDigest::from_bytes(*blake3::hash(&postcard::to_allocvec(&parts)?).as_bytes());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

#[test]
fn requested_lifetime_cap_is_durable_and_recovers_unchanged() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    let workflow = waiting_workflow()?;
    vb_storage::submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Journaled, &[])?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Journaled,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    let run = RunId::new(71_001);
    let requested = NonZeroU64::new(1).ok_or("one-second cap must be nonzero")?;
    runtime.submit_direct_with_lifetime_cap(
        run,
        workflow.clone(),
        vb_core::CapabilitySet::empty(),
        requested,
    )?;
    assert_eq!(runtime.tick_all(), Ok(true));

    let before = journal.events_for_run_full(run)?;
    let started_millis = before
        .iter()
        .find_map(|event| match event {
            vb_storage::JournalEvent::RunBudgetOriginEvent { started_millis, .. } => {
                Some(*started_millis)
            }
            _ => None,
        })
        .ok_or("durable run lifetime origin missing")?;
    let recorded = before
        .iter()
        .find_map(|event| match event {
            vb_storage::JournalEvent::RunAdmissionBudget { budget, .. } => Some(*budget),
            _ => None,
        })
        .ok_or("durable admission budget missing")?;
    assert_eq!(recorded.max_run_time_seconds, requested.get());
    assert_eq!(
        before
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunBudgetOriginEvent { .. }))
            .count(),
        1
    );

    let product = match runtime.recover_product(run)? {
        crate::recovery::RuntimeRecoveryProduct::Resumable(product) => product,
        other => return Err(format!("expected resumable recovery product, got {other:?}").into()),
    };
    let command = product.into_recover_command();
    assert_eq!(command.run_started_millis, Some(started_millis));
    let admission = match &command.admission {
        crate::shard::types::RecoveredAdmissionKind::Historical(admission) => admission,
        _ => return Err("cold recovery lost historical admission".into()),
    };
    assert_eq!(admission.budget(), Some(recorded));
    assert_eq!(
        admission.budget().map(|budget| budget.max_run_time_seconds),
        Some(1)
    );
    assert_eq!(journal.events_for_run_full(run)?, before);

    let profile_cap = vb_core::BoundednessPolicy::DEFAULT.absolute_max_run_time_seconds;
    let profile_run = RunId::new(71_002);
    runtime.submit_direct_with_lifetime_cap(
        profile_run,
        workflow.clone(),
        vb_core::CapabilitySet::empty(),
        NonZeroU64::new(profile_cap).ok_or("profile cap must be nonzero")?,
    )?;
    assert_eq!(runtime.tick_all(), Ok(true));
    let profile_events = journal.events_for_run_full(profile_run)?;
    assert!(profile_events.iter().any(|event| matches!(event,
        vb_storage::JournalEvent::RunAdmissionBudget { budget, .. }
            if budget.max_run_time_seconds == profile_cap)));

    let legacy_run = RunId::new(71_004);
    runtime.submit_direct(legacy_run, workflow.clone())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert!(
        journal
            .events_for_run_full(legacy_run)?
            .iter()
            .any(|event| matches!(
                event,
                vb_storage::JournalEvent::RunAdmissionBudget { budget, .. }
                    if budget.max_run_time_seconds == profile_cap
            ))
    );

    let over_cap_run = RunId::new(71_003);
    assert!(matches!(
        runtime.submit_direct_with_lifetime_cap(
            over_cap_run,
            workflow,
            vb_core::CapabilitySet::empty(),
            NonZeroU64::new(profile_cap.checked_add(1).ok_or("profile cap overflow")?)
                .ok_or("over-profile cap must be nonzero")?,
        ),
        Err(RuntimeError::AdmissionArtifactInvalid { .. })
    ));
    assert!(journal.events_for_run_full(over_cap_run)?.is_empty());
    Ok(())
}
