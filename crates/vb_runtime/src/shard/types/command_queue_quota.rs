impl ShardCommandQueue {
    pub(crate) fn enqueue_with_reservation(
        &self,
        cmd: ShardCommand,
        limit: Option<u32>,
        quota_candidate: Option<PendingQuotaReservation>,
        quota_ledger: Option<&crate::quota::SharedQuotaLedger>,
    ) -> RuntimeResult<()> {
        let run = cmd.run_id();
        let submit_run = pending_submit_run(&cmd);
        let submit_depth = submit_depth(&cmd);
        let mut pending = Self::lock_or_recover(&self.pending);
        let mut admitted = Self::lock_or_recover(&self.admitted_depth);
        let mut submissions = Self::lock_or_recover(&self.pending_submissions);
        let quota = pending_quota(submit_run, &submissions, quota_candidate);
        let next = next_pending(run, limit, submit_depth, &pending, &admitted)?;
        let next_submission = next_submission(submit_run, &submissions)?;
        reserve_queued_quota(quota, quota_ledger)?;
        match self.inner.push(QueuedShardCommand {
            command: cmd,
            quota,
        }) {
            Ok(()) => {
                update_pending(
                    next,
                    submit_depth,
                    next_submission,
                    &mut pending,
                    &mut admitted,
                    &mut submissions,
                );
                Ok(())
            }
            Err(_) => {
                self.rollback_reservation(crate::RuntimeError::QueueFull, quota, quota_ledger)
            }
        }
    }

    fn rollback_reservation(
        &self,
        error: crate::RuntimeError,
        reservation: Option<PendingQuotaReservation>,
        ledger: Option<&crate::quota::SharedQuotaLedger>,
    ) -> RuntimeResult<()> {
        let Some(reservation) = reservation else {
            return Err(error);
        };
        let rollback = ledger.map_or_else(
            || {
                Err(crate::RuntimeError::ResourceAccountingUnderflow {
                    resource: "pending_quota_ledger_missing",
                })
            },
            |ledger| ledger.release(reservation.tenant, reservation.cost_units),
        );
        match rollback {
            Ok(()) => Err(error),
            Err(rollback) => Err(crate::RuntimeError::rollback_failed(
                "pending_submission_quota",
                error,
                rollback,
            )),
        }
    }
}

fn submit_depth(command: &ShardCommand) -> Option<(RunId, u32)> {
    match command {
        ShardCommand::Submit { run, workflow, .. }
        | ShardCommand::SubmitWithLifetimeCap { run, workflow, .. }
        | ShardCommand::SubmitPrePersisted { run, workflow, .. }
        | ShardCommand::SubmitWithInputs { run, workflow, .. }
        | ShardCommand::SubmitWithContracts { run, workflow, .. }
        | ShardCommand::SubmitWithInputsAndContracts { run, workflow, .. }
        | ShardCommand::SubmitTenantScoped { run, workflow, .. }
        | ShardCommand::SubmitTenantWithInputsAndContracts { run, workflow, .. } => {
            vb_core::budget::AggregateResourceBudget::from_workflow(workflow)
                .ok()
                .map(|budget| (*run, budget.max_queue_depth))
        }
        _ => None,
    }
}

fn pending_quota(
    submit_run: Option<RunId>,
    submissions: &indexmap::IndexMap<RunId, u32>,
    candidate: Option<PendingQuotaReservation>,
) -> Option<PendingQuotaReservation> {
    match submit_run {
        Some(run) if submissions.contains_key(&run) => None,
        _ => candidate,
    }
}

fn next_pending(
    run: Option<RunId>,
    limit: Option<u32>,
    submit_depth: Option<(RunId, u32)>,
    pending: &indexmap::IndexMap<RunId, u32>,
    admitted: &indexmap::IndexMap<RunId, u32>,
) -> RuntimeResult<Option<(RunId, u32)>> {
    let recorded = run.and_then(|run| admitted.get(&run).copied());
    let effective = limit
        .or(recorded)
        .or_else(|| submit_depth.map(|(_, depth)| depth));
    let current = run.map_or(0, |run| pending.get(&run).copied().map_or(0, |count| count));
    if let (Some(run), Some(limit)) = (run, effective)
        && current >= limit
    {
        return Err(crate::RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: current,
            limit,
        });
    }
    match run {
        Some(run) => current.checked_add(1).map(|next| Some((run, next))).ok_or(
            crate::RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: current,
                limit: current,
            },
        ),
        None => Ok(None),
    }
}

fn next_submission(
    submit_run: Option<RunId>,
    submissions: &indexmap::IndexMap<RunId, u32>,
) -> RuntimeResult<Option<(RunId, u32)>> {
    match submit_run {
        Some(run) => submissions
            .get(&run)
            .copied()
            .map_or(0, |count| count)
            .checked_add(1)
            .map(|count| Some((run, count)))
            .ok_or(crate::RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: u32::MAX,
                limit: u32::MAX,
            }),
        None => Ok(None),
    }
}

fn reserve_queued_quota(
    reservation: Option<PendingQuotaReservation>,
    ledger: Option<&crate::quota::SharedQuotaLedger>,
) -> RuntimeResult<()> {
    if let Some(reservation) = reservation {
        ledger
            .ok_or(crate::RuntimeError::ResourceAccountingUnderflow {
                resource: "pending_quota_ledger_missing",
            })?
            .reserve(reservation.tenant, reservation.cost_units)?;
    }
    Ok(())
}

fn update_pending(
    next: Option<(RunId, u32)>,
    submit_depth: Option<(RunId, u32)>,
    next_submission: Option<(RunId, u32)>,
    pending: &mut indexmap::IndexMap<RunId, u32>,
    admitted: &mut indexmap::IndexMap<RunId, u32>,
    submissions: &mut indexmap::IndexMap<RunId, u32>,
) {
    if let Some((run, count)) = next {
        let _previous = pending.insert(run, count);
    }
    if let Some((run, depth)) = submit_depth {
        let _previous = admitted.insert(run, depth);
    }
    if let Some((run, count)) = next_submission {
        let _previous = submissions.insert(run, count);
    }
}
