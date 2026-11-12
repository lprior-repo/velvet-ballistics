//! Private translation from interning failures to existing compiler errors.

use crate::CompileError;
use crate::expression_bytecode::ExpressionReferenceResolver;
use crate::source_symbols::{SourceSymbolError, SourceSymbolTable};
use vb_core::workflow::source_metadata::SourceMetadataError;
use vb_core::{ExprOp, SymbolId, WorkflowError};

pub(crate) struct SymbolReferenceResolver<'a, R> {
    pub(crate) references: &'a mut R,
    pub(crate) symbols: &'a mut SourceSymbolTable,
}

impl<R: ExpressionReferenceResolver> ExpressionReferenceResolver
    for SymbolReferenceResolver<'_, R>
{
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        self.references.resolve_reference(reference)
    }

    fn intern_text(&mut self, text: &str) -> Result<SymbolId, CompileError> {
        intern_source_text(self.symbols, text)
    }
}

pub(crate) fn intern_source_text(
    symbols: &mut SourceSymbolTable,
    text: &str,
) -> Result<SymbolId, CompileError> {
    let next = symbols.len();
    symbols
        .intern(text)
        .map_err(|error| source_symbol_error(error, next))
}

pub(crate) fn source_symbol_error(error: SourceSymbolError, next: usize) -> CompileError {
    let source = match error {
        SourceSymbolError::SymbolTooLong { actual, limit } => {
            symbol_length_error(next, actual, limit)
        }
        SourceSymbolError::SymbolCountLimit { count, limit } => symbol_count_error(count, limit),
        SourceSymbolError::SymbolIndexOverflow => SourceMetadataError::SymbolIndexOverflow,
        SourceSymbolError::SymbolBytesOverflow => SourceMetadataError::SymbolBytesOverflow,
        SourceSymbolError::AllocationFailed => SourceMetadataError::ValidationAllocationFailed,
        SourceSymbolError::NonDenseSymbolIds => {
            return CompileError::ExpressionLoweringUnsupported {
                feature: "non-dense compiler symbol IDs".into(),
            };
        }
    };
    CompileError::Workflow(WorkflowError::InvalidSymbolMetadata { source })
}

fn symbol_length_error(index: usize, bytes: usize, max: usize) -> SourceMetadataError {
    match u32::try_from(index) {
        Ok(index) => SourceMetadataError::SymbolTooLong {
            id: SymbolId::new(index),
            bytes,
            max,
        },
        Err(_) => SourceMetadataError::SymbolIndexOverflow,
    }
}

fn symbol_count_error(count: usize, max: usize) -> SourceMetadataError {
    match count.checked_add(1) {
        Some(count) => SourceMetadataError::TooManySymbols { count, max },
        None => SourceMetadataError::SymbolIndexOverflow,
    }
}

#[cfg(test)]
#[path = "source_symbol_binding_tests.rs"]
mod tests;
