#![forbid(unsafe_code)]
//! Reuse metadata invariant cases against the compiled production type.

use vb_core::workflow::source_metadata::{
    CompiledSecretRequirement, SecretNameRole, SourceMetadataError, WorkflowSourceMetadata,
};
use vb_core::{ids, limits};

#[path = "../../vb_core/src/workflow/source_metadata/tests.rs"]
mod source_metadata_tests;
