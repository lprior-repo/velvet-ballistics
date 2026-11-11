type BoundInputTestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq)]
struct BoundPublicationSnapshot {
    boundary: crate::shard::ShardPendingBoundarySnapshot,
    counters: crate::counters::CounterSnapshot,
    events: Vec<RuntimeJournalEvent>,
    traces: Vec<TraceEvent>,
    sequences: indexmap::IndexMap<RunId, vb_storage::EventSeq>,
    pools: Vec<((u16, u16), usize)>,
    capacities: (usize, usize, usize),
}

fn bound_publication_snapshot(
    shard: &Shard,
    journal: &crate::journal::VolatileRuntimeJournal,
    run: RunId,
) -> BoundInputTestResult<BoundPublicationSnapshot> {
    Ok(BoundPublicationSnapshot {
        boundary: shard.pending_boundary_snapshot(0, 4),
        counters: shard.counters.snapshot(),
        events: journal.snapshot()?,
        traces: shard.trace_ring.snapshot_for_run(run, 16),
        sequences: shard.journal_sequences.clone(),
        pools: shard
            .frame_pools
            .iter()
            .map(|(key, pool)| (*key, pool.available()))
            .collect(),
        capacities: (
            shard.runs.capacity(),
            shard.journal_sequences.capacity(),
            shard.pending_ask_authorities.capacity(),
        ),
    })
}

fn bound_submission_workflow(
    bound: u16,
    prefix: &[&str],
) -> BoundInputTestResult<CompiledWorkflow> {
    let mut parts = suspended_workflow()
        .ok_or("suspended workflow must compile")?
        .to_parts();
    parts.resource_contract.max_slots = bound;
    parts.symbols_count = u32::try_from(prefix.len())?;
    parts.nodes = bound_submission_nodes();
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        parts,
        prefix.iter().map(|text| Box::<str>::from(*text)).collect(),
    )?)
}

fn bound_submission_nodes() -> Box<[vb_core::CompiledNode]> {
    Box::from([
        vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
            None,
            Some(StepIdx::new(1)),
        ),
        vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
            None,
            None,
        ),
    ])
}

fn bound_submission_shard() -> (
    Shard,
    std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
) {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shard = Shard::new_with_journal(small_config(), journal.clone());
    (shard, journal)
}

fn bound_submit(
    shard: &mut Shard,
    run: RunId,
    workflow: CompiledWorkflow,
    value: SlotValue,
) -> crate::RuntimeResult<()> {
    shard.handle_submit_with_inputs(
        run,
        workflow,
        &[(SlotIdx::ZERO, value)],
        CapabilitySet::from_grants(Box::default()),
    )
}

#[test]
fn bound_symbol_runtime_publication_atomicity() -> BoundInputTestResult {
    // Given a valid bound workflow whose prefix exceeds the run arena budget.
    let (mut shard, journal) = bound_submission_shard();
    let workflow = bound_submission_workflow(1, &["key", "compiler"])?;
    let run = RunId::new(400);
    let frame = shard.take_frame_for(run, &workflow)?;
    shard.release_frame(frame);
    let before = bound_publication_snapshot(&shard, &journal, run)?;
    // When the real fresh-submit handler prepares the prefix.
    let result = bound_submit(&mut shard, run, workflow, SlotValue::I64(7));
    assert_eq!(
        result,
        Err(crate::RuntimeError::from(
            vb_core::value_store::InputSymbolBindingError::Core(
                vb_core::CoreError::BudgetExceeded {
                    budget: "max_slots",
                    limit: 1
                }
            )
        ))
    );
    // Then no authority, frame, journal, trace, counter or reservation was published.
    assert_eq!(bound_publication_snapshot(&shard, &journal, run)?, before);
    assert!(shard.run_state_get(run).is_none());
    assert!(shard.runtime_state_get(run).is_none());
    Ok(())
}

#[test]
fn bound_symbol_runtime_direct_handles_require_owned_store() -> BoundInputTestResult {
    [
        SlotValue::Symbol(vb_core::SymbolId::new(0)),
        SlotValue::List(vb_core::ListId::new(0)),
        SlotValue::Object(vb_core::ObjectId::new(0)),
        SlotValue::Blob(vb_core::BlobId::new(0)),
    ]
    .into_iter()
    .try_for_each(|value| assert_bound_handle_rejected(value))
}

fn assert_bound_handle_rejected(value: SlotValue) -> BoundInputTestResult {
    let (mut shard, journal) = bound_submission_shard();
    let run = RunId::new(401);
    let before = bound_publication_snapshot(&shard, &journal, run)?;
    assert_eq!(
        bound_submit(&mut shard, run, bound_submission_workflow(8, &[])?, value),
        Err(RuntimeError::UnsupportedOperation {
            operation: "bound_input_requires_owned_store"
        })
    );
    assert_eq!(bound_publication_snapshot(&shard, &journal, run)?, before);
    assert!(shard.run_state_get(run).is_none());
    Ok(())
}

#[test]
fn bound_symbol_runtime_direct_scalars_install_compiler_prefix() -> BoundInputTestResult {
    assert_bound_scalar_store(&["key", "compiler"], vec!["key".into(), "compiler".into()])
}

#[test]
fn bound_symbol_runtime_direct_empty_prefix_is_bound() -> BoundInputTestResult {
    assert_bound_scalar_store(&[], vec![])
}

fn assert_bound_scalar_store(
    prefix: &[&str],
    expected_symbols: Vec<Box<str>>,
) -> BoundInputTestResult {
    let (mut shard, _) = bound_submission_shard();
    let run = RunId::new(402);
    bound_submit(
        &mut shard,
        run,
        bound_submission_workflow(8, prefix)?,
        SlotValue::I64(7),
    )?;
    let state = shard
        .run_state_get(run)
        .ok_or("submitted Wait must retain state")?;
    let mut expected = vb_core::ValueStore::with_max_slots(8).checkpoint_v1();
    expected.symbols = expected_symbols;
    assert_eq!(state.store.checkpoint_v1(), expected);
    assert_eq!(state.frame.read_slot(SlotIdx::ZERO)?, &SlotValue::I64(7));
    assert_eq!(state.frame.read_taint(SlotIdx::ZERO)?, Taint::Clean);
    assert_eq!(shard.counters.snapshot().runs_submitted, 1);
    Ok(())
}
