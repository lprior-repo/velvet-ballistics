use super::*;

fn metadata(source: SourceMetadataError) -> CompileError {
    CompileError::Workflow(WorkflowError::InvalidSymbolMetadata { source })
}

#[test]
fn non_dense_ids_fail_with_precise_unsupported_invariant() {
    assert_eq!(
        source_symbol_error(SourceSymbolError::NonDenseSymbolIds, 0),
        CompileError::ExpressionLoweringUnsupported {
            feature: "non-dense compiler symbol IDs".into(),
        },
    );
}

#[test]
fn overlong_payload_retains_next_id_and_byte_limits() {
    assert_eq!(
        source_symbol_error(
            SourceSymbolError::SymbolTooLong {
                actual: 4097,
                limit: 4096
            },
            7
        ),
        metadata(SourceMetadataError::SymbolTooLong {
            id: SymbolId::new(7),
            bytes: 4097,
            max: 4096,
        }),
    );
}

#[test]
fn count_limit_reports_the_attempted_new_unique_count() {
    assert_eq!(
        source_symbol_error(
            SourceSymbolError::SymbolCountLimit {
                count: 1_000_000,
                limit: 1_000_000,
            },
            1_000_000
        ),
        metadata(SourceMetadataError::TooManySymbols {
            count: 1_000_001,
            max: 1_000_000
        }),
    );
}

#[test]
fn attempted_count_overflow_remains_index_overflow() {
    assert_eq!(
        source_symbol_error(
            SourceSymbolError::SymbolCountLimit {
                count: usize::MAX,
                limit: 1_000_000,
            },
            0
        ),
        metadata(SourceMetadataError::SymbolIndexOverflow),
    );
}

#[test]
fn unrepresentable_next_id_remains_index_overflow() {
    assert_eq!(
        source_symbol_error(
            SourceSymbolError::SymbolTooLong {
                actual: 4097,
                limit: 4096
            },
            usize::MAX
        ),
        metadata(SourceMetadataError::SymbolIndexOverflow),
    );
}

#[test]
fn explicit_index_overflow_retains_its_metadata_error() {
    assert_eq!(
        source_symbol_error(SourceSymbolError::SymbolIndexOverflow, 0),
        metadata(SourceMetadataError::SymbolIndexOverflow),
    );
}

#[test]
fn byte_accounting_overflow_retains_its_metadata_error() {
    assert_eq!(
        source_symbol_error(SourceSymbolError::SymbolBytesOverflow, 0),
        metadata(SourceMetadataError::SymbolBytesOverflow),
    );
}

#[test]
fn reservation_failure_retains_the_metadata_allocation_error() {
    assert_eq!(
        source_symbol_error(SourceSymbolError::AllocationFailed, 0),
        metadata(SourceMetadataError::ValidationAllocationFailed),
    );
}
