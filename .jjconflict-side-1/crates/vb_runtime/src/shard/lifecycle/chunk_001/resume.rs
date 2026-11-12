impl Shard {
    pub fn handle_resume(&mut self, run: RunId) -> Result<ResumeResult, ResumeError> {
        self.validate_run_exists(run)?;
        if let Some(gate) = self.open_gate_for(run) {
            return Err(ResumeError::GateOpen { run_id: run, gate });
        }
        let current_state = self.get_runtime_state_or_running(run);
        if current_state == RuntimeState::Running {
            return Ok(ResumeResult {
                run_id: run,
                status: ResumeStatus::AlreadyRunning,
                timestamp: current_timestamp(),
            });
        }
        if current_state != RuntimeState::Resumable {
            return Err(ResumeError::NotResumable {
                run_id: run,
                current_state,
            });
        }
        let timestamp = self.append_resumed_event(run)?;
        let drive_result = self.drive_run(run);
        self.observe_resume_drive_result(run, drive_result)?;
        Ok(ResumeResult {
            run_id: run,
            status: ResumeStatus::Resumed,
            timestamp,
        })
    }
}
