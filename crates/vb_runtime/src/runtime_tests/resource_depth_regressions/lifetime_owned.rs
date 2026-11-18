fn action_workflow_without_caller_inputs() -> Result<vb_core::workflow::CompiledWorkflow, String> {
    let nodes = Box::from([
        vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(1)),
        ),
        vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
            Some(SlotIdx::new(1)),
            None,
        ),
    ]);
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("costed_action"),
        digest: WorkflowDigest::from_bytes([81; 32]),
        nodes,
        expressions: Box::default(),
        accessors: Box::default(),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::default(),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::default(),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .map_err(|error| format!("costed action workflow: {error:?}"))
}

#[test]
fn requested_lifetime_submission_reserves_shared_global_quota() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let workflow = action_workflow_without_caller_inputs()?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
    runtime
        .set_resource_quotas(u64::MAX, cost)
        .map_err(|error| error.to_string())?;
    let cap = std::num::NonZeroU64::new(1).ok_or_else(|| String::from("positive cap"))?;
    runtime
        .submit_direct_with_lifetime_cap(
            vb_core::ids::RunId::new(903),
            workflow.clone(),
            CapabilitySet::empty(),
            cap,
        )
        .map_err(|error| format!("first requested lifetime submit: {error:?}"))?;
    assert_eq!(runtime.global_cost_usage(), cost);
    let requested = cost
        .checked_mul(2)
        .ok_or_else(|| String::from("quota cost overflow"))?;
    assert_eq!(
        runtime.submit_direct_with_lifetime_cap(
            vb_core::ids::RunId::new(904),
            workflow,
            CapabilitySet::empty(),
            cap
        ),
        Err(RuntimeError::ResourceQuotaExceeded {
            resource: "global_quota_cost_units",
            requested,
            available: cost
        })
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        Vec::new()
    );
    Ok(())
}

fn owned_waiting_workflow(deadline: u64) -> Result<vb_core::workflow::CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("owned_wait"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            vb_core::workflow::node(
                StepIdx::ZERO,
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::ZERO),
                Some(StepIdx::new(1)),
            ),
            vb_core::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::ZERO,
                },
                None,
                None,
            ),
        ]),
        expressions: Box::default(),
        accessors: Box::default(),
        constants: Box::from([ConstValue::I64(
            i64::try_from(deadline).map_err(|error| error.to_string())?,
        )]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::default(),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::default(),
    };
    let encoded = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(*blake3::hash(&encoded).as_bytes());
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

#[test]
fn owned_activation_keeps_one_durable_budget_origin_in_sequence() -> Result<(), String> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch()
        .ok_or_else(|| String::from("clock unavailable"))?;
    let deadline = now
        .checked_add(60_000)
        .ok_or_else(|| String::from("deadline overflow"))?;
    let workflow = owned_waiting_workflow(deadline)?;
    let run = vb_core::ids::RunId::new(905);
    let source_bytes = b"owned workflow source".to_vec();
    let source_digest = WorkflowDigest::from_bytes(*blake3::hash(&source_bytes).as_bytes());
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )
    .map_err(|error| error.to_string())?;
    let checkpoint =
        vb_core::value_store::ValueStore::with_max_slots(workflow.resource_contract().max_slots)
            .checkpoint_v1();
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(Box::default(), checkpoint, &workflow)
        .map_err(|error| error.to_string())?;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )
    .map_err(|error| error.to_string())?;
    let submission = vb_storage::RunSubmission::owned(
        vb_storage::WorkflowSourceRecord {
            digest: source_digest,
            source: source_bytes,
        },
        artifact.clone(),
        inputs,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: now,
        },
    )
    .map_err(|error| error.to_string())?;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    journal
        .commit_run_submission(&submission, vb_core::RuntimePolicy::Strict)
        .map_err(|error| error.to_string())?;
    let mut runtime = Runtime::new(
        NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?,
        ShardConfig {
            policy: vb_core::RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        crate::journal::StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    runtime
        .activate_owned_submission(run, source_digest)
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    let active = runtime
        .inspect_run_state(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(active.run_started_millis, Some(now));
    let events = journal
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    let expected_budget = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        active
            .admission
            .as_ref()
            .and_then(|admission| admission.budget()),
        Some(expected_budget)
    );
    assert!(
        matches!(events.get(1), Some(vb_storage::JournalEvent::RunAdmissionBudget { budget, seq, .. })
            if *budget == expected_budget && *seq == vb_storage::EventSeq::new(1))
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunBudgetOriginEvent { .. }))
            .count(),
        1
    );
    assert!(
        matches!(events.get(3), Some(vb_storage::JournalEvent::RunBudgetOriginEvent { run: found, seq, started_millis })
            if *found == run && *seq == vb_storage::EventSeq::new(3) && *started_millis == now)
    );
    Ok(())
}
