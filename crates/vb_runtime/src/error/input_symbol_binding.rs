use vb_core::CoreError;
use vb_core::DiagnosticCode;
use vb_core::HasSymbolicCode;
use vb_core::SymbolicCode;
use vb_core::value_store::InputSymbolBindingError;
use vb_core::workflow::source_metadata::SourceMetadataError;

pub(super) fn diagnostic_code(source: &InputSymbolBindingError) -> DiagnosticCode {
    match source {
        InputSymbolBindingError::Core(core_err) => core_err.diagnostic_code(),
        InputSymbolBindingError::Metadata(meta) => metadata_diagnostic_code(meta),
    }
}

pub(super) fn symbolic_code(source: &InputSymbolBindingError) -> SymbolicCode {
    match source {
        InputSymbolBindingError::Core(core_err) => core_err.symbolic_code(),
        InputSymbolBindingError::Metadata(meta) => metadata_symbolic_code(meta),
    }
}

pub(super) fn runtime_code(source: &InputSymbolBindingError) -> Option<&'static str> {
    match source {
        InputSymbolBindingError::Core(core_err) => core_err.runtime_code(),
        InputSymbolBindingError::Metadata(meta) => metadata_runtime_code(meta),
    }
}

fn metadata_diagnostic_code(meta: &SourceMetadataError) -> DiagnosticCode {
    match meta {
        SourceMetadataError::TooManySymbols { .. } | SourceMetadataError::SymbolTooLong { .. } => {
            CoreError::BUDGET_EXCEEDED_CODE
        }
        SourceMetadataError::SymbolIndexOverflow | SourceMetadataError::SymbolBytesOverflow => {
            CoreError::INTERNAL_INVARIANT_CODE
        }
        SourceMetadataError::ValidationAllocationFailed => CoreError::ALLOCATION_FAILED_CODE,
        SourceMetadataError::DuplicateSymbol { .. }
        | SourceMetadataError::TooManyRequirements { .. }
        | SourceMetadataError::RequirementNameOutOfBounds { .. }
        | SourceMetadataError::EmptyRequirementName { .. }
        | SourceMetadataError::RequirementsNotStrictlyOrdered { .. }
        | SourceMetadataError::SymbolCountMismatch { .. } => {
            CoreError::INVALID_COMPILED_WORKFLOW_CODE
        }
    }
}

fn metadata_symbolic_code(meta: &SourceMetadataError) -> SymbolicCode {
    match metadata_diagnostic_code(meta).symbolic_code() {
        Some(code) => code,
        None => SymbolicCode::INTERNAL_INVARIANT,
    }
}

fn metadata_runtime_code(meta: &SourceMetadataError) -> Option<&'static str> {
    match meta {
        SourceMetadataError::TooManySymbols { .. } | SourceMetadataError::SymbolTooLong { .. } => {
            Some("BUDGET_EXCEEDED")
        }
        SourceMetadataError::SymbolIndexOverflow | SourceMetadataError::SymbolBytesOverflow => {
            Some("INTERNAL_INVARIANT_VIOLATION")
        }
        SourceMetadataError::ValidationAllocationFailed => None,
        SourceMetadataError::DuplicateSymbol { .. }
        | SourceMetadataError::TooManyRequirements { .. }
        | SourceMetadataError::RequirementNameOutOfBounds { .. }
        | SourceMetadataError::EmptyRequirementName { .. }
        | SourceMetadataError::RequirementsNotStrictlyOrdered { .. }
        | SourceMetadataError::SymbolCountMismatch { .. } => Some("INVALID_COMPILED_WORKFLOW"),
    }
}

impl From<InputSymbolBindingError> for super::RuntimeError {
    fn from(source: InputSymbolBindingError) -> Self {
        Self::InputSymbolBinding {
            source: Box::new(source),
        }
    }
}
