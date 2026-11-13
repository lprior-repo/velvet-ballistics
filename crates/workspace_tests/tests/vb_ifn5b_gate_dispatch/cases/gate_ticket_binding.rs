use super::*;

pub(super) fn repeated_workflow() -> Result<CompiledWorkflow, String> {
    repeated_workflow_with_retry(false)
}

pub(super) fn repeated_retry_workflow() -> Result<CompiledWorkflow, String> {
    repeated_workflow_with_retry(true)
}

fn repeated_workflow_with_retry(retry_first: bool) -> Result<CompiledWorkflow, String> {
    let mut nodes = Vec::new();
    for index in 0..9_u16 {
        nodes.push(vb_core::workflow::node(
            StepIdx::new(index),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(
                index.checked_add(1).ok_or("workflow step overflow")?,
            )),
        ));
    }
    for (step, output, next) in [(9, 2, 10), (10, 3, 11)] {
        let mut node = vb_core::workflow::node(
            StepIdx::new(step),
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(1),
            },
            Some(SlotIdx::new(output)),
            Some(StepIdx::new(next)),
        );
        if retry_first && step == 9 {
            node.retry = Some(vb_core::workflow::CompiledRetryPolicy {
                max_attempts: 2,
                delay_ms: 0,
            });
        }
        nodes.push(node);
    }
    nodes.push(vb_core::workflow::node(
        StepIdx::new(11),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(1),
        },
        Some(SlotIdx::new(3)),
        Some(StepIdx::new(12)),
    ));
    nodes.push(vb_core::workflow::node(
        StepIdx::new(12),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(3),
        },
        None,
        None,
    ));
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::from([CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: InputSlotKind::Any,
            constraint: CompiledInputConstraint::required(),
        }]),
        name: Box::from("vb_ifn5b_repeated_gate_action"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::ConstValue::I64(42)]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let bytes = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

pub(super) fn pending_ticket_from_events(
    runtime: &mut Runtime,
    journal: &FjallJournal,
    run: RunId,
) -> Result<vb_core::ActionTicket, Box<dyn std::error::Error>> {
    for _ in 0..16 {
        let mut pending = None;
        for event in journal.events_for_run_full(run)? {
            match event {
                JournalEvent::ActionScheduledTicket { ticket, .. } => pending = Some(ticket),
                JournalEvent::ActionCompletedEnvelope { ticket, .. }
                | JournalEvent::ActionAbandoned { ticket, .. }
                    if pending == Some(ticket) =>
                {
                    pending = None
                }
                _ => {}
            }
        }
        if let Some(ticket) = pending
            && runtime.lookup_pending_action_ticket(run, u64::from(ticket.step.get()))
                == Some(ticket)
        {
            return Ok(ticket);
        }
        runtime.tick_all()?;
    }
    Err("registered action did not become pending".into())
}

#[test]
fn old_approval_cannot_authorize_second_real_action_occurrence() -> TestResult {
    let dir = storage_dir()?;
    let run = RunId::new(11_799);
    let action = ActionId::new(1);
    let workflow = repeated_workflow().map_err(|error| format!("workflow: {error}"))?;
    let contract = contract()?;
    let preceding = preceding_contract()?;
    let contracts = [preceding, contract.clone()];
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    let journal = Arc::new(FjallJournal::open(&dir, Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
        &[pack],
    )?;
    let source = owned_submission::commit(&journal, &catalog, &workflow, &artifact, run)?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut runtime = Runtime::new(
        std::num::NonZeroUsize::new(1).ok_or("shard")?,
        config,
        storage,
    );
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([4; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime
        .activate_owned_submission(run, source)
        .map_err(|error| format!("activate: {error:?}"))?;
    runtime.tick_all()?;
    let first = pending_ticket_from_events(&mut runtime, &journal, run)?;
    let approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(791),
        GateDecision::Approve,
    )?;
    let register = catalog.issue_register_grant(&artifact, first, scope())?;
    let params = vb_core::ScopedGrantParams {
        id: vb_core::GrantId::new(791),
        issuer: person("grant-authority")?,
        subject: person("run-operator")?,
        capability: Capability::new("velvet.register.write".into(), action),
        scope: scope(),
        issued_at: vb_core::GrantTimeMs::new(150),
        expires_at: vb_core::GrantTimeMs::new(1_000),
        delegable: false,
        delegated_from: None,
        revocation_epoch: vb_core::RevocationEpoch::new(1),
        break_glass: false,
        approval_receipt: approval.digest,
        signature: None,
    };
    let bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register,
        params.clone(),
        &approval,
    )?;
    let dispatch = vb_core::GrantDispatchContext::new(
        artifact.digest,
        first,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
    );
    let mut adapter = FjallRegister::open(&dir.join("register"), 8)?;
    runtime
        .execute_registered_action(
            &mut adapter,
            run,
            u64::from(first.step.get()),
            &register,
            vb_core::ScopedDispatch::new(&bound, &dispatch, &[]).with_gate_approval(&approval),
        )
        .map_err(|error| format!("first approved effect: {error:?}"))?;
    runtime.tick_all()?;
    let fresh_approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(792),
        GateDecision::Approve,
    )?;
    let mut second_pending = None;
    for _ in 0..16 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_action_ticket(run, 10) {
            second_pending = Some(ticket);
            break;
        }
    }
    let second = second_pending.ok_or("second Do occurrence did not become pending")?;
    if first == second {
        return Err("expected distinct repeated action tickets".into());
    }
    let second_register = catalog.issue_register_grant(&artifact, second, scope())?;
    let second_bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &second_register,
        params.clone(),
        &approval,
    )?;
    let second_dispatch = vb_core::GrantDispatchContext::new(
        artifact.digest,
        second,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
    );
    let count_before = adapter.effect_count()?;
    let events_before = journal.events_for_run_full(run)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(second.step.get()),
        &second_register,
        vb_core::ScopedDispatch::new(&second_bound, &second_dispatch, &[])
            .with_gate_approval(&approval),
    );
    if !matches!(
        result,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(format!(
            "expected exact Unauthorized; first={first:?}; second={second:?}; got {result:?}"
        )
        .into());
    }
    if adapter.effect_count()? != count_before
        || adapter.value(scope())? != Some(42)
        || journal.events_for_run_full(run)? != events_before
        || runtime.lookup_pending_action_ticket(run, 10) != Some(second)
    {
        return Err(
            "rejected old approval mutated adapter, receipt journal, or pending ticket".into(),
        );
    }
    let fresh_params = vb_core::ScopedGrantParams {
        approval_receipt: fresh_approval.digest,
        ..params
    };
    let fresh_bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &second_register,
        fresh_params,
        &fresh_approval,
    )?;
    runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(second.step.get()),
        &second_register,
        vb_core::ScopedDispatch::new(&fresh_bound, &second_dispatch, &[])
            .with_gate_approval(&fresh_approval),
    )?;
    if adapter.effect_count()? != count_before + 1 || adapter.value(scope())? != Some(42) {
        return Err("fresh second-ticket approval did not authorize exactly one effect".into());
    }
    Ok(())
}
