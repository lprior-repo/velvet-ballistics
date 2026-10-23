#![forbid(unsafe_code)]
//! Compile the actual unregistered metadata module for native boundary checks.
//! This target does not enable workflow, artifact, or runtime metadata wiring.

use vb_core::{ids, limits};

#[path = "../../vb_core/src/workflow/source_metadata.rs"]
mod source_metadata;
