impl Shard {
    fn check_bound_binding(
        state: &crate::shard::types::RunState,
        gate: &BoundManualGate,
    ) -> RuntimeResult<()> {
        let binding = gate.binding;
        let admission = state
            .admission
            .as_ref()
            .ok_or_else(|| binding_error(gate, "admission"))?;
        if admission.artifact_digest().as_bytes() != binding.artifact_digest.as_bytes() {
            return Err(binding_error(gate, "artifact_digest"));
        }
        if state.run_attempt.get() != binding.attempt {
            return Err(binding_error(gate, "attempt"));
        }
        match admission.policy_pin() {
            vb_storage::RunPolicyPin::Pinned(policy_digest)
                if policy_digest.as_bytes() == binding.policy_digest.as_bytes() => {}
            vb_storage::RunPolicyPin::Unverified => {
                return Err(binding_error(gate, "policy_digest"));
            }
            _ => return Err(binding_error(gate, "policy_digest")),
        }
        if admission.input_digest()
            != vb_storage::RunInputBinding::Digest(vb_core::ids::WorkflowDigest::from_bytes(
                binding.input_digest.as_bytes(),
            ))
        {
            return Err(binding_error(gate, "input_digest"));
        }
        Ok(())
    }

    fn verify_bound_gate_authority(
        record: &BoundGateOutcomeRecord,
        gate: &BoundManualGate,
        opening_seq: u64,
    ) -> RuntimeResult<()> {
        let mismatch = || RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::BoundReceiptMismatch,
        };
        if !matches!(
            record.record.outcome,
            vb_core::manual_gate::GateOutcome::Approved
                | vb_core::manual_gate::GateOutcome::BreakGlassApproved
        ) {
            return Ok(());
        }
        let receipt = record
            .bound_receipt
            .as_ref()
            .filter(|receipt| receipt.opening_seq == opening_seq)
            .ok_or_else(mismatch)?;
        match crate::manual_gate::verify_bound_receipt(receipt, gate) {
            Ok(true) => Ok(()),
            Ok(false) => Err(mismatch()),
            Err(source) => Err(RuntimeError::GateEvaluationFailed { source }),
        }
    }
}
