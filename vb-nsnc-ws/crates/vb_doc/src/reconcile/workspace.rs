use std::path::Path;

use crate::{DocReconcileError, PreservedNonGoal};

pub(super) fn validate_workspace(
    path: &Path,
    policy_root: &Path,
    master_doc_file: &str,
) -> Result<(), DocReconcileError> {
    if !path.starts_with(policy_root) || !is_master_doc_path(path, master_doc_file) {
        Err(DocReconcileError::WrongWorkspace {
            path: path.to_path_buf(),
        })
    } else {
        Ok(())
    }
}

pub(super) fn preserved_non_goals(text: &str) -> Vec<PreservedNonGoal> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("does not track control-flow taint") {
        vec![PreservedNonGoal::ControlFlowTaintV1NonGoal]
    } else {
        Vec::new()
    }
}

fn is_master_doc_path(path: &Path, master_doc_file: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == master_doc_file)
}
