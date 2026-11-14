impl Shard {
    fn dispatch_legacy_gate(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::OpenGate {
                gate,
                opened_at_ms,
                requester,
            } => {
                self.open_gate(gate, opened_at_ms, requester)?;
            }
            ShardCommand::SubmitGateDecision { run, input } => {
                self.submit_gate_decision(run, input)?;
            }
            ShardCommand::EscalateGate { run, gate, now_ms } => {
                self.escalate_timed_out_gate(run, gate, now_ms)?;
            }
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}

impl Shard {
    fn dispatch_bound_gate(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::OpenBoundGate {
                gate,
                opened_at_ms,
                requester,
            } => {
                self.open_bound_gate(gate, opened_at_ms, requester)?;
            }
            ShardCommand::SubmitBoundGateDecision {
                run,
                input,
                operator_trust,
            } => {
                self.submit_bound_gate_decision(run, input, operator_trust)?;
            }
            ShardCommand::EscalateBoundGate { run, gate, now_ms } => {
                self.escalate_timed_out_bound_gate(run, gate, now_ms)?;
            }
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}
