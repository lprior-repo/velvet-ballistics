//! Cold host presence authority. Names only; values are never loaded.
use super::Shard;
use crate::{RuntimeError, RuntimeResult};
use vb_core::CompiledWorkflow;

impl Shard {
    pub(crate) fn validate_secret_submission(
        &self,
        command: &super::ShardCommand,
    ) -> RuntimeResult<()> {
        use super::ShardCommand;
        let (run, workflow, caps, contracts) = match command {
            ShardCommand::Submit {
                run,
                workflow,
                caps,
            }
            | ShardCommand::SubmitPrePersisted {
                run,
                workflow,
                caps,
            }
            | ShardCommand::SubmitWithInputs {
                run,
                workflow,
                caps,
                ..
            } => (run, workflow, caps, None),
            ShardCommand::SubmitWithContracts {
                run,
                workflow,
                caps,
                action_contracts,
            }
            | ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow,
                caps,
                action_contracts,
                ..
            } => (run, workflow, caps, Some(action_contracts.as_ref())),
            _ => return Ok(()),
        };
        if workflow
            .source_metadata()
            .is_some_and(|metadata| !metadata.secret_requirements().is_empty())
        {
            match contracts {
                Some(contracts) => self.validate_submit_admission_with_contracts(
                    *run,
                    workflow,
                    caps.clone(),
                    contracts,
                )?,
                None => self.validate_submit_admission(*run, workflow, caps.clone())?,
            }
        }
        Ok(())
    }
    pub(crate) fn initialize_secret_frame(
        &self,
        workflow: &CompiledWorkflow,
        frame: &mut vb_core::RunFrame,
    ) -> RuntimeResult<()> {
        self.validate_secret_presence(workflow)?;
        if let Some(metadata) = workflow.source_metadata() {
            for binding in metadata.secret_requirements() {
                let Some(slot) = binding.slot() else {
                    continue;
                };
                let expected = vb_core::SlotValue::Symbol(binding.requirement());
                match frame.read_slot(slot) {
                    Ok(value)
                        if *value == expected
                            && frame.read_taint(slot)? == vb_core::Taint::Secret => {}
                    Err(vb_core::CoreError::SlotUninitialized { .. }) => {
                        frame.write_slot_with_taint(slot, expected, vb_core::Taint::Secret)?
                    }
                    _ => {
                        return Err(vb_core::CoreError::InternalInvariantViolation {
                            reason: "secret_input_binding".into(),
                        }
                        .into());
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn prepare_secret_inputs(
        &self,
        workflow: &CompiledWorkflow,
        inputs: &[(vb_core::SlotIdx, vb_core::SlotValue)],
    ) -> RuntimeResult<Vec<(vb_core::SlotIdx, vb_core::SlotValue)>> {
        self.validate_secret_presence(workflow)?;
        let mut filled = Vec::new();
        filled
            .try_reserve_exact(inputs.len())
            .map_err(|_| vb_core::CoreError::AllocationFailed)?;
        filled.extend_from_slice(inputs);
        if let Some(metadata) = workflow.source_metadata() {
            for binding in metadata.secret_requirements() {
                let Some(slot) = binding.slot() else {
                    continue;
                };
                let expected = vb_core::SlotValue::Symbol(binding.requirement());
                if let Some((_, value)) = filled.iter().find(|(candidate, _)| *candidate == slot) {
                    if *value != expected {
                        return Err(vb_core::CoreError::InternalInvariantViolation {
                            reason: "secret_input_binding".into(),
                        }
                        .into());
                    }
                } else {
                    filled
                        .try_reserve(1)
                        .map_err(|_| vb_core::CoreError::AllocationFailed)?;
                    filled.push((slot, expected));
                }
            }
        }
        Ok(filled)
    }

    /// Installs host-confirmed requirement names for subsequent admissions.
    pub fn set_available_secret_names(&mut self, names: std::sync::Arc<[Box<str>]>) {
        self.secret_availability = names;
    }

    pub(crate) fn validate_secret_presence(
        &self,
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<()> {
        if let Some(metadata) = workflow.source_metadata() {
            for binding in metadata.secret_requirements() {
                let name = metadata
                    .symbol(binding.requirement())
                    .ok_or(RuntimeError::SecretUnavailable)?;
                if !self
                    .secret_availability
                    .iter()
                    .any(|available| available.as_ref() == name)
                {
                    return Err(RuntimeError::SecretUnavailable);
                }
            }
        }
        Ok(())
    }
}
