#[test]
fn new_creates_configured_shard_count() {
    let Some(shard_count) = NonZeroUsize::new(3) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(false));
}

/// Type-level enforcement: `Runtime::new` requires an explicit `SharedRuntimeJournal`.
///
/// vb-8lijy: this enforces the master §18 contract at the compile boundary.
/// If the third argument ever disappears (silently defaulting to a drop-only
/// or volatile journal), the function-pointer coercion in this test will fail
/// to typecheck, surfacing the regression as a compile error in CI rather
/// than a silent production durability loss.
#[test]
fn new_requires_explicit_journal_at_compile_time() {
    // The signature at the right is the contract: (shard_count, config, journal) -> Runtime.
    // If anyone removes the journal argument, this line stops compiling and the
    // CI red signal is loud.
    let _new_signature: fn(
        std::num::NonZeroUsize,
        ShardConfig,
        crate::journal::SharedRuntimeJournal,
    ) -> Runtime = Runtime::new;

    // The test/benchmark-only long-named factory is the explicit non-durable path.
    // It accepts (shard_count, config) and internally wires `VolatileRuntimeJournal::shared()`.
    let _bench_signature: fn(std::num::NonZeroUsize, ShardConfig) -> Runtime =
        Runtime::new_for_tests_and_benchmarks_only;
}

#[test]
fn shutdown_graceful_enqueues_on_all_shards() {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(false));
}

#[test]
fn shutdown_graceful_processes_shards_before_journal_drain() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return Err(String::from("expected non-zero shard count"));
    };
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
    let Some(wf) = finished_workflow() else {
        return Err(String::from("expected finished workflow fixture"));
    };
    let expected_budget = vb_core::budget::AggregateResourceBudget::from_workflow(&wf)
        .map_err(|error| format!("finished workflow budget must compute: {error:?}"))?;
    let run = vb_core::ids::RunId::new(31);
    assert_eq!(runtime.submit_direct(run, wf), Ok(()));

    assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
    let encoded_bool = postcard::to_allocvec(&SlotValue::Bool(true))
        .map_err(|err| format!("bool slot serialization failed: {err:?}"))?;

    let actual = journal
        .snapshot()
        .map_err(|error| format!("journal snapshot failed: {error:?}"))?;
    let recorded_start = actual
        .iter()
        .find_map(|event| match event {
            RuntimeJournalEvent::RunBudgetOrigin { started_millis, .. } => Some(*started_millis),
            _ => None,
        })
        .ok_or_else(|| String::from("RunBudgetOrigin must record the run start"))?;
    assert_eq!(
        actual,
        vec![
            RuntimeJournalEvent::RunSubmitted {
                run,
                workflow: WorkflowDigest::from_bytes([2; 32]),
            },
            RuntimeJournalEvent::RunAdmission {
                admission: crate::admission::RunAdmission::new(
                    WorkflowDigest::from_bytes([2; 32]),
                    run,
                    CapabilitySet::empty(),
                    RuntimePolicy::Relaxed,
                )
                .with_attached_budget(expected_budget),
            },
            RuntimeJournalEvent::RunBudgetOrigin {
                run,
                started_millis: recorded_start,
            },
            RuntimeJournalEvent::StepStarted {
                run,
                step: StepIdx::new(0),
                attempt: 1
            },
            RuntimeJournalEvent::SlotWritten {
                run,
                slot: SlotIdx::new(0),
                value: encoded_bool,
                taint: vb_core::Taint::Clean,
                extra: None,
            },
            RuntimeJournalEvent::StepSucceeded {
                run,
                step: StepIdx::new(0),
                output: SlotIdx::new(0),
                attempt: 1,
            },
            RuntimeJournalEvent::StepStarted {
                run,
                step: StepIdx::new(1),
                attempt: 1
            },
            RuntimeJournalEvent::StepSucceeded {
                run,
                step: StepIdx::new(1),
                output: SlotIdx::ZERO,
                attempt: 1,
            },
            RuntimeJournalEvent::RunFinished {
                run,
                result: SlotIdx::ZERO,
                attempt: 1
            },
        ]
    );
    Ok(())
}

#[test]
fn counters_snapshot_aggregates_across_shards() {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf1),
        Ok(())
    );
    assert_eq!(
        submit_suspended(&runtime, vb_core::ids::RunId::new(2), wf2),
        Ok(())
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 2);
}

#[test]
fn drain_trace_aggregates_across_shards() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(2).ok_or("shard count")?;
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let wf1 = suspended_workflow().ok_or("first workflow")?;
    let wf2 = suspended_workflow().ok_or("second workflow")?;
    assert_eq!(
        submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf1),
        Ok(())
    );
    assert_eq!(
        submit_suspended(&runtime, vb_core::ids::RunId::new(2), wf2),
        Ok(())
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    let events = runtime.drain_trace();
    for run in [RunId::new(1), RunId::new(2)] {
        let actual = events
            .iter()
            .filter(|event| event.run_id() == run)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                TraceEvent::RunSubmitted { run },
                TraceEvent::StepStarted {
                    run,
                    step: StepIdx::ZERO
                },
                TraceEvent::ActionScheduled {
                    run,
                    step: StepIdx::ZERO
                },
            ]
        );
    }
    assert_eq!(events.len(), 6);
    assert_eq!(runtime.drain_trace(), Vec::new());
    Ok(())
}

// Helper: workflow that finishes immediately (SetConst -> Finish).
fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("finished"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts);
    assert_eq!(workflow.as_ref().map(|_| ()), Ok(()));
    workflow.ok()
}

#[test]
fn runtime_submit_direct_enqueues_on_correct_shard() {
    // Given a 2-shard runtime
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = vb_core::ids::RunId::new(1);
    // When submitting a run
    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    // Then counters show 1 run submitted
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 1);
}

#[test]
fn runtime_global_quota_spans_two_shards_and_rejects_limit_plus_one() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return Err(String::from("expected two shards"));
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let wf = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&wf)
        .map_err(|error| format!("workflow budget: {error:?}"))?
        .max_cost_units;
    assert!(cost > 0, "fixture must have a positive cost");
    let global = cost
        .checked_mul(3)
        .ok_or_else(|| String::from("cost overflow"))?;
    runtime
        .set_resource_quotas(cost, global)
        .map_err(|error| format!("set quota: {error:?}"))?;

    let tenant_a = vb_core::ids::TenantId::new(11);
    let tenant_b = vb_core::ids::TenantId::new(12);
    submit_suspended_tenant(&runtime, RunId::new(1), wf.clone(), tenant_a)
        .map_err(|error| format!("tenant a submit: {error:?}"))?;
    submit_suspended_tenant(&runtime, RunId::new(2), wf.clone(), tenant_b)
        .map_err(|error| format!("tenant b submit: {error:?}"))?;
    let pending_total = cost
        .checked_mul(2)
        .ok_or_else(|| String::from("pending cost overflow"))?;
    let pending_usage = runtime.usage_snapshot();
    assert_eq!(pending_usage.global_cost_usage(), pending_total);
    assert_eq!(
        pending_usage.tenants().tenant_cost_usage(tenant_a),
        Ok(cost)
    );
    assert_eq!(
        pending_usage.tenants().tenant_cost_usage(tenant_b),
        Ok(cost)
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    let admitted_global = cost
        .checked_mul(2)
        .ok_or_else(|| String::from("cost overflow"))?;
    let usage = runtime.usage_snapshot();
    assert_eq!(usage.tenants().tenant_cost_usage(tenant_a), Ok(cost));
    assert_eq!(usage.tenants().tenant_cost_usage(tenant_b), Ok(cost));
    assert_eq!(usage.global_cost_usage(), admitted_global);

    let rejected_total = cost
        .checked_mul(2)
        .ok_or_else(|| String::from("tenant cost overflow"))?;
    assert_eq!(
        submit_suspended_tenant(&runtime, RunId::new(3), wf, tenant_a),
        Err(RuntimeError::ResourceQuotaExceeded {
            resource: "tenant_quota_cost_units",
            requested: rejected_total,
            available: cost,
        })
    );
    assert_eq!(
        runtime.usage_snapshot().global_cost_usage(),
        admitted_global
    );
    assert_eq!(
        runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(tenant_a),
        Ok(cost)
    );
    Ok(())
}

#[test]
fn runtime_global_quota_exact_limit_admits_one_and_rejects_limit_plus_one() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return Err(String::from("expected two shards"));
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let wf = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&wf)
        .map_err(|error| format!("workflow budget: {error:?}"))?
        .max_cost_units;
    runtime
        .set_resource_quotas(u64::MAX, cost)
        .map_err(|error| format!("set quota: {error:?}"))?;
    let tenant = vb_core::ids::TenantId::new(21);
    submit_suspended_tenant(&runtime, RunId::new(11), wf.clone(), tenant)
        .map_err(|error| format!("first submit: {error:?}"))?;
    assert_eq!(runtime.global_cost_usage(), cost);
    let requested = cost
        .checked_mul(2)
        .ok_or_else(|| String::from("cost overflow"))?;
    assert_eq!(
        submit_suspended_tenant(&runtime, RunId::new(12), wf, tenant),
        Err(RuntimeError::ResourceQuotaExceeded {
            resource: "global_quota_cost_units",
            requested,
            available: cost,
        })
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.usage_snapshot().global_cost_usage(), cost);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    Ok(())
}

#[test]
fn runtime_duplicate_pending_submit_does_not_double_reserve_quota() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return Err(String::from("expected one shard"));
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let workflow = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| format!("workflow budget: {error:?}"))?
        .max_cost_units;
    let tenant = vb_core::ids::TenantId::new(22);
    let run = RunId::new(22);
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| format!("set quota: {error:?}"))?;
    submit_suspended_tenant(&runtime, run, workflow.clone(), tenant)
        .map_err(|error| format!("first submit: {error:?}"))?;
    assert_eq!(runtime.global_cost_usage(), cost);
    submit_suspended_tenant(&runtime, run, workflow, tenant)
        .map_err(|error| format!("duplicate submit: {error:?}"))?;
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.tick_all(), Err(RuntimeError::RunAlreadyExists));
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(cost)
    );
    Ok(())
}

#[test]
fn runtime_quota_rejection_leaves_no_durable_or_live_run() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return Err(String::from("expected one shard"));
    };
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
    let wf = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&wf)
        .map_err(|error| format!("workflow budget: {error:?}"))?
        .max_cost_units;
    runtime
        .set_resource_quotas(u64::MAX, 0)
        .map_err(|error| format!("set quota: {error:?}"))?;
    let tenant = vb_core::ids::TenantId::new(31);
    let run = RunId::new(21);
    assert_eq!(
        submit_suspended_tenant(&runtime, run, wf, tenant),
        Err(RuntimeError::ResourceQuotaExceeded {
            resource: "global_quota_cost_units",
            requested: cost,
            available: 0,
        })
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    let events = journal
        .snapshot()
        .map_err(|error| format!("snapshot: {error:?}"))?;
    assert_eq!(events.len(), 0, "quota rejection must precede the header");
    assert_eq!(runtime.usage_snapshot().global_cost_usage(), 0);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    Ok(())
}

#[test]
fn runtime_queue_full_rolls_back_pending_quota_reservation() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return Err(String::from("expected one shard"));
    };
    let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    for correlation in 0..16 {
        runtime
            .inspect_run(RunId::new(51), correlation)
            .map_err(|error| format!("fill command queue: {error:?}"))?;
    }
    let workflow = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let tenant = vb_core::ids::TenantId::new(51);
    assert_eq!(
        submit_suspended_tenant(&runtime, RunId::new(52), workflow, tenant),
        Err(RuntimeError::QueueFull)
    );
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(0)
    );
    Ok(())
}

#[test]
fn runtime_shutdown_releases_admitted_quota() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return Err(String::from("expected one shard"));
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let wf = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&wf)
        .map_err(|error| format!("workflow budget: {error:?}"))?
        .max_cost_units;
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| format!("set quota: {error:?}"))?;
    let tenant = vb_core::ids::TenantId::new(41);
    submit_suspended_tenant(&runtime, RunId::new(31), wf, tenant)
        .map_err(|error| format!("submit: {error:?}"))?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.usage_snapshot().global_cost_usage(), cost);
    let _report = runtime
        .shutdown_graceful()
        .map_err(|failure| format!("shutdown: {failure}"))?;
    assert_eq!(runtime.usage_snapshot().global_cost_usage(), 0);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(0)
    );
    Ok(())
}

#[test]
fn runtime_shutdown_releases_pending_quota_reservation() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return Err(String::from("expected one shard"));
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let workflow = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| format!("workflow budget: {error:?}"))?
        .max_cost_units;
    let tenant = vb_core::ids::TenantId::new(43);
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| format!("set quota: {error:?}"))?;
    submit_suspended_tenant(&runtime, RunId::new(33), workflow, tenant)
        .map_err(|error| format!("submit: {error:?}"))?;
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(cost)
    );
    let _report = runtime
        .shutdown_graceful()
        .map_err(|failure| format!("shutdown: {failure}"))?;
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(0)
    );
    Ok(())
}

#[test]
fn runtime_finish_releases_admitted_quota() -> Result<(), String> {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return Err(String::from("expected one shard"));
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let wf = finished_workflow().ok_or_else(|| String::from("finished workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&wf)
        .map_err(|error| format!("workflow budget: {error:?}"))?
        .max_cost_units;
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| format!("set quota: {error:?}"))?;
    let tenant = vb_core::ids::TenantId::new(42);
    let run = RunId::new(32);
    runtime
        .submit_direct_with_tenant(run, wf, CapabilitySet::empty(), tenant)
        .map_err(|error| format!("submit: {error:?}"))?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.usage_snapshot().global_cost_usage(), 0);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(0)
    );
    Ok(())
}

#[test]
fn runtime_cancel_run_routes_to_correct_shard() {
    // Given a 2-shard runtime with a submitted run
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&wf)
        .map(|budget| budget.max_cost_units)
        .unwrap_or_default();
    assert_eq!(runtime.set_resource_quotas(cost, cost), Ok(()));
    let run = vb_core::ids::RunId::new(1);
    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.usage_snapshot().global_cost_usage(), cost);
    // When cancelling the run
    assert_eq!(runtime.cancel_run(run), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.usage_snapshot().global_cost_usage(), 0);
    // Then the failed counter is incremented
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_failed, 1);
}

#[test]
fn runtime_complete_action_routes_to_correct_shard() {
    // Given a 2-shard runtime with a suspended run
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = vb_core::ids::RunId::new(1);
    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    // When completing the action
    assert_eq!(runtime.complete_action(run, StepIdx::new(0)), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    // Then the trace contains the ActionCompleted event
    let events = runtime.list_events(run);
    match events {
        Ok(evts) => {
            let found = evts.iter().any(|e| {
                *e == TraceEvent::ActionCompleted {
                    run,
                    step: StepIdx::new(0),
                }
            });
            assert_eq!(found, true);
        }
        Err(error) => {
            assert_eq!(Err(error), Ok(Vec::<TraceEvent>::new()));
        }
    }
}

#[test]
fn do_action_completion_writes_output_and_journals_events() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
    let Some(wf) = action_then_finish_workflow() else {
        return;
    };
    let run = vb_core::ids::RunId::new(11);
    let submit_result = submit_action_then_finish(&runtime, run, wf);
    assert_eq!(submit_result, Ok(()));
    let tick_result = runtime.tick_all();
    assert_eq!(tick_result, Ok(true));

    let events = runtime.list_events(run);
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 0);
    assert!(matches!(
        events,
        Ok(ref evts) if evts.contains(&TraceEvent::ActionScheduled {
            run,
            step: StepIdx::ZERO,
        })
    ));

    let ticket = ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(7),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run, 1, SeqNo::ZERO, ActionId::new(7)),
        capacity: 1,
        fencing_token: 0,
    };
    let value = SlotValue::I64(99);
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len: encoded_len(&value),
    };
    assert_eq!(runtime.complete_action_with_output(ticket, output), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));

    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 1);
    let trace = runtime.list_events(run);
    assert!(matches!(
        trace,
        Ok(ref evts) if evts.iter().any(|e| matches!(e,
            TraceEvent::SlotWritten { run: r, slot, .. }
            if *r == run && *slot == SlotIdx::new(1)
        )) && evts.contains(&TraceEvent::ActionCompleted {
            run,
            step: StepIdx::ZERO,
        }) && evts.contains(&TraceEvent::RunFinished { run })
    ));
    let journal_events = journal.snapshot();
    assert!(matches!(
        journal_events,
        Ok(ref evts) if evts.iter().any(|e| matches!(e,
            RuntimeJournalEvent::ActionScheduledTicket { ticket: t, output, .. }
            if t.run == run && t.step == StepIdx::ZERO && t.action == ActionId::new(7)
                && *output == SlotIdx::new(1)
        )) && evts.iter().any(|e| matches!(e,
            RuntimeJournalEvent::ActionCompletedEnvelope { ticket: t, output, encoded_len: len, .. }
            if t.run == run && t.step == StepIdx::ZERO && t.action == ActionId::new(7)
                && *output == SlotIdx::new(1) && *len == encoded_len(&value)
        ))
    ));
}

#[test]
fn runtime_action_completion_preserves_frame_when_completion_envelope_append_fails() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let journal = RejectCompletionJournal::shared();
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
    let Some(wf) = action_then_finish_workflow() else {
        return;
    };
    let run = vb_core::ids::RunId::new(13);
    assert_eq!(submit_action_then_finish(&runtime, run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    let before = active_frame(&runtime, run);
    assert!(matches!(
        before,
        Some(ref frame) if frame.read_slot(SlotIdx::new(1)).is_err()
    ));

    let value = SlotValue::I64(99);
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len: encoded_len(&value),
    };
    let completion_ticket = ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(7),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run, 1, SeqNo::ZERO, ActionId::new(7)),
        capacity: 1,
        fencing_token: 0,
    };
    assert_eq!(
        runtime.complete_action_with_output(completion_ticket, output),
        Ok(())
    );
    assert_eq!(
        runtime.tick_all(),
        Err(RuntimeError::JournalFull { capacity: 0 })
    );
    assert_eq!(active_frame(&runtime, run), before);

    let trace = runtime.list_events(run);
    assert!(matches!(
        trace,
        Ok(ref evts) if !evts.iter().any(|event| matches!(
            event,
            TraceEvent::SlotWritten { run: r, slot, .. }
                if *r == run && *slot == SlotIdx::new(1)
        )) && !evts.contains(&TraceEvent::ActionCompleted {
            run,
            step: StepIdx::ZERO,
        })
    ));
    let journal_events = journal.snapshot();
    assert!(matches!(
        journal_events,
        Ok(ref evts) if evts.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::ActionScheduledTicket { ticket: t, .. }
                if t.run == run && t.action == ActionId::new(7)
        )) && !evts.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::ActionCompletedEnvelope { ticket: t, .. }
                if t.run == run && t.action == ActionId::new(7)
        ))
    ));
}

#[test]
fn do_action_completion_rejects_wrong_action_ticket() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let Some(wf) = action_then_finish_workflow() else {
        return;
    };
    let run = vb_core::ids::RunId::new(12);
    assert_eq!(submit_action_then_finish(&runtime, run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    let ticket = ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(8),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run, 1, SeqNo::ZERO, ActionId::new(8)),
        capacity: 1,
        fencing_token: 0,
    };
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    assert_eq!(
        runtime.complete_action_with_output(ticket, output),
        Err(RuntimeError::InvalidActionCompletion)
    );
}

#[test]
fn runtime_inspect_run_returns_found_from_correct_shard() {
    // Given a 2-shard runtime with a submitted run
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = vb_core::ids::RunId::new(1);
    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    // When inspecting the run
    assert_eq!(runtime.inspect_run(run, 42), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    // Then the inspect response is Found with correct fields
    let response = runtime.take_inspect_response(run);
    match response {
        Ok(Some(InspectResponse::Found(snapshot))) => {
            assert_eq!(snapshot.run, run);
            assert_eq!(snapshot.correlation, 42);
        }
        other => {
            // Wrong: expected Found
            assert_eq!(other, Ok(None));
        }
    }
}

#[test]
fn runtime_tick_all_returns_false_when_any_shard_shuts_down() {
    // Given a 2-shard runtime
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    // When shutting down only one shard
    // Use shutdown_graceful which enqueues to all shards
    assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
    // Then tick_all returns false
    assert_eq!(runtime.tick_all(), Ok(false));
}

#[test]
fn runtime_tick_all_returns_true_when_all_shards_alive() {
    // Given a 2-shard runtime with no shutdown
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    // When ticking with empty queues
    let result = runtime.tick_all();
    // Then result is true
    assert_eq!(result, Ok(true));
}

#[test]
fn runtime_list_events_returns_events_for_target_run_only() {
    // Given a 2-shard runtime with two runs
    let Some(shard_count) = NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    let run1 = vb_core::ids::RunId::new(1);
    let run2 = vb_core::ids::RunId::new(2);
    assert_eq!(submit_suspended(&runtime, run1, wf1), Ok(()));
    assert_eq!(submit_suspended(&runtime, run2, wf2), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    // When listing events for run1
    let events = runtime.list_events(run1);
    assert_eq!(events.is_ok(), true);
    let events = events;
    let events = match events {
        Ok(e) => e,
        Err(_) => return,
    };
    // Then all events are for run1 only
    let all_run1 = events.iter().all(|e| e.run_id() == run1);
    assert_eq!(all_run1, true);
}

#[test]
fn runtime_take_inspect_response_returns_none_initially() {
    // RA-030 wave-15 (vb-sxkz6): take_inspect_response now scans all
    // shards and returns Err(RunNotFound) when the run is not on any
    // shard. The "no prior inspect" property is preserved for runs that
    // ARE on a shard; for unknown runs we expect RunNotFound.
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let run = vb_core::ids::RunId::new(1);
    let response = runtime.take_inspect_response(run);
    assert_eq!(response, Err(RuntimeError::RunNotFound));
}
