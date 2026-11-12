use super::*;
use std::num::NonZeroUsize;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, ExprOp, ExprProgram, InputSlotKind,
    ResourceContract, WorkflowIrV2, WorkflowParts, node,
};
use vb_core::{
    ActionId, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprIdx, RuntimePolicy,
    ValueStore, WorkflowDigest,
};
use vb_storage::{
    AcceptedArtifact, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission, WorkflowSourceRecord,
};

pub(super) struct Case {
    directory: tempfile::TempDir,
    pub(super) workflow: CompiledWorkflow,
    pub(super) source: WorkflowSourceRecord,
    artifact: AcceptedArtifact,
    original: OwnedRootBundle,
}

impl Case {
    pub(super) fn new() -> TestResult<Self> {
        let workflow = workflow()?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[contract()?],
        )?;
        let bytes = b"duplicate-tail private runtime establishment";
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(*blake3::hash(bytes).as_bytes()),
            source: bytes.to_vec(),
        };
        let original = original(&workflow)?;
        let case = Self {
            directory: tempfile::tempdir()?,
            workflow,
            source,
            artifact,
            original,
        };
        case.persist()?;
        Ok(case)
    }

    pub(super) fn open(&self) -> TestResult<Arc<FjallJournal>> {
        Ok(Arc::new(FjallJournal::open(self.directory.path(), None)?))
    }

    fn record(&self) -> TestResult<OwnedRunInputsRecordV1> {
        Ok(OwnedRunInputsRecordV1::new(
            RUN,
            self.artifact.digest,
            self.source.digest,
            &self.original,
            &self.workflow,
        )?)
    }

    fn persist(&self) -> TestResult {
        let header = RunHeaderRecord {
            run: RUN,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: self.workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        };
        let submission = RunSubmission::owned(
            self.source.clone(),
            self.artifact.clone(),
            self.record()?,
            header,
        )?;
        let journal = self.open()?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        Ok(())
    }

    pub(super) fn retained_bytes(&self, journal: &FjallJournal) -> TestResult<Vec<u8>> {
        let actual = journal
            .owned_run_inputs(
                RUN,
                self.artifact.digest,
                self.source.digest,
                &self.workflow,
            )?
            .ok_or("missing owned input")?;
        assert_eq!(actual.checkpoint(), self.original.checkpoint());
        assert_eq!(actual.roots(), self.original.roots());
        let expected = postcard::to_allocvec(&self.record()?)?;
        let bytes = postcard::to_allocvec(&actual)?;
        assert_eq!(bytes, expected);
        Ok(bytes)
    }
}

pub(super) fn actor(journal: Arc<FjallJournal>, step_budget: u64) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        crate::shard::ShardConfig {
            step_budget_per_tick: step_budget,
            policy: RuntimePolicy::Strict,
            ..crate::shard::ShardConfig::default()
        },
        crate::journal::StorageRuntimeJournal::shared_strict(journal),
    )
}

fn original(workflow: &CompiledWorkflow) -> TestResult<OwnedRootBundle> {
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let noise = store.insert_symbol("noise")?;
    assert_eq!(noise, SymbolId::new(0));
    let key = store.insert_symbol("key")?;
    assert_eq!(key, SymbolId::new(1));
    let object = store.insert_object(Box::from([ObjectField::clean(
        key,
        SlotValue::Symbol(noise),
    )]))?;
    assert_eq!(object, ObjectId::new(0));
    Ok(OwnedRootBundle::new(
        Box::from([(SlotIdx::ZERO, SlotValue::Object(object), Taint::Clean)]),
        store.checkpoint_v1(),
        workflow,
    )?)
}

fn workflow() -> TestResult<CompiledWorkflow> {
    let mut parts = parts()?;
    let payloads: Box<[Box<str>]> = Box::from([Box::from("key")]);
    let ir = WorkflowIrV2 {
        wire_version: 2,
        parts: parts.clone(),
        symbol_payloads: payloads.clone(),
    };
    let mut preimage = vec![1_u8, 255, 86, 66];
    preimage.extend(postcard::to_allocvec(&ir)?);
    parts.digest = WorkflowDigest::from_bytes(*blake3::hash(&preimage).as_bytes());
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        parts, payloads,
    )?)
}

fn parts() -> TestResult<WorkflowParts> {
    Ok(WorkflowParts {
        semantic_source_identity: None,
        name: "duplicate-tail-retry".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes(),
        expressions: Box::from([ExprProgram::try_from_ops(Box::from([
            ExprOp::LoadSlot(SlotIdx::ZERO),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Has,
        ]))?]),
        constants: Box::from([ConstValue::I64(11), ConstValue::Symbol(SymbolId::new(0))]),
        accessors: Box::default(),
        slot_count: 5,
        symbols_count: 1,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        input_slots: Box::from([CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: InputSlotKind::Object,
            constraint: CompiledInputConstraint::default(),
        }]),
    })
}

fn nodes() -> Box<[vb_core::CompiledNode]> {
    Box::from([setup_node(), action_node(), has_node(), finish_node()])
}

fn setup_node() -> vb_core::CompiledNode {
    node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(ACTION),
    )
}

fn action_node() -> vb_core::CompiledNode {
    node(
        ACTION,
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(2)),
    )
}

fn has_node() -> vb_core::CompiledNode {
    node(
        StepIdx::new(2),
        CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        },
        Some(SlotIdx::new(3)),
        Some(StepIdx::new(3)),
    )
}

fn finish_node() -> vb_core::CompiledNode {
    node(
        StepIdx::new(3),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(3),
        },
        None,
        None,
    )
}

pub(super) fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("duplicate-retry-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::default(),
    })
}
