impl Shard {
    fn finish_queued_quota(
        &mut self,
        submit_run: Option<RunId>,
        result: RuntimeResult<bool>,
    ) -> RuntimeResult<bool> {
        let pending = self.pending_quota_reservation.take();
        match result {
            Ok(alive) if pending.is_none() => Ok(alive),
            Ok(_) => {
                let (_, reservation) =
                    pending.ok_or(RuntimeError::ResourceAccountingUnderflow {
                        resource: "pending_quota_reservation_missing",
                    })?;
                self.quota
                    .release(reservation.tenant, reservation.cost_units)?;
                Err(RuntimeError::ResourceAccountingUnderflow {
                    resource: "pending_quota_reservation_unclaimed",
                })
            }
            Err(error) => self.rollback_queued_quota(submit_run, pending, error),
        }
    }

    fn rollback_queued_quota(
        &mut self,
        submit_run: Option<RunId>,
        pending: Option<(RunId, crate::shard::types::PendingQuotaReservation)>,
        error: RuntimeError,
    ) -> RuntimeResult<bool> {
        let rollback = match pending {
            Some((_, reservation)) => self
                .quota
                .release(reservation.tenant, reservation.cost_units),
            None => match submit_run {
                Some(run) if !self.run_state_contains(run) => self.release_admission_quota(run),
                _ => Ok(()),
            },
        };
        match rollback {
            Ok(()) => Err(error),
            Err(rollback_error) => Err(RuntimeError::rollback_failed(
                "queued_submission_quota",
                error,
                rollback_error,
            )),
        }
    }
}
