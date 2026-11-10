use std::{collections::BTreeSet, sync::Arc};
use vb_core::{CompiledWorkflow, WorkflowDigest};
use vb_runtime::runtime::Runtime;

#[derive(Debug, Default)]
pub(crate) struct AvailableNames {
    names: BTreeSet<Box<str>>,
}

impl AvailableNames {
    pub(crate) fn install(&mut self, runtime: &mut Runtime, workflow: &CompiledWorkflow) {
        if let Some(metadata) = workflow.source_metadata() {
            for binding in metadata.secret_requirements() {
                if let Some(name) = metadata.symbol(binding.requirement())
                    && super::present(name)
                {
                    self.names.insert(name.into());
                }
            }
        }
        runtime.set_available_secret_names(
            self.names
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
    }

    pub(crate) fn install_stored(
        &mut self,
        runtime: &mut Runtime,
        journal: Arc<vb_storage::FjallJournal>,
        digest: WorkflowDigest,
    ) {
        if let Some(workflow) = super::stored_workflow(journal, digest) {
            self.install(runtime, &workflow);
        }
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
