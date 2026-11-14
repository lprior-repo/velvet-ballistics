impl Shard {
    fn dispatch_command(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        use ShardCommand::*;
        match cmd {
            cmd @ (Submit { .. } | SubmitPrePersisted { .. } | SubmitWithLifetimeCap { .. }) => {
                self.dispatch_basic_submit(cmd)
            }
            cmd @ (SubmitTenantScoped { .. } | SubmitTenantWithInputsAndContracts { .. }) => {
                self.dispatch_tenant_submit(cmd)
            }
            cmd @ (SubmitWithInputs { .. }
            | SubmitWithContracts { .. }
            | SubmitWithInputsAndContracts { .. }) => self.dispatch_input_submit(cmd),
            cmd @ (OpenGate { .. } | SubmitGateDecision { .. } | EscalateGate { .. }) => {
                self.dispatch_legacy_gate(cmd)
            }
            cmd @ (OpenBoundGate { .. }
            | SubmitBoundGateDecision { .. }
            | EscalateBoundGate { .. }) => self.dispatch_bound_gate(cmd),
            cmd @ (ActionCompleted { .. }
            | RegisteredActionCompleted { .. }
            | ActionCompletedLegacy { .. }) => self.dispatch_action_completion(cmd),
            cmd @ (ActionFailed { .. } | RuntimeActionFailed { .. } | AskAnswered { .. }) => {
                self.dispatch_resolution(cmd)
            }
            cmd @ (ActivateOwnedSubmission { .. } | Recover { .. } | Resume { .. }) => {
                self.dispatch_activation(cmd)
            }
            cmd @ (TimerFired { .. } | Cancel { .. } | Kill { .. } | Inspect { .. } | Shutdown) => {
                self.dispatch_lifecycle(cmd)
            }
        }
    }
}

impl Shard {
    fn invalid_command_group() -> RuntimeError {
        RuntimeError::UnsupportedOperation {
            operation: "internal_shard_command_group",
        }
    }
}
