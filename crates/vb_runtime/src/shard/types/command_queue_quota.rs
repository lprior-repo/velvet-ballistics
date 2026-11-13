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
        let mut indexes = PendingQueueIndexes {
            pending: &mut pending,
            admitted: &mut admitted,
            submissions: &mut submissions,
        };
        let next = indexes.next_pending(run, limit, submit_depth)?;
        let next_submission = indexes.next_submission(submit_run)?;
        reserve_queued_quota(quota, quota_ledger)?;
        match self.inner.push(QueuedShardCommand {
            command: cmd,
            quota,
        }) {
            Ok(()) => {
                indexes.update(next, submit_depth, next_submission);
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

struct PendingQueueIndexes<'a> {
    pending: &'a mut indexmap::IndexMap<RunId, u32>,
    admitted: &'a mut indexmap::IndexMap<RunId, u32>,
    submissions: &'a mut indexmap::IndexMap<RunId, u32>,
}

impl PendingQueueIndexes<'_> {
    fn next_pending(
        &self,
        run: Option<RunId>,
        limit: Option<u32>,
        submit_depth: Option<(RunId, u32)>,
    ) -> RuntimeResult<Option<(RunId, u32)>> {
        let recorded = run.and_then(|run| self.admitted.get(&run).copied());
        let candidate = submit_depth.map(|(_, depth)| depth);
        let effective = [limit, recorded, candidate].into_iter().flatten().min();
        let current = run.map_or(0, |run| self.pending.get(&run).copied().unwrap_or(0));
        if let (Some(run), Some(limit)) = (run, effective)
            && current >= limit
        {
            return Err(crate::RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: current,
                limit,
            });
        }
        next_count(run, current)
    }

    fn next_submission(&self, submit_run: Option<RunId>) -> RuntimeResult<Option<(RunId, u32)>> {
        let Some(run) = submit_run else {
            return Ok(None);
        };
        let current = self.submissions.get(&run).copied().unwrap_or(0);
        current
            .checked_add(1)
            .map(|count| Some((run, count)))
            .ok_or(crate::RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: u32::MAX,
                limit: u32::MAX,
            })
    }

    fn update(
        &mut self,
        pending: Option<(RunId, u32)>,
        submit_depth: Option<(RunId, u32)>,
        submission: Option<(RunId, u32)>,
    ) {
        if let Some((run, count)) = pending {
            let _previous = self.pending.insert(run, count);
        }
        if let Some((run, depth)) = submit_depth {
            let recorded = self.admitted.entry(run).or_insert(depth);
            *recorded = (*recorded).min(depth);
        }
        if let Some((run, count)) = submission {
            let _previous = self.submissions.insert(run, count);
        }
    }
}

fn next_count(run: Option<RunId>, current: u32) -> RuntimeResult<Option<(RunId, u32)>> {
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
