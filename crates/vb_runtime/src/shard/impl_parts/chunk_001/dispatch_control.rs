impl Shard {
    fn dispatch_action_completion(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::ActionCompleted { ticket, output } => {
                self.handle_action_completion(ticket, output)?;
            }
            ShardCommand::RegisteredActionCompleted {
                ticket,
                receipt,
                output,
            } => {
                self.handle_registered_action_completion(ticket, receipt, output)?;
            }
            ShardCommand::ActionCompletedLegacy { run, step } => {
                self.handle_legacy_action_completion(run, step)?;
            }
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}

impl Shard {
    fn dispatch_activation(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::ActivateOwnedSubmission {
                run,
                expected_source_digest,
            } => {
                self.handle_owned_activation(run, expected_source_digest)?;
            }
            ShardCommand::Recover { command } => {
                self.handle_recover(*command)?;
            }
            ShardCommand::Resume { run } => {
                let resumed = self.handle_resume(run).map_err(RuntimeError::from)?;
                if resumed.status == crate::shard::ResumeStatus::AlreadyRunning {
                    self.drive_one_pending_run()?;
                }
            }
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}

impl Shard {
    fn dispatch_resolution(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::ActionFailed { ticket, failure } => {
                self.handle_action_failure(ticket, failure)?;
            }
            ShardCommand::RuntimeActionFailed { ticket, failure } => {
                self.handle_action_failure(ticket, failure)
                    .map_err(Self::runtime_action_failure_error)?;
            }
            ShardCommand::AskAnswered { answer } => self.handle_ask_answer(answer)?,
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}

impl Shard {
    fn dispatch_lifecycle(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::TimerFired {
                run,
                generation,
                deadline,
                kind,
            } => self.handle_timer(run, generation, deadline, kind)?,
            ShardCommand::Cancel { run, reason } => self.handle_cancel(run, reason)?,
            ShardCommand::Kill { run, reason } => self.handle_kill(run, reason)?,
            ShardCommand::Inspect { run, correlation } => {
                self.handle_inspect(run, correlation);
            }
            ShardCommand::Shutdown => {
                self.shutting_down = true;
                return Ok(false);
            }
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}
