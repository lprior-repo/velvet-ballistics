#![forbid(unsafe_code)]

//! Owned-submission fixture and helpers for the Reduce journal recovery test.

use std::{
    error::Error,
    num::NonZeroUsize,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use vb_compile::compile_workflow;
use vb_core::action::{ActionOutputReady, ActionTicket};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{
    ActionContract, ActionId, CompiledNodeKind, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx,
    Taint, ValueStore, WorkflowDigest,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    FjallJournal, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission, WorkflowSourceRecord,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn current_time_millis() -> TestResult<u64> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(u64::try_from(millis)?)
}

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: reduce_owned_journal_recovery
when:
  manual: {}
inputs:
  items:
    is: list
    of: number
    max: 2
    secret: true
    default: [3, 5]
steps:
  - id: folded
    reduce:
      variable: total
      input: "$input.items"
      initial: "10"
      steps:
        - id: tmp
          set:
            output: tmp_value
            value: "1"
        - id: update
          do:
            action: "0"
            input: "$tmp"
  - id: done
    finish:
      result: "$steps.folded"
"#;

pub(super) const RUN: u64 = 977_001;

fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::new("reduce-update")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 128,
        max_output_bytes: 128,
        timeout_ms: 1_000,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}

pub(super) fn open(journal: Arc<FjallJournal>) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

pub(super) struct Shape {
    pub(super) source: WorkflowSourceRecord,
    pub(super) accumulator: SlotIdx,
    pub(super) input_root: SlotIdx,
    pub(super) do_step: StepIdx,
    pub(super) do_output: SlotIdx,
    pub(super) limit: u32,
    pub(super) directory: tempfile::TempDir,
}

pub(super) fn shape() -> TestResult<Shape> {
    let workflow = compile_workflow(SOURCE.as_bytes())?;
    let parts = workflow.to_parts();
    let (input, accumulator, limit) = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::ReduceStart {
                input,
                accumulator,
                limit,
                ..
            } => Some((input, accumulator, limit)),
            _ => None,
        })
        .ok_or("ReduceStart absent")?;
    let input_root = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::Copy { source } if node.output.slot() == Some(input) => Some(source),
            _ => None,
        })
        .ok_or("Reduce input Copy absent")?;
    let do_node = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .ok_or("Do body absent")?;
    let do_output = do_node.output.slot().ok_or("Do output absent")?;
    let do_step = do_node.id;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        RuntimePolicy::Strict,
        &[contract()?],
    )?;
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into()),
        source: SOURCE.as_bytes().to_vec(),
    };
    let directory = tempfile::tempdir()?;
    let declaration = workflow
        .input_slots()
        .iter()
        .find(|declaration| declaration.constraint.default_cold.is_some())
        .ok_or("list default declaration absent")?
        .clone();
    let default = declaration
        .constraint
        .default_cold
        .as_ref()
        .ok_or("list default absent")?;
    let taint = if declaration.constraint.secretness.is_secret() {
        Taint::Secret
    } else {
        Taint::Clean
    };
    let mut checkpoint =
        ValueStore::with_max_slots(workflow.resource_contract().max_slots).checkpoint_v1();
    let root = vb_core::input_schema::materialize_input_default(default, &mut checkpoint, taint)?;
    let bundle = OwnedRootBundle::new(
        Box::from([(declaration.slot, root, taint)]),
        checkpoint,
        &workflow,
    )?;
    let record = OwnedRunInputsRecordV1::new(
        RunId::new(RUN),
        artifact.digest,
        source.digest,
        &bundle,
        &workflow,
    )?;
    let submission = RunSubmission::owned(
        source.clone(),
        artifact,
        record,
        RunHeaderRecord {
            run: RunId::new(RUN),
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: current_time_millis()?,
        },
    )?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    }
    Ok(Shape {
        source,
        accumulator,
        input_root,
        do_step,
        do_output,
        limit,
        directory,
    })
}

pub(super) fn start(shape: &Shape) -> TestResult<(Arc<FjallJournal>, Runtime)> {
    let journal = Arc::new(FjallJournal::open(shape.directory.path(), None)?);
    let runtime = open(Arc::clone(&journal));
    runtime.activate_owned_submission(RunId::new(RUN), shape.source.digest)?;
    Ok((journal, runtime))
}

pub(super) fn tick_to_boundary(
    runtime: &mut Runtime,
    shape: &Shape,
) -> TestResult<Option<ActionTicket>> {
    for _ in 0..8 {
        runtime.tick_all()?;
        if let Some(ticket) =
            runtime.lookup_pending_action_ticket(RunId::new(RUN), u64::from(shape.do_step.get()))
        {
            return Ok(Some(ticket));
        }
        if runtime.counters_snapshot().runs_completed > 0 {
            return Ok(None);
        }
    }
    Err("run did not reach a boundary".into())
}

pub(super) fn complete(
    runtime: &mut Runtime,
    shape: &Shape,
    ticket: ActionTicket,
    value: i64,
) -> TestResult {
    let payload = postcard::to_allocvec(&SlotValue::I64(value))?;
    runtime.complete_action_with_output(
        ticket,
        ActionOutputReady {
            output_slot: shape.do_output,
            value: SlotValue::I64(value),
            encoded_len: u32::try_from(payload.len())?,
            taint: Taint::Secret,
        },
    )?;
    Ok(())
}

pub(super) fn list_items(runtime: &Runtime, shape: &Shape) -> TestResult<Vec<SlotValue>> {
    let state = runtime.inspect_run_state(RunId::new(RUN))?;
    let SlotValue::List(id) = *state.frame.read_slot(shape.input_root)? else {
        return Err("owned input root is not a list".into());
    };
    Ok(state.store.list(id)?.to_vec())
}

pub(super) fn assert_resumable(runtime: &Runtime) -> TestResult {
    let product = runtime
        .recover_product(RunId::new(RUN))
        .map_err(|error| format!("recover_product failed before owned input seeding: {error:?}"))?;
    match product {
        RuntimeRecoveryProduct::Resumable(_) => Ok(()),
        other => Err(format!(
            "reduce run is not resumable: reason={:?} product={other:?}",
            other.cannot_resume_reason()
        )
        .into()),
    }
}

pub(super) fn run_to_finish(shape: &Shape) -> TestResult {
    let (_journal, mut runtime) = start(shape)?;
    let first = tick_to_boundary(&mut runtime, shape)?.ok_or("expected first suspension")?;
    complete(&mut runtime, shape, first, 20)?;
    let second = tick_to_boundary(&mut runtime, shape)?.ok_or("expected second suspension")?;
    complete(&mut runtime, shape, second, 30)?;
    tick_to_boundary(&mut runtime, shape)?;
    let counters = runtime.counters_snapshot();
    assert_eq!((counters.runs_completed, counters.runs_failed), (1, 0));
    Ok(())
}
