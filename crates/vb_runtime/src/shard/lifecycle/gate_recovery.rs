// Recovered gate restoration separate from general recovery flow.

impl Shard {
    /// Restores reconstructed open gates before any boundary handling.
    ///
    /// A gated run is suspended on its gate, never on another boundary: a
    /// recovered gate together with a pending-action or open-ask boundary is
    /// inconsistent journal evidence, so recovery fails instead of driving a
    /// gated run or dropping the gate. The run stays suspended (AwaitAction)
    /// and is not driven; the gate decision resumes or fails it through the
    /// normal submission flow.
    pub(crate) fn restore_recovered_gates(
        &mut self,
        run: RunId,
        open_gates: &[crate::shard::types::OpenGate],
        bound_open_gates: &[crate::shard::types::BoundOpenGate],
        boundary_kind: crate::recovery::RecoveredRunBoundaryKind,
    ) -> RuntimeResult<bool> {
        if open_gates.is_empty() && bound_open_gates.is_empty() {
            return Ok(false);
        }
        if boundary_kind != crate::recovery::RecoveredRunBoundaryKind::None {
            return Err(RuntimeError::Recovery {
                error: "recovered run has both an open gate and a suspended boundary".to_string(),
            });
        }
        if !open_gates.is_empty() && !bound_open_gates.is_empty() {
            return Err(RuntimeError::Recovery {
                error: "recovered run has both legacy and bound open gates".to_string(),
            });
        }
        if open_gates.is_empty() {
            self.restore_bound_open_gates(run, bound_open_gates)?;
        } else {
            self.restore_open_gates(run, open_gates)?;
        }
        self.apply(run, crate::shard::types::RuntimeEvent::AwaitAction)?;
        Ok(true)
    }
}
