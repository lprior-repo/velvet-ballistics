use super::*;

fn runtime(journal: Arc<FjallJournal>) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

pub(crate) struct Cut {
    pub(crate) directory: tempfile::TempDir,
    pub(crate) workflow: CompiledWorkflow,
    pub(crate) roots: Box<[(SlotIdx, SlotValue, Taint)]>,
    pub(crate) input_store: ValueStoreCheckpointV1,
    pub(crate) frame: RunFrameCheckpointV1,
    pub(crate) store: ValueStoreCheckpointV1,
    pub(crate) events: Vec<JournalEvent>,
    pub(crate) run: RunId,
    pub(crate) input_slot: SlotIdx,
    pub(crate) leaf: StepIdx,
    pub(crate) tail: StepIdx,
    pub(crate) leaf_slot: SlotIdx,
    pub(crate) tail_slot: SlotIdx,
}

fn input_slot(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    workflow
        .input_slots()
        .first()
        .map(|input| input.slot)
        .ok_or_else(|| "missing declared input slot".to_owned())
}

fn current_time_millis() -> TestResult<u64> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    u64::try_from(millis).map_err(|error| error.to_string())
}

/// Real owned admission on a strict Fjall journal, driven to the first
/// successful nested leaf execution (a nonterminal cut after inner progress).
pub(crate) fn cut_at_leaf(cell: Cell) -> TestResult<Cut> {
    let source = source_for(cell)?;
    let workflow = workflow_for(&source)?;
    let run = cell.run;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
            .map_err(|error| format!("artifact: {error:?}"))?;
    let source_record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let source_digest = source_record.digest;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = store
        .insert_list_with_taint(Box::from(INPUT_VALUES), Box::from(INPUT_TAINTS))
        .map_err(|error| format!("owned list: {error:?}"))?;
    let slot = input_slot(&workflow)?;
    let roots = Box::from([(slot, SlotValue::List(list), Taint::Secret)]);
    let bundle = OwnedRootBundle::new(roots, store.checkpoint_v1(), &workflow)
        .map_err(|error| format!("owned root bundle: {error:?}"))?;
    let input_store = bundle.checkpoint().clone();
    let roots = bundle.roots().to_vec().into_boxed_slice();
    let inputs =
        OwnedRunInputsRecordV1::new(run, artifact.digest, source_digest, &bundle, &workflow)
            .map_err(|error| format!("owned inputs: {error:?}"))?;
    let header = RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: current_time_millis()?,
    };
    let submission = RunSubmission::owned(source_record, artifact, inputs, header)
        .map_err(|error| format!("owned submission: {error:?}"))?;
    let directory = tempfile::tempdir().map_err(|error| format!("tempdir: {error}"))?;
    let leaf = leaf_step(&workflow)?;
    let tail = step_named(&workflow, "tail")?;
    let leaf_slot = workflow
        .node(leaf)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| "missing leaf output slot".to_owned())?;
    let tail_slot = workflow
        .node(tail)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| "missing tail output slot".to_owned())?;
    let listed = match bundle.root(slot) {
        Some((SlotValue::List(listed), Taint::Secret)) => listed,
        other => return Err(format!("owned root projection changed: {other:?}")),
    };
    let (frame, store, events) = {
        let journal = Arc::new(
            FjallJournal::open(directory.path(), None)
                .map_err(|error| format!("journal open: {error:?}"))?,
        );
        journal
            .commit_run_submission(&submission, RuntimePolicy::Strict)
            .map_err(|error| format!("commit: {error:?}"))?;
        let mut runtime = runtime(Arc::clone(&journal));
        runtime
            .activate_owned_submission(run, source_digest)
            .map_err(|error| format!("activate: {error:?}"))?;
        let mut live = None;
        for _ in 0..4096 {
            runtime
                .tick_all()
                .map_err(|error| format!("live tick: {error:?}"))?;
            let state = runtime
                .inspect_run_state(run)
                .map_err(|error| format!("live inspect: {error:?}"))?;
            if state
                .frame
                .step_state(tail)
                .map_err(|error| format!("tail state: {error:?}"))?
                == StepState::Succeeded
            {
                let frame_input = state
                    .frame
                    .read_slot(slot)
                    .map_err(|error| format!("input slot: {error:?}"))?;
                if frame_input != &SlotValue::List(listed) {
                    return Err(format!("{} input handle changed", cell.label()));
                }
                if state
                    .frame
                    .read_taint(slot)
                    .map_err(|error| format!("input taint: {error:?}"))?
                    != Taint::Secret
                {
                    return Err(format!("{} input taint changed", cell.label()));
                }
                let values = state
                    .store
                    .list(listed)
                    .map_err(|error| format!("input list: {error:?}"))?;
                if values != INPUT_VALUES.as_slice() {
                    return Err(format!("{} input order changed", cell.label()));
                }
                let first = state
                    .store
                    .list_item_with_taint(listed, 0)
                    .map_err(|error| format!("item 0: {error:?}"))?;
                let second = state
                    .store
                    .list_item_with_taint(listed, 1)
                    .map_err(|error| format!("item 1: {error:?}"))?;
                if first != (SlotValue::I64(3), Taint::Secret)
                    || second != (SlotValue::I64(5), Taint::DerivedFromSecret)
                {
                    return Err(format!("{} item taints changed", cell.label()));
                }
                if state
                    .frame
                    .read_slot(leaf_slot)
                    .map_err(|error| format!("leaf slot: {error:?}"))?
                    != &SlotValue::I64(LEAF)
                    || state
                        .frame
                        .read_taint(leaf_slot)
                        .map_err(|error| format!("leaf taint: {error:?}"))?
                        != Taint::Clean
                {
                    return Err(format!("{} leaf write mismatch", cell.label()));
                }
                if state
                    .frame
                    .read_slot(tail_slot)
                    .map_err(|error| format!("tail slot: {error:?}"))?
                    != &SlotValue::I64(SENTINEL)
                    || state
                        .frame
                        .read_taint(tail_slot)
                        .map_err(|error| format!("tail taint: {error:?}"))?
                        != Taint::Clean
                {
                    return Err(format!("{} tail write mismatch", cell.label()));
                }
                live = Some((state.frame.checkpoint_v1(), state.store.checkpoint_v1()));
                break;
            }
        }
        let (frame, store) = live.ok_or_else(|| format!("{} cut never reached", cell.label()))?;
        let events = journal
            .events_for_run_full(run)
            .map_err(|error| format!("cut events: {error:?}"))?;
        if events
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFinished { .. }))
        {
            return Err(format!("{} cut landed terminal", cell.label()));
        }
        (frame, store, events)
    };
    Ok(Cut {
        directory,
        workflow,
        roots,
        input_store,
        frame,
        store,
        events,
        run,
        input_slot: slot,
        leaf,
        tail,
        leaf_slot,
        tail_slot,
    })
}
