#![forbid(unsafe_code)]
//! Owned strict fixtures for Together failure recovery cut scenarios.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{ActionId, CompiledWorkflow, RunId, RuntimePolicy, SlotValue, Taint, ValueStore};
use vb_runtime::journal::{SharedRuntimeJournal, StorageRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

mod cut_journal;
use cut_journal::CutJournal;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const RUN: RunId = RunId::new(8_117);

pub(super) fn source(retry: Option<(u16, &str)>, handler: bool) -> String {
    let retry = retry.map_or_else(String::new, |(attempts, delay)| {
        format!("              try_again: {{max_attempts: {attempts}, delay: '{delay}'}}\n")
    });
    let handler_route = if handler {
        "              on_error: {handler: branch_recovered}\n"
    } else {
        ""
    };
    format!(
        "version: velvet-ballistics/v1\nname: together_failure_cuts\nwhen: {{manual: {{}}}}\ninputs: {{payload: number}}\nsteps:\n  - id: group\n    together:\n      branches:\n        - label: stable\n          steps:\n            - id: stable_value\n              set: {{output: stable_value, value: '7'}}\n        - label: attempted\n          steps:\n            - id: external\n{retry}{handler_route}              do: {{action: '0', input: '$input.payload'}}\n            - id: branch_recovered\n              set: {{output: branch_result, value: '$input.payload'}}\n  - id: success_result\n    set: {{output: success_value, value: '$step.group'}}\n  - id: done\n    finish: {{result: '$step.group'}}\n"
    )
}

pub(super) struct Case {
    pub(super) directory: tempfile::TempDir,
    pub(super) workflow: CompiledWorkflow,
    pub(super) source_digest: vb_core::WorkflowDigest,
}

impl Case {
    pub(super) fn new(retry: Option<(u16, &str)>, handler: bool) -> TestResult<Self> {
        Self::from_source(source(retry, handler))
    }

    pub(super) fn new_ask_handler() -> TestResult<Self> {
        let ask_source = source(None, true)
            .replace(
                "when: {manual: {}}",
                "when: {manual: {}}\nvars: {prompt: 'Review failed branch'}",
            )
            .replace(
                "set: {output: branch_result, value: '$input.payload'}",
                "ask: {prompt: '$vars.prompt'}",
            );
        Self::from_source(ask_source)
    }

    fn from_source(source: String) -> TestResult<Self> {
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        vb_validate::shared::validate(&workflow.to_parts())?;
        let payload = workflow
            .input_slots()
            .first()
            .map(|input| input.slot)
            .ok_or("payload input missing")?;
        let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let bundle = OwnedRootBundle::new(
            Box::from([(payload, SlotValue::I64(19), Taint::Clean)]),
            store.checkpoint_v1(),
            &workflow,
        )?;
        let contract = action_contract()?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[contract],
        )?;
        let source_digest =
            vb_core::WorkflowDigest::from_bytes(*blake3::hash(source.as_bytes()).as_bytes());
        let record =
            OwnedRunInputsRecordV1::new(RUN, artifact.digest, source_digest, &bundle, &workflow)?;
        let submission = RunSubmission::owned(
            WorkflowSourceRecord {
                digest: source_digest,
                source: source.into_bytes(),
            },
            artifact,
            record,
            RunHeaderRecord {
                run: RUN,
                workflow_id: vb_core::WorkflowId::new(0),
                compiled_digest: workflow.digest(),
                status: 0,
                accepted_at_ms: current_time_ms()?,
            },
        )?;
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        Ok(Self {
            directory,
            workflow,
            source_digest,
        })
    }

    pub(super) fn open(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let journal = Arc::new(FjallJournal::open(self.directory.path(), None)?);
        self.open_with_journal(
            journal.clone(),
            StorageRuntimeJournal::shared_strict(journal.clone()),
        )
    }

    pub(super) fn open_before_handler_cut(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let journal = Arc::new(FjallJournal::open(self.directory.path(), None)?);
        let handler = handler_step(&self.workflow)?;
        let adapter: SharedRuntimeJournal = Arc::new(CutJournal::new(journal.clone(), handler));
        self.open_with_journal(journal, adapter)
    }

    fn open_with_journal(
        &self,
        journal: Arc<FjallJournal>,
        adapter: SharedRuntimeJournal,
    ) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: RuntimePolicy::Strict,
                step_budget_per_tick: 1,
                ..ShardConfig::default()
            },
            adapter,
        );
        Ok((journal, runtime))
    }

    pub(super) fn activate(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let (journal, mut runtime) = self.open()?;
        self.start(&journal, &mut runtime)?;
        Ok((journal, runtime))
    }

    pub(super) fn activate_before_handler_cut(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let (journal, mut runtime) = self.open_before_handler_cut()?;
        self.start(&journal, &mut runtime)?;
        Ok((journal, runtime))
    }

    fn start(&self, journal: &FjallJournal, runtime: &mut Runtime) -> TestResult {
        runtime.activate_owned_submission(RUN, self.source_digest)?;
        let action = action_step(&self.workflow)?;
        for _ in 0..128 {
            runtime.tick_all()?;
            if runtime
                .lookup_pending_action_ticket(RUN, u64::from(action.get()))
                .is_some()
            {
                return Ok(());
            }
        }
        Err(format!(
            "Together branch action did not reach its pending boundary: nodes={:?}, events={:?}, counters={:?}, trace={:?}",
            self.workflow.to_parts().nodes.iter().map(|node| (node.id, &node.kind)).collect::<Vec<_>>(),
            journal.events_for_run_full(RUN)?,
            runtime.counters_snapshot(),
            runtime.list_events(RUN)?
        ).into())
    }

    pub(super) fn events(&self, journal: &FjallJournal) -> TestResult<Vec<JournalEvent>> {
        Ok(journal.events_for_run_full(RUN)?)
    }
}

pub(super) fn action_contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("together-failure-cut")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 128,
        max_output_bytes: 128,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

pub(super) fn action_step(workflow: &CompiledWorkflow) -> TestResult<vb_core::StepIdx> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, vb_core::CompiledNodeKind::Do { .. }).then_some(node.id)
        })
        .ok_or_else(|| "Do step missing".into())
}

pub(super) fn handler_step(workflow: &CompiledWorkflow) -> TestResult<vb_core::StepIdx> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .zip(workflow.to_parts().step_names.iter())
        .find_map(|(node, name)| (name.as_ref() == "branch_recovered").then_some(node.id))
        .ok_or_else(|| "branch recovery handler missing".into())
}

pub(super) fn current_time_ms() -> TestResult<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .try_into()
        .map_err(|_| "current timestamp overflow".into())
}
