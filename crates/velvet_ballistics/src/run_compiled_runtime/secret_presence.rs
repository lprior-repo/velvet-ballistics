//! Cold host presence checks. Only declared names cross into the runtime.
mod registry;
pub(crate) use registry::AvailableNames;

pub(crate) fn check(workflow: &vb_core::CompiledWorkflow) -> Result<(), vb_runtime::RuntimeError> {
    if let Some(metadata) = workflow.source_metadata() {
        for binding in metadata.secret_requirements() {
            if !metadata.symbol(binding.requirement()).is_some_and(present) {
                return Err(vb_runtime::RuntimeError::SecretUnavailable);
            }
        }
    }
    Ok(())
}

pub(super) fn install(
    runtime: &mut vb_runtime::runtime::Runtime,
    workflow: &vb_core::CompiledWorkflow,
) {
    let mut available = Vec::new();
    if let Some(metadata) = workflow.source_metadata() {
        for binding in metadata.secret_requirements() {
            if let Some(name) = metadata.symbol(binding.requirement())
                && present(name)
            {
                available.push(Box::<str>::from(name));
            }
        }
    }
    runtime.set_available_secret_names(available.into_boxed_slice());
}

pub(crate) fn install_stored(
    runtime: &mut vb_runtime::runtime::Runtime,
    journal: std::sync::Arc<vb_storage::FjallJournal>,
    digest: vb_core::WorkflowDigest,
) {
    // Recovery retains authority over errors; a failed load must not mask its diagnostic.
    if let Some(workflow) = stored_workflow(journal, digest) {
        install(runtime, &workflow);
    }
}

fn stored_workflow(
    journal: std::sync::Arc<vb_storage::FjallJournal>,
    digest: vb_core::WorkflowDigest,
) -> Option<vb_core::CompiledWorkflow> {
    use vb_runtime::admission::AcceptedArtifactStore;
    let store = vb_runtime::admission::StorageArtifactStore::new(journal);
    let artifact = store.load_accepted_artifact(digest).ok()?;
    vb_storage::admission::verify_accepted_artifact(&artifact).ok()
}

fn present(key: &str) -> bool {
    !key.is_empty()
        && !key.as_bytes().contains(&0)
        && !key.contains('=')
        && std::env::var_os(key).is_some()
}

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_host_keys_are_absent_without_calling_the_environment() {
        for key in ["", "bad=key", "bad\0key"] {
            assert!(!super::present(key), "invalid key must not reach var_os");
        }
    }
}
