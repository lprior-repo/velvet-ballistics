impl Runtime {
    /// Cancels a run.
    pub fn cancel_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Cancel { run, reason: None })
    }

    /// Kills a run unconditionally.
    pub fn kill_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Kill { run, reason: None })
    }

    /// Resumes a suspended run from its current program counter.
    pub fn resume_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Resume { run })
    }
    /// Recovers a run from durable Fjall journal evidence and enqueues it for resumption.
    ///
    /// This is the end-to-end recovery entry point: it reads the journal,
    /// replays events to reconstruct a typed `RecoveryFrameSeedProduct`,
    /// hydrates the recoverable live-frame portion, restores durable
    /// pending-action authority when present, and enqueues
    /// `ShardCommand::Recover` on the owning shard.
    ///
    /// Recovery crosses the bead `vb-w25-runtime-a2` FINDING-001/FINDING-002
    /// typestate boundary: the storage layer never hands back a raw
    /// `RecoveryFrameSeed`. It returns the typed
    /// `vb_storage::recovery::RecoveryFrameSeedProduct` (the storage
    /// cannot-resume/resumable witness, FINDING-002), which
    /// `recover_product` classifies through the parallel runtime boundary
    /// into a `crate::recovery::RuntimeRecoveryProduct`
    /// (`SummaryOnly` / `CannotResume { reason }` / `Resumable`,
    /// FINDING-001). `recover_and_resume` enqueues only the `Resumable`
    /// product and rejects the rest with a typed error.
    ///
    /// Returns `RuntimeError::RecoveryNotAvailable` when the runtime's
    /// journal is not storage-backed (noop or volatile journals cannot
    /// be replayed for crash recovery).
    ///
    /// Returns `RuntimeError::Recovery` when the recovery operation fails
    /// the journal replay fails (missing data, corrupt snapshot, digest
    /// mismatch, or unsupported recovery state).
    pub fn recover_and_resume(&self, run: RunId) -> RuntimeResult<()> {
        match self.recover_product(run)? {
            crate::recovery::RuntimeRecoveryProduct::Resumable(product) => {
                self.enqueue_recovered_product(*product)
            }
            other => Err(Self::rejected_recovery_product_error(other)),
        }
    }

    /// Recovers a failed run from a validated retry checkpoint.
    ///
    /// Unlike [`Runtime::recover_and_resume`], this path uses the full
    /// [`crate::run_checkpoint::RunCheckpointV1`] captured at the failure boundary,
    /// rather than a legacy resumable frame seed. The checkpoint identity is checked
    /// against the run before the recovery command is enqueued; `handle_recover`
    /// re-validates the checkpoint against the independently loaded accepted artifact
    /// (artifact digest, workflow digest, policy, ABI grant authority, frame/store/collect
    /// integrity) before any runtime state mutation.
    pub fn recover_retry(
        &self,
        run: RunId,
        checkpoint: crate::run_checkpoint::RunCheckpointV1,
        next_seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        if checkpoint.run_id != run {
            return Err(RuntimeError::Recovery {
                error: "retry checkpoint run identity mismatch".to_string(),
            });
        }
        // Bind the caller-supplied checkpoint and next_seq to the actual durable chain
        // (latest accepted RetryCheckpoint + its preceding RunFailed event). A legacy,
        // missing, tampered, stale, or cross-run checkpoint fails closed here, before any
        // state or journal mutation. The CLI boundary is digest-protected, but this public
        // raw API must not trust caller-supplied run-attempt / prior-seq / next-seq authority.
        self.validate_retry_checkpoint_against_durable_chain(run, &checkpoint, next_seq)?;
        let command = crate::shard::types::RecoverRunCommand::for_retry(checkpoint, next_seq)?;
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Recover {
            command: Box::new(command),
        })
    }
}
