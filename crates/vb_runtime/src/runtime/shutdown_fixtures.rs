use crate::{journal::QueuedStorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use std::{num::NonZeroUsize, sync::Arc};

use vb_core::{
    ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest},
    value::ConstValue,
    workflow::{
        CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind,
        CompiledWorkflow, InputSlotKind, ResourceContract, WorkflowParts,
    },
};
use vb_storage::{FjallJournal, JournalWriterQueue, StorageLimits};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn waiting_workflow() -> TestResult<CompiledWorkflow> {
    workflow(
        vec![
            node(
                0,
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::ZERO),
                Some(StepIdx::new(1)),
            ),
            node(
                1,
                CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::ZERO,
                },
                None,
                Some(StepIdx::new(2)),
            ),
            node(
                2,
                CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(1_000_000)],
    )
}

pub(super) fn runtime(journal: &Arc<FjallJournal>, queue: &Arc<JournalWriterQueue>) -> Runtime {
    let config = ShardConfig {
        policy: vb_core::policy::RuntimePolicy::Journaled,
        ..Default::default()
    };
    Runtime::new(
        NonZeroUsize::MIN,
        config,
        Arc::new(QueuedStorageRuntimeJournal::journaled(
            Arc::clone(journal),
            Arc::clone(queue),
        )),
    )
    .expect("valid test configuration")
}

pub(super) fn accept_workflow(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    contracts: &[vb_core::action::ActionContract],
) -> TestResult {
    vb_storage::submit_artifact_with_contracts(
        journal,
        workflow,
        vb_core::policy::RuntimePolicy::Journaled,
        contracts,
    )?;
    Ok(())
}

pub(super) fn storage() -> TestResult<(
    tempfile::TempDir,
    Arc<FjallJournal>,
    Arc<JournalWriterQueue>,
)> {
    let dir = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(dir.path(), None)?);
    let queue = Arc::new(JournalWriterQueue::new(128, 2, StorageLimits::DEFAULT)?);
    Ok((dir, journal, queue))
}

pub(super) fn finished_workflow() -> TestResult<CompiledWorkflow> {
    workflow(
        vec![
            node(
                0,
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::ZERO),
                Some(StepIdx::new(1)),
            ),
            node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(7)],
    )
}

fn node(
    id: u16,
    kind: CompiledNodeKind,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CompiledNode {
    vb_core::workflow::node(StepIdx::new(id), kind, output, next)
}

fn workflow(nodes: Vec<CompiledNode>, constants: Vec<ConstValue>) -> TestResult<CompiledWorkflow> {
    let parts = WorkflowParts {
        name: Box::from("shutdown_wait"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: constants.into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    canonical_workflow(parts)
}

fn canonical_workflow(parts: WorkflowParts) -> TestResult<CompiledWorkflow> {
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let mut canonical_parts = workflow.to_parts();
    canonical_parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let canonical = postcard::to_allocvec(&canonical_parts)?;
    canonical_parts.digest = WorkflowDigest::from_bytes(*blake3::hash(&canonical).as_bytes());
    Ok(CompiledWorkflow::try_from_parts(canonical_parts)?)
}

pub(super) fn submit_action(
    runtime: &Runtime,
    journal: &FjallJournal,
    run: vb_core::RunId,
) -> TestResult {
    use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
    use vb_core::capability::{Capability, CapabilitySet};
    use vb_core::ids::ActionId;
    use vb_core::value::SlotValue;
    let action = ActionId::new(0);
    let capability = Capability::new("shutdown_action".into(), action);
    let contract = ActionContract {
        id: action,
        name: ActionName::new("shutdown-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([capability.clone()]),
    };
    let wf = canonical_workflow(WorkflowParts {
        name: Box::from("shutdown_action"),
        digest: WorkflowDigest::from_bytes([34; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::from([node(
            0,
            CompiledNodeKind::Do {
                action,
                input: SlotIdx::ZERO,
            },
            Some(SlotIdx::ZERO),
            None,
        )]),
        constants: Box::from([]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: InputSlotKind::Any,
            constraint: CompiledInputConstraint::required(),
        }]),
    })?;
    accept_workflow(journal, &wf, std::slice::from_ref(&contract))
        .map_err(|error| format!("persist shutdown action artifact: {error}"))?;
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            wf,
            Box::from([(SlotIdx::ZERO, SlotValue::I64(9))]),
            CapabilitySet::from_grants(Box::from([capability])),
            Box::from([contract]),
        )
        .map_err(|error| format!("submit shutdown action: {error}"))?;
    Ok(())
}
