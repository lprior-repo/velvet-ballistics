use super::{CompiledSecretRequirement, SourceMetadataError, WorkflowSourceMetadata};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct MetadataWire {
    symbols: Box<[Box<str>]>,
    secret_requirements: Box<[CompiledSecretRequirement]>,
}
impl TryFrom<MetadataWire> for WorkflowSourceMetadata {
    type Error = SourceMetadataError;
    fn try_from(value: MetadataWire) -> Result<Self, Self::Error> {
        Self::try_new(value.symbols, value.secret_requirements)
    }
}
impl From<WorkflowSourceMetadata> for MetadataWire {
    fn from(value: WorkflowSourceMetadata) -> Self {
        Self {
            symbols: value.symbols,
            secret_requirements: value.secret_requirements,
        }
    }
}
