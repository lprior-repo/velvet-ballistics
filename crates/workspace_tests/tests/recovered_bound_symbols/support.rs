use std::{num::NonZeroUsize, sync::Arc};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{
    CompiledWorkflow, InputSlotKind, ObjectField, RunId, RuntimePolicy, SlotIdx, SlotValue, Taint,
    ValueStore, WorkflowDigest,
};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use vb_storage::{
    AcceptedArtifact, FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord,
    RunSubmission, WorkflowSourceRecord,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) struct Case {
    pub(super) directory: tempfile::TempDir,
    pub(super) workflow: CompiledWorkflow,
    pub(super) source: WorkflowSourceRecord,
    pub(super) artifact: AcceptedArtifact,
    pub(super) bundle: OwnedRootBundle,
    pub(super) run: RunId,
    pub(super) token: SlotValue,
    pub(super) started_millis: u64,
}
impl Case {
    pub(super) fn new(present: bool) -> TestResult<Self> {
        let source_text = b"version: velvet-ballistics/v1\nname: current_bound_symbol_has\nwhen: {manual: {}}\ninputs: {obj: {is: object}, token: {is: text}}\nsteps:\n  - id: before\n    set: {output: copied, value: '$input.token'}\n  - id: check\n    set: {output: answer, value: 'has($input.obj, \"key\")'}\n  - id: done\n    finish: {result: answer}\n";
        let workflow = vb_compile::compile_workflow(source_text)?;
        let metadata = workflow
            .source_metadata()
            .ok_or("source metadata missing")?;
        assert_eq!(metadata.symbols(), [Box::<str>::from("key")]);
        let mut caller = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let noise = caller.insert_symbol("noise")?;
        let key = caller.insert_symbol(if present { "key" } else { "other" })?;
        assert_eq!(noise, vb_core::SymbolId::new(0));
        assert_eq!(key, vb_core::SymbolId::new(1));
        let object = caller.insert_object(Box::from([ObjectField::clean(
            key,
            SlotValue::Symbol(noise),
        )]))?;
        let input = |kind| {
            workflow
                .input_slots()
                .iter()
                .find(|input| input.kind == kind)
                .map(|input| input.slot)
                .ok_or("input missing")
        };
        let bundle = OwnedRootBundle::new(
            Box::from([
                (
                    input(InputSlotKind::Object)?,
                    SlotValue::Object(object),
                    Taint::DerivedFromSecret,
                ),
                (
                    input(InputSlotKind::Symbol)?,
                    SlotValue::Symbol(noise),
                    Taint::Secret,
                ),
            ]),
            caller.checkpoint_v1(),
            &workflow,
        )?;
        assert_eq!(
            bundle
                .checkpoint()
                .symbols
                .iter()
                .map(Box::as_ref)
                .collect::<Vec<_>>(),
            if present {
                vec!["key", "noise"]
            } else {
                vec!["key", "noise", "other"]
            }
        );
        let token = bundle
            .root(input(InputSlotKind::Symbol)?)
            .ok_or("token root missing")?
            .0;
        assert_eq!(token, SlotValue::Symbol(vb_core::SymbolId::new(1)));
        let prepared = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[],
        )?;
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(blake3::hash(source_text).into()),
            source: source_text.to_vec(),
        };
        let run = RunId::new(17);
        let record =
            OwnedRunInputsRecordV1::new(run, prepared.digest, source.digest, &bundle, &workflow)?;
        let started_millis = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?;
        assert!(started_millis > 0);
        let submission = RunSubmission::owned(
            source.clone(),
            prepared.clone(),
            record,
            RunHeaderRecord {
                run,
                workflow_id: vb_core::WorkflowId::new(0),
                compiled_digest: workflow.digest(),
                status: 0,
                accepted_at_ms: started_millis,
            },
        )?;
        let directory = tempfile::tempdir()?;
        let artifact = {
            let journal = FjallJournal::open(directory.path(), None)?;
            journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
            let events = journal.events_for_run_full(run)?;
            let digest = events
                .iter()
                .find_map(|event| match event {
                    JournalEvent::RunAdmissionBudget {
                        artifact_digest, ..
                    } => Some(*artifact_digest),
                    _ => None,
                })
                .ok_or("budget receipt missing")?;
            let stored = journal
                .compiled_ir(digest)?
                .ok_or("issued artifact missing")?;
            let artifact: AcceptedArtifact = postcard::from_bytes(&stored.ir)?;
            assert_eq!(artifact.digest, digest);
            assert_eq!(artifact.accepted_at_seq, vb_storage::EventSeq::new(1));
            assert_ne!(artifact.digest, prepared.digest);
            assert_eq!(
                vb_storage::admission::verify_accepted_artifact(&artifact)?.to_parts(),
                workflow.to_parts()
            );
            artifact
        };
        Ok(Self {
            directory,
            workflow,
            source,
            artifact,
            bundle,
            run,
            token,
            started_millis,
        })
    }
    pub(super) fn open(&self) -> TestResult<Arc<FjallJournal>> {
        Ok(Arc::new(FjallJournal::open(self.directory.path(), None)?))
    }
    pub(super) fn runtime(&self, journal: Arc<FjallJournal>) -> TestResult<Runtime> {
        Ok(Runtime::new(
            NonZeroUsize::new(1).ok_or("zero shards")?,
            ShardConfig {
                step_budget_per_tick: 1,
                ..ShardConfig::default()
            },
            StorageRuntimeJournal::shared_strict(journal),
        )?)
    }
    pub(super) fn retained(&self, journal: &FjallJournal) -> TestResult<Vec<u8>> {
        let record = journal
            .owned_run_inputs(
                self.run,
                self.artifact.digest,
                self.source.digest,
                &self.workflow,
            )?
            .ok_or("owned record missing")?;
        assert_eq!(record.roots(), self.bundle.roots());
        assert_eq!(record.checkpoint(), self.bundle.checkpoint());
        Ok(postcard::to_allocvec(&record)?)
    }
    pub(super) fn assert_prefix(&self, journal: &FjallJournal) -> TestResult {
        let events = journal.events_for_run_full(self.run)?;
        let [
            JournalEvent::RunAccepted { run, seq, workflow },
            admission @ JournalEvent::RunAdmissionBudget { .. },
            JournalEvent::CompatibilityDecisionEvent {
                run: compat_run,
                seq: compat_seq,
                ..
            },
            JournalEvent::RunBudgetOriginEvent {
                run: origin_run,
                seq: origin_seq,
                started_millis,
            },
        ] = events.as_slice()
        else {
            return Err("wrong current prefix".into());
        };
        assert_eq!(
            (*run, seq.get(), *workflow),
            (self.run, 0, self.workflow.digest())
        );
        assert_eq!((*compat_run, compat_seq.get()), (self.run, 2));
        assert_eq!(
            (*origin_run, origin_seq.get(), *started_millis),
            (self.run, 3, self.started_millis)
        );
        let record = journal
            .owned_run_inputs(
                self.run,
                self.artifact.digest,
                self.source.digest,
                &self.workflow,
            )?
            .ok_or("owned receipt missing")?;
        assert_eq!(
            *admission,
            JournalEvent::RunAdmissionBudget {
                run: self.run,
                seq: vb_storage::EventSeq::new(1),
                artifact_digest: self.artifact.digest,
                input_digest: vb_storage::RunInputBinding::Digest(record.input_digest()),
                policy_pin: vb_storage::RunPolicyPin::Pinned(self.artifact.policy_digest),
                granted_capabilities: vb_core::CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
                budget: vb_core::AggregateResourceBudget::from_workflow(&self.workflow)?
            }
        );
        record.validate_admission(admission)?;
        Ok(())
    }
    pub(super) fn output(&self, index: u16) -> TestResult<SlotIdx> {
        self.workflow
            .node(vb_core::StepIdx::new(index))
            .and_then(|node| node.output.slot())
            .ok_or_else(|| "output missing".into())
    }
}

pub(super) type Fixture = Case;
pub(super) fn record(case: &Case) -> TestResult<OwnedRunInputsRecordV1> {
    case.open()?
        .owned_run_inputs(
            case.run,
            case.artifact.digest,
            case.source.digest,
            &case.workflow,
        )?
        .ok_or_else(|| "owned record missing".into())
}
