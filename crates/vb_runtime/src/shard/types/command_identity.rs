impl ShardCommand {
    /// Returns the run this command belongs to, when the command has one.
    ///
    /// Ticket and answer variants carry the run inside their ticket rather
    /// than as a direct field; `Shutdown` carries no run.
    #[must_use]
    pub fn run_id(&self) -> Option<RunId> {
        self.gate_command_run_id()
            .or_else(|| self.direct_run_id())
            .or_else(|| self.boundary_run_id())
    }

    fn direct_run_id(&self) -> Option<RunId> {
        match self {
            Self::ActivateOwnedSubmission { run, .. }
            | Self::Submit { run, .. }
            | Self::SubmitWithLifetimeCap { run, .. }
            | Self::SubmitTenantScoped { run, .. }
            | Self::SubmitTenantWithInputsAndContracts { run, .. }
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
            _ => None,
        }
    }

    fn boundary_run_id(&self) -> Option<RunId> {
        match self {
            Self::ActionCompleted { ticket, .. }
            | Self::RegisteredActionCompleted { ticket, .. }
            | Self::ActionFailed { ticket, .. }
            | Self::RuntimeActionFailed { ticket, .. } => Some(ticket.run),
            Self::AskAnswered { answer } => Some(answer.ticket.run),
            Self::Recover { command } => Some(command.run),
            Self::Shutdown
            | Self::OpenGate { .. }
            | Self::OpenBoundGate { .. }
            | Self::SubmitGateDecision { .. }
            | Self::EscalateGate { .. }
            | Self::SubmitBoundGateDecision { .. }
            | Self::EscalateBoundGate { .. } => None,
            _ => None,
        }
    }

    fn gate_command_run_id(&self) -> Option<RunId> {
        match self {
            Self::OpenGate { gate, .. } => Some(gate.run),
            Self::OpenBoundGate { gate, .. } => Some(gate.gate.run),
            Self::SubmitGateDecision { run, .. }
            | Self::EscalateGate { run, .. }
            | Self::SubmitBoundGateDecision { run, .. }
            | Self::EscalateBoundGate { run, .. } => Some(*run),
            _ => None,
        }
    }
}
