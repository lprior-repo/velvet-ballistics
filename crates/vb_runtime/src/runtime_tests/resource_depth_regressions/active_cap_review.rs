#[test]
fn review_duplicate_submit_cannot_raise_active_run_queue_cap() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let workflow = suspended_workflow().ok_or_else(|| String::from("workflow"))?;
    let mut parts = workflow.to_parts();
    parts.resource_contract.max_queue_depth = 1;
    let limited =
        vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut runtime = Runtime::new(
        shard_count,
        runtime_config(),
        Arc::new(VolatileRuntimeJournal::new()),
    );
    let run = RunId::new(950);
    let tenant = vb_core::ids::TenantId::new(950);
    submit_suspended_tenant(&runtime, run, limited, tenant).map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert!(runtime.lookup_pending_action_ticket(run, 0).is_some());
    submit_suspended_tenant(&runtime, run, workflow.clone(), tenant)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        1
    );
    assert_eq!(
        submit_suspended_tenant(&runtime, run, workflow, tenant),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: 1,
            limit: 1
        }),
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        1
    );
    Ok(())
}
