#![forbid(unsafe_code)]

use vb_core::workflow::source_metadata::{
    CompiledSecretRequirement, SecretNameRole, SourceMetadataError, WorkflowSourceMetadata,
};
use vb_core::{SlotIdx, SymbolId};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn given_duplicate_symbol_payloads_then_metadata_rejects_exact_ids() -> TestResult {
    let symbols = Box::from([Box::from("same"), Box::from("same")]);
    let requirements = Box::from([]);
    let result = WorkflowSourceMetadata::try_new(symbols, requirements);
    assert_eq!(
        result,
        Err(SourceMetadataError::DuplicateSymbol {
            first: SymbolId::new(0),
            duplicate: SymbolId::new(1)
        })
    );
    Ok(())
}

#[test]
fn given_missing_requirement_payload_then_rejects_exact_role_and_id() -> TestResult {
    let symbols = Box::from([Box::from("token")]);
    let requirement =
        CompiledSecretRequirement::with_slot(SymbolId::new(0), SymbolId::new(1), SlotIdx::new(0));
    let requirements = Box::from([requirement]);
    let result = WorkflowSourceMetadata::try_new(symbols, requirements);
    assert_eq!(
        result,
        Err(SourceMetadataError::RequirementNameOutOfBounds {
            role: SecretNameRole::Requirement,
            name: SymbolId::new(1)
        })
    );
    Ok(())
}

#[test]
fn given_valid_metadata_then_preserves_literal_payload_order_and_secret_binding() -> TestResult {
    let symbols = Box::from([Box::from("token"), Box::from("TOKEN")]);
    let requirement =
        CompiledSecretRequirement::with_slot(SymbolId::new(0), SymbolId::new(1), SlotIdx::new(0));
    let requirements = Box::from([requirement]);
    let metadata = WorkflowSourceMetadata::try_new(symbols, requirements)?;
    let expected_symbols: [Box<str>; 2] = [Box::from("token"), Box::from("TOKEN")];
    assert_eq!(metadata.symbols(), &expected_symbols);
    assert_eq!(metadata.symbols_count(), 2);
    let expected_requirements = [CompiledSecretRequirement::with_slot(
        SymbolId::new(0),
        SymbolId::new(1),
        SlotIdx::new(0),
    )];
    assert_eq!(metadata.secret_requirements(), &expected_requirements);
    Ok(())
}
