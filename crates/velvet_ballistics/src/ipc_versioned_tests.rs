use super::StorageWorkflowResolver;
use std::sync::Arc;
use vb_core::CompiledWorkflow;
use vb_ipc::server::{WorkflowResolutionError, WorkflowResolver};
use vb_storage::compat::{CompatibilityClass, SchemaVersion};

use crate::compile::versioned_test_support as support;
use support::TestResult;

fn resolve(
    record: vb_storage::CompiledIrRecord,
) -> TestResult<Result<CompiledWorkflow, WorkflowResolutionError>> {
    let dir = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    journal.put_compiled_ir(&record)?;
    let mut resolver = StorageWorkflowResolver { journal };
    Ok(resolver.resolve_workflow(record.digest))
}

#[test]
fn versioned_resolver_preserves_empty_binding() -> TestResult {
    let expected = support::bound(&[])?;
    let found = resolve(support::raw_record(&expected)?)??;
    assert_eq!(
        found.symbol_payloads(),
        Some(support::symbols(&[]).as_ref())
    );
    assert_eq!(found, expected);
    Ok(())
}

#[test]
fn versioned_resolver_preserves_ordered_payloads() -> TestResult {
    let expected = support::bound(&["key", "é", " "])?;
    let found = resolve(support::raw_record(&expected)?)??;
    assert_eq!(
        found.symbol_payloads(),
        Some(support::symbols(&["key", "é", " "]).as_ref())
    );
    assert_eq!(found, expected);
    Ok(())
}

#[test]
fn versioned_resolver_keeps_supported_v1_minor() -> TestResult {
    let artifact = support::accepted(SchemaVersion::new(1, 2, 0))?;
    let found = resolve(support::accepted_record(&artifact)?)??;
    assert_eq!(found.symbol_payloads(), None);
    assert_eq!(found.digest(), artifact.source_digest);
    Ok(())
}

#[test]
fn versioned_resolver_rejects_future_outer_version() -> TestResult {
    let artifact = support::accepted(SchemaVersion::new(3, 0, 0))?;
    match vb_storage::decode_compiled_ir(&artifact.ir, Some(artifact.artifact_version)) {
        Err(vb_storage::JournalError::ArtifactVersionIncompatible {
            held,
            required,
            class,
        }) => {
            assert_eq!(held, SchemaVersion::new(3, 0, 0));
            assert_eq!(required, SchemaVersion::new(2, 0, 0));
            assert_eq!(class, CompatibilityClass::Rejected);
        }
        other => return Err(format!("unexpected codec result: {other:?}").into()),
    }
    assert_eq!(
        resolve(support::accepted_record(&artifact)?)?,
        Err(WorkflowResolutionError::InvalidArtifact)
    );
    Ok(())
}

#[test]
fn versioned_resolver_accepted_empty_binding() -> TestResult {
    let expected = support::bound(&[])?;
    let artifact = support::accepted_v2(&expected)?;
    let found = resolve(support::accepted_record(&artifact)?)??;
    assert_eq!(
        found.symbol_payloads(),
        Some(support::symbols(&[]).as_ref())
    );
    assert_eq!(found, expected);
    Ok(())
}

#[test]
fn versioned_resolver_accepted_ordered_payloads() -> TestResult {
    let expected = support::bound(&["key", "é", " "])?;
    let artifact = support::accepted_v2(&expected)?;
    let found = resolve(support::accepted_record(&artifact)?)??;
    assert_eq!(
        found.symbol_payloads(),
        Some(support::symbols(&["key", "é", " "]).as_ref())
    );
    assert_eq!(found, expected);
    Ok(())
}
