/// Bounded command processed by a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShardCommand {
    /// Activate one pristine strict submission by independently checked identities.
    ActivateOwnedSubmission {
        /// Durable run whose same-journal owned receipt must be revalidated.
        run: RunId,
        /// Expected digest of authored source bytes, not the compiled workflow.
        expected_source_digest: WorkflowDigest,
    },
    /// Submit a new run for execution.
    Submit {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
    },
    /// Submit a run whose durable header was already persisted by the runtime shell.
    SubmitPrePersisted {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
    },
    /// Submit a new run with runtime input slots already mapped by the caller.
    SubmitWithInputs {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Initial slot values written before deterministic execution starts.
        inputs: Box<[(SlotIdx, SlotValue)]>,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
    },
    /// Submit a new run with validated action contracts already bound.
    SubmitWithContracts {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
        /// Validated action contracts for Do execution.
        action_contracts: Box<[ActionContract]>,
    },
    /// Submit a new run with input slots and validated action contracts already bound.
    SubmitWithInputsAndContracts {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Initial slot values written before deterministic execution starts.
        inputs: Box<[(SlotIdx, SlotValue)]>,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
        /// Validated action contracts for Do execution.
        action_contracts: Box<[ActionContract]>,
    },
    /// Resume a suspended run from its current program counter.
    Resume {
        /// Run identifier.
        run: RunId,
    },
    /// Open a manual gate for a run and suspend the run until decided.
    OpenGate {
        /// Gate definition opened by the operator.
        gate: ManualGate,
        /// Opening time in milliseconds since the UNIX epoch.
        opened_at_ms: u64,
        /// Operator who requested the gated action.
        requester: ApproverId,
    },
    /// Submit an operator decision to an open manual gate.
    SubmitGateDecision {
        /// Run the gate belongs to; the decision counts against the
        /// run's admitted per-run queue depth.
        run: RunId,
        /// Operator submission for the open gate.
        input: GateDecisionInput,
    },
    /// Escalate an open manual gate whose deadline has passed.
    EscalateGate {
        /// Run the gate belongs to; the escalation counts against the
        /// run's admitted per-run queue depth.
        run: RunId,
        /// Gate to escalate.
        gate: GateId,
        /// Escalation time in milliseconds since the UNIX epoch.
        now_ms: u64,
    },
    /// An external action completed.
    ActionCompleted {
        /// Ticket emitted by the suspended Do step.
        ticket: vb_core::action::ActionTicket,
        /// Typed action output payload.
        output: vb_core::action::ActionOutputReady,
    },
    /// A registered adapter produced an opaque checked durable receipt.
    RegisteredActionCompleted {
        /// Canonical pending action ticket.
        ticket: vb_core::action::ActionTicket,
        /// Receipt can only be produced by the concrete registered adapter.
        receipt: vb_storage::executor::RegisterReceipt,
        /// Runtime output derived from the adapter receipt.
        output: vb_core::action::ActionOutputReady,
    },
    /// An external action completed without a typed output payload.
    ActionCompletedLegacy {
        /// Run identifier.
        run: RunId,
        /// Step that was waiting for this action.
        step: StepIdx,
    },
    /// An external action failed.
    ActionFailed {
        /// Ticket for the action being failed.
        ticket: vb_core::action::ActionTicket,
        /// Typed failure payload.
        failure: vb_core::action::ActionFailure,
    },
    /// Public runtime facade action failure.
    RuntimeActionFailed {
        /// Ticket for the action being failed.
        ticket: vb_core::action::ActionTicket,
        /// Typed failure payload.
        failure: vb_core::action::ActionFailure,
    },
    /// An external ask was answered.
    AskAnswered {
        /// Typed ask answer payload.
        answer: AskAnswer,
    },
    /// A timer fired for a suspended run.
    TimerFired {
        /// Run identifier.
        run: RunId,
        /// Freshness generation captured when the timer was emitted.
        generation: u64,
        /// Deadline captured when the timer was emitted.
        deadline: Instant,
        /// Timer kind captured when the timer was emitted.
        kind: PendingTimerKind,
    },
    /// Cancel an active run.
    Cancel {
        /// Run identifier.
        run: RunId,
        /// Optional cancellation reason.
        reason: Option<String>,
    },
    /// Kill an active run unconditionally.
    Kill {
        /// Run identifier.
        run: RunId,
        /// Optional kill reason.
        reason: Option<String>,
    },
    /// Inspect run state for diagnostic purposes.
    Inspect {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier echoed in the response.
        correlation: u64,
    },
    /// Shut down the shard gracefully.
    Shutdown,
    /// Recover a run from durable Fjall journal evidence.
    ///
    /// The runtime has already reconstructed a `RunFrame` from the journal
    /// replay and validated full recovery context. The typed
    /// `RecoverRunCommand` carries artifact verification, journal sequencing,
    /// collect pagination, and boundary classification in a single value.
    Recover { command: Box<RecoverRunCommand> },
}

impl ShardCommand {
    /// Returns the run this command belongs to, when the command has one.
    ///
    /// Ticket and answer variants carry the run inside their ticket rather
    /// than as a direct field; `Shutdown` carries no run.
    #[must_use]
    pub fn run_id(&self) -> Option<RunId> {
        match self {
            Self::ActivateOwnedSubmission { run, .. }
            | Self::Submit { run, .. }
            | Self::SubmitPrePersisted { run, .. }
            | Self::SubmitWithInputs { run, .. }
            | Self::SubmitWithContracts { run, .. }
            | Self::SubmitWithInputsAndContracts { run, .. }
            | Self::Resume { run }
            | Self::ActionCompletedLegacy { run, .. }
            | Self::TimerFired { run, .. }
            | Self::Cancel { run, .. }
            | Self::Kill { run, .. }
            | Self::Inspect { run, .. } => Some(*run),
            Self::ActionCompleted { ticket, .. }
            | Self::RegisteredActionCompleted { ticket, .. }
            | Self::ActionFailed { ticket, .. }
            | Self::RuntimeActionFailed { ticket, .. } => Some(ticket.run),
            Self::AskAnswered { answer } => Some(answer.ticket.run),
            Self::Recover { command } => Some(command.run),
            Self::OpenGate { gate, .. } => Some(gate.run),
            Self::SubmitGateDecision { run, .. } | Self::EscalateGate { run, .. } => Some(*run),
            Self::Shutdown => None,
        }
    }
}

/// Typed recovery command carrying full recovery context from journal replay.
///
/// Constructed by recovery machinery after validating journal evidence
/// and before enqueueing a Recover shard command.
/// Distinguishes a recovered historical admission from an absence
/// of durable admission evidence (legacy raw recovery path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecoveredAdmissionKind {
    /// No durable admission record exists. Legacy/raw recovery path.
    /// Carries only run identity for validation.
    Unverified {
        run_id: RunId,
        artifact_digest: WorkflowDigest,
    },
    /// Historical admission recovered from durable storage or checkpoint.
    /// Boxed: the admission record is large, and an inline variant would
    /// make this enum disproportionately large.
    Historical(Box<crate::admission::RunAdmission>),
}
