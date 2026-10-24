use super::{
    CompiledSecretRequirement, SecretNameRole, SourceMetadataError, WorkflowSourceMetadata,
};
use crate::ids::SymbolId;
use crate::limits::{MAX_SYMBOL_BYTES_PER_VALUE, MAX_VALUES_PER_RUN};

fn symbols(values: &[&str]) -> Box<[Box<str>]> {
    values
        .iter()
        .map(|value| Box::<str>::from(*value))
        .collect()
}

fn binding(alias: u32, requirement: u32) -> CompiledSecretRequirement {
    CompiledSecretRequirement::new(SymbolId::new(alias), SymbolId::new(requirement))
}

#[test]
fn empty_metadata_has_no_resolvable_identifiers() -> Result<(), SourceMetadataError> {
    let metadata = WorkflowSourceMetadata::try_new(Box::default(), Box::default())?;
    assert_eq!(metadata, WorkflowSourceMetadata::empty());
    assert_eq!(metadata.symbols_count(), 0);
    assert_eq!(metadata.symbol_bytes(), 0);
    assert!(metadata.symbols().is_empty());
    assert!(metadata.secret_requirements().is_empty());
    assert_eq!(metadata.symbol(SymbolId::new(0)), None);
    assert_eq!(metadata.symbol(SymbolId::new(u32::MAX)), None);
    metadata.validate_symbols_count(0)
}

#[test]
fn validation_preserves_identifier_order_and_exact_bytes() -> Result<(), SourceMetadataError> {
    let metadata = WorkflowSourceMetadata::try_new(
        symbols(&["zebra", "", "alpha", "é", "e\u{301}"]),
        Box::default(),
    )?;
    assert_eq!(
        metadata.symbols(),
        symbols(&["zebra", "", "alpha", "é", "e\u{301}"]).as_ref()
    );
    assert_eq!(metadata.symbol(SymbolId::new(0)), Some("zebra"));
    assert_eq!(metadata.symbol(SymbolId::new(1)), Some(""));
    assert_eq!(metadata.symbol(SymbolId::new(4)), Some("e\u{301}"));
    assert_eq!(metadata.symbol(SymbolId::new(5)), None);
    assert_eq!(metadata.symbol(SymbolId::new(u32::MAX)), None);
    assert_eq!(metadata.symbols_count(), 5);
    assert_eq!(metadata.symbol_bytes(), 15);
    assert_eq!(metadata.clone(), metadata);
    metadata.validate_symbols_count(5)
}

#[test]
fn declared_count_must_match_payload_count() -> Result<(), SourceMetadataError> {
    let metadata = WorkflowSourceMetadata::try_new(symbols(&["x"]), Box::default())?;
    assert_eq!(
        metadata.validate_symbols_count(0),
        Err(SourceMetadataError::SymbolCountMismatch {
            declared: 0,
            actual: 1
        })
    );
    metadata.validate_symbols_count(1)
}

#[test]
fn duplicate_payloads_report_original_identifiers() {
    let result = WorkflowSourceMetadata::try_new(symbols(&["z", "a", "z"]), Box::default());
    assert_eq!(
        result,
        Err(SourceMetadataError::DuplicateSymbol {
            first: SymbolId::new(0),
            duplicate: SymbolId::new(2),
        })
    );
    assert!(matches!(
        WorkflowSourceMetadata::try_new(symbols(&["", ""]), Box::default()),
        Err(SourceMetadataError::DuplicateSymbol { .. })
    ));
}

#[test]
fn symbol_limit_counts_utf8_bytes() -> Result<(), SourceMetadataError> {
    let at_limit = "é".repeat(2_048).into_boxed_str();
    let metadata =
        WorkflowSourceMetadata::try_new(vec![at_limit].into_boxed_slice(), Box::default())?;
    assert_eq!(metadata.symbol_bytes(), 4_096);
    let over_limit = "x".repeat(4_097).into_boxed_str();
    assert_eq!(
        WorkflowSourceMetadata::try_new(vec![over_limit].into_boxed_slice(), Box::default()),
        Err(SourceMetadataError::SymbolTooLong {
            id: SymbolId::new(0),
            bytes: 4_097,
            max: MAX_SYMBOL_BYTES_PER_VALUE,
        })
    );
    let over_utf8_limit = format!("{}x", "é".repeat(2_048)).into_boxed_str();
    assert_eq!(
        WorkflowSourceMetadata::try_new(vec![over_utf8_limit].into_boxed_slice(), Box::default()),
        Err(SourceMetadataError::SymbolTooLong {
            id: SymbolId::new(0),
            bytes: 4_097,
            max: MAX_SYMBOL_BYTES_PER_VALUE,
        })
    );
    Ok(())
}

#[test]
fn no_one_mebibyte_aggregate_ceiling_is_introduced() -> Result<(), SourceMetadataError> {
    let payloads: Box<[Box<str>]> = (0..257)
        .map(|index| format!("{index:08}{}", "x".repeat(4_088)).into_boxed_str())
        .collect();
    let metadata = WorkflowSourceMetadata::try_new(payloads, Box::default())?;
    assert_eq!(metadata.symbols_count(), 257);
    assert_eq!(metadata.symbol_bytes(), 1_052_672);
    Ok(())
}

#[test]
fn existing_outer_arena_count_boundary_is_accepted() -> Result<(), SourceMetadataError> {
    let payloads: Box<[Box<str>]> = (0..MAX_VALUES_PER_RUN)
        .map(|index| index.to_string().into_boxed_str())
        .collect();
    let metadata = WorkflowSourceMetadata::try_new(payloads, Box::default())?;
    assert_eq!(metadata.symbols_count(), 1_000_000);
    assert_eq!(metadata.symbol(SymbolId::new(999_999)), Some("999999"));
    Ok(())
}

#[test]
fn oversized_table_is_rejected_before_duplicate_validation() {
    let payloads = vec![Box::<str>::from(""); 1_000_001].into_boxed_slice();
    assert_eq!(
        WorkflowSourceMetadata::try_new(payloads, Box::default()),
        Err(SourceMetadataError::TooManySymbols {
            count: 1_000_001,
            max: MAX_VALUES_PER_RUN,
        })
    );
}

#[test]
fn source_alias_and_requirement_target_are_both_retained() -> Result<(), SourceMetadataError> {
    let requirements = vec![binding(0, 2), binding(1, 2)].into_boxed_slice();
    let metadata = WorkflowSourceMetadata::try_new(
        symbols(&["token", "other_token", "TOKEN"]),
        requirements.clone(),
    )?;
    assert_eq!(metadata.secret_requirements(), requirements.as_ref());
    let first = binding(0, 2);
    assert_eq!(metadata.symbol(first.alias()), Some("token"));
    assert_eq!(metadata.symbol(first.requirement()), Some("TOKEN"));
    let changed = WorkflowSourceMetadata::try_new(
        symbols(&["token", "other_token", "TOKEN"]),
        vec![binding(0, 1), binding(1, 2)].into_boxed_slice(),
    )?;
    assert_ne!(metadata, changed);
    Ok(())
}

#[test]
fn alias_and_requirement_may_share_one_nonempty_payload() -> Result<(), SourceMetadataError> {
    let metadata = WorkflowSourceMetadata::try_new(
        symbols(&["TOKEN"]),
        vec![binding(0, 0)].into_boxed_slice(),
    )?;
    assert_eq!(metadata.secret_requirements(), &[binding(0, 0)]);
    Ok(())
}

#[test]
fn both_binding_sides_are_checked_for_bounds_and_empty_names() {
    for (requirement, role) in [
        (binding(u32::MAX, 0), SecretNameRole::Alias),
        (binding(0, u32::MAX), SecretNameRole::Requirement),
    ] {
        assert_eq!(
            WorkflowSourceMetadata::try_new(
                symbols(&["TOKEN"]),
                vec![requirement].into_boxed_slice(),
            ),
            Err(SourceMetadataError::RequirementNameOutOfBounds {
                role,
                name: SymbolId::new(u32::MAX),
            })
        );
    }
    for (requirement, role) in [
        (binding(1, 0), SecretNameRole::Alias),
        (binding(0, 1), SecretNameRole::Requirement),
    ] {
        assert_eq!(
            WorkflowSourceMetadata::try_new(
                symbols(&["TOKEN", ""]),
                vec![requirement].into_boxed_slice(),
            ),
            Err(SourceMetadataError::EmptyRequirementName {
                role,
                name: SymbolId::new(1)
            })
        );
    }
}

#[test]
fn duplicate_and_descending_aliases_are_rejected() {
    for requirements in [
        vec![binding(0, 1), binding(0, 2)],
        vec![binding(1, 2), binding(0, 2)],
    ] {
        assert!(matches!(
            WorkflowSourceMetadata::try_new(
                symbols(&["a", "b", "TOKEN"]),
                requirements.into_boxed_slice(),
            ),
            Err(SourceMetadataError::RequirementsNotStrictlyOrdered { .. })
        ));
    }
}

#[test]
fn impossible_requirement_count_is_rejected_before_traversal() {
    assert_eq!(
        WorkflowSourceMetadata::try_new(Box::default(), vec![binding(0, 0)].into_boxed_slice()),
        Err(SourceMetadataError::TooManyRequirements {
            count: 1,
            symbols: 0
        })
    );
}
