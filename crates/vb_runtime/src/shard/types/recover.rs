#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverRunCommand {
    /// Run identifier being recovered.
    pub run: RunId,
    /// Hydrated run frame from journal replay.
    pub frame: RunFrame,
    /// Artifact digest for artifact store verification.
    pub artifact_digest: WorkflowDigest,
    /// Workflow digest for artifact store lookup during recovery.
    pub workflow_digest: WorkflowDigest,
    /// Next durable journal sequence for this run.
    pub next_seq: EventSeq,
    /// Collect pagination states to resume after recovery.
    pub collect_states: CollectStates,
    /// Validated full state to restore for a durable retry, when present.
    pub retry_checkpoint: Option<crate::run_checkpoint::RunCheckpointV1>,
    /// Recovery boundary classification.
    pub boundary: crate::recovery::RecoveredRunBoundary,
    /// Open manual gates reconstructed from the durable journal by
    /// `reconstruct_open_gates`. Empty on the raw and retry paths, which
    /// carry no gate evidence. Restored into the shard registry by
    /// `handle_recover`; a gated run is never driven until its gate is decided.
    pub(crate) open_gates: Vec<OpenGate>,
    /// Admission evidence carried by the recovery command. The Unverified
    /// variant represents missing historical evidence rather than a fabricated
    /// verified pin. Callers on the raw command path cannot gain verified
    /// authority from the current artifact; that requires the validated
    /// recovery product path.
    pub(crate) admission: RecoveredAdmissionKind,
    /// Restored cold value store for arena-handle resolution, present when
    /// the run retained owned inputs. `None` on the raw and retry paths and
    /// for scalar runs, which restore the default empty store.
    pub(crate) restored_store: Option<vb_core::value_store::ValueStoreCheckpointV1>,
}

impl RecoverRunCommand {
    /// Constructs a legacy recovery command with no historical admission evidence.
    ///
    /// The command carries an Unverified admission record by default, representing
    /// missing historical evidence rather than a fabricated verified pin. Callers
    /// on the raw command path cannot gain verified authority from the current
    /// artifact; that requires the validated recovery product path.
    #[must_use]
    pub fn new(
        run: RunId,
        frame: RunFrame,
        artifact_digest: WorkflowDigest,
        workflow_digest: WorkflowDigest,
        next_seq: EventSeq,
        collect_states: CollectStates,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> Self {
        Self {
            run,
            frame,
            artifact_digest,
            workflow_digest,
            next_seq,
            collect_states,
            retry_checkpoint: None,
            boundary,
            open_gates: Vec::new(),
            admission: RecoveredAdmissionKind::Unverified {
                run_id: run,
                artifact_digest,
            },
            restored_store: None,
        }
    }

    /// Attaches reconstructed open manual gates from journal replay.
    ///
    /// Crate-private: only the validated recovery product path should call this,
    /// after `reconstruct_open_gates` has folded the durable gate evidence.
    #[must_use]
    pub(crate) fn with_open_gates(mut self, open_gates: Vec<OpenGate>) -> Self {
        self.open_gates = open_gates;
        self
    }

    /// Attaches a validated historical admission from a recovery product.
    ///
    /// Crate-private: only the validated recovery prefix flow should call this,
    /// after `validate_recovered_admission_evidence` has checked the admission
    /// against the run, artifact, workflow, and policy.
    #[must_use]
    pub(crate) fn with_validated_admission(
        mut self,
        admission: crate::admission::RunAdmission,
    ) -> Self {
        self.admission = RecoveredAdmissionKind::Historical(Box::new(admission));
        self
    }

    /// Attaches a restored cold value store from a validated recovery product.
    ///
    /// Crate-private: only the validated recovery product path should call
    /// this, after the owned retained inputs have been revalidated against
    /// the independently loaded artifact and workflow. The shard rebuilds
    /// this checkpoint into the recovered `RunState` instead of the default
    /// empty store.
    #[must_use]
    pub(crate) fn with_restored_store(
        mut self,
        store: Option<vb_core::value_store::ValueStoreCheckpointV1>,
    ) -> Self {
        self.restored_store = store;
        self
    }

    /// Returns the admitted per-run queue depth carried by a historical
    /// admission, or `None` when the recovery carries no verified admission.
    #[must_use]
    pub(crate) fn admitted_queue_depth(&self) -> Option<u32> {
        match &self.admission {
            RecoveredAdmissionKind::Historical(admission) => {
                admission.budget().map(|budget| budget.max_queue_depth)
            }
            RecoveredAdmissionKind::Unverified { .. } => None,
        }
    }

    /// Attaches full retry state whose identity is checked against the loaded accepted artifact.
    #[must_use]
    pub fn with_retry_checkpoint(
        mut self,
        checkpoint: crate::run_checkpoint::RunCheckpointV1,
    ) -> Self {
        self.retry_checkpoint = Some(checkpoint);
        self
    }

    /// Restores a RunFrame from checkpoint data.
    fn restore_retry_frame(frame_data: RunFrameCheckpointV1) -> RuntimeResult<RunFrame> {
        RunFrame::from_checkpoint_v1(frame_data).map_err(|error| crate::RuntimeError::Recovery {
            error: format!("retry checkpoint frame restore failed: {error:?}"),
        })
    }

    /// Restores CollectStates from checkpoint data.
    fn restore_retry_collect_states(
        data: CollectStatesCheckpointV1,
    ) -> RuntimeResult<CollectStates> {
        CollectStates::from_checkpoint_v1(data).map_err(|error| crate::RuntimeError::Recovery {
            error: format!("retry checkpoint collect restore failed: {error:?}"),
        })
    }

    fn retry_admission(
        checkpoint: &crate::run_checkpoint::RunCheckpointV1,
    ) -> RecoveredAdmissionKind {
        checkpoint.admission.clone().map_or(
            RecoveredAdmissionKind::Unverified {
                run_id: checkpoint.run_id,
                artifact_digest: checkpoint.artifact_digest,
            },
            |admission| RecoveredAdmissionKind::Historical(Box::new(admission)),
        )
    }

    /// Builds a recovery command that restores a full retry checkpoint instead of the
    /// legacy resumable frame seed. The checkpoint's own frame, value store, collect
    /// states, and admission are authoritative; the independently loaded accepted
    /// artifact is still validated by `handle_recover` before any mutation.
    ///
    /// Checkpoints without a recorded admission retain Unverified authority,
    /// preserving legacy checkpoint semantics.
    pub fn for_retry(
        checkpoint: crate::run_checkpoint::RunCheckpointV1,
        next_seq: EventSeq,
    ) -> RuntimeResult<Self> {
        let frame = Self::restore_retry_frame(checkpoint.frame.clone())?;
        let collect_states = Self::restore_retry_collect_states(checkpoint.collect_states.clone())?;
        let admission = Self::retry_admission(&checkpoint);
        Ok(Self {
            run: checkpoint.run_id,
            frame,
            artifact_digest: checkpoint.artifact_digest,
            workflow_digest: checkpoint.workflow_digest,
            next_seq,
            collect_states,
            retry_checkpoint: Some(checkpoint),
            boundary: crate::recovery::RecoveredRunBoundary::none(),
            open_gates: Vec::new(),
            admission,
            restored_store: None,
        })
    }
}
