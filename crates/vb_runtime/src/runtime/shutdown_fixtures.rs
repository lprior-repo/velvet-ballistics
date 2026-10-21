use crate::{journal::QueuedStorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{
    ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest},
    value::ConstValue,
    workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts},
};
use vb_storage::{FjallJournal, JournalWriterQueue, StorageLimits};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn waiting_workflow() -> TestResult<CompiledWorkflow> {
    let node = |id, kind, output, next| CompiledNode {
        id: StepIdx::new(id),
        kind,
        output,
        next,
        on_error: None,
        error_slot: None,
    };
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("shutdown_wait"),
        digest: WorkflowDigest::from_bytes([33; 32]),
        semantic_source_identity: None,
        nodes: Box::from([
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
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(1_000_000)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    })?)
}

pub(super) fn runtime(journal: &Arc<FjallJournal>, queue: &Arc<JournalWriterQueue>) -> Runtime {
    let mut config = ShardConfig::default();
    config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    Runtime::new(
        NonZeroUsize::MIN,
        config,
        Arc::new(QueuedStorageRuntimeJournal::journaled(
            Arc::clone(journal),
            Arc::clone(queue),
        )),
    )
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
    CompiledNode {
        id: StepIdx::new(id),
        kind,
        output,
        next,
        on_error: None,
        error_slot: None,
    }
}

fn workflow(nodes: Vec<CompiledNode>, constants: Vec<ConstValue>) -> TestResult<CompiledWorkflow> {
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("shutdown_fixture"),
        digest: WorkflowDigest::from_bytes([34; 32]),
        semantic_source_identity: None,
        nodes: nodes.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    })?)
}

pub(super) fn submit_action(runtime: &Runtime, run: vb_core::RunId) -> TestResult {
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
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow(
            vec![node(
                0,
                CompiledNodeKind::Do {
                    action,
                    input: SlotIdx::ZERO,
                },
                Some(SlotIdx::ZERO),
                None,
            )],
            vec![],
        )?,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(9))]),
        CapabilitySet::from_grants(Box::from([capability])),
        Box::from([contract]),
    )?;
    Ok(())
}
