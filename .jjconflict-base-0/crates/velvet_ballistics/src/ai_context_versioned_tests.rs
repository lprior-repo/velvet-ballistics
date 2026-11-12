use super::{DecodeCompiledWorkflowError, decode_compiled_workflow_from_ir};
use vb_storage::compat::SchemaVersion;

use crate::compile::versioned_test_support as support;
use support::TestResult;

#[test]
fn versioned_context_reader_retains_raw_payloads() -> TestResult {
    let expected = support::bound(&["key", "é", " "])?;
    let bytes = vb_compile::encode_compiled_ir(&expected)?;
    let found = decode_compiled_workflow_from_ir(&bytes).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        found.symbol_payloads(),
        Some(support::symbols(&["key", "é", " "]).as_ref())
    );
    assert_eq!(found, expected);
    Ok(())
}

#[test]
fn versioned_context_reader_keeps_v1_minor() -> TestResult {
    let artifact = support::accepted(SchemaVersion::new(1, 2, 0))?;
    let found = decode_compiled_workflow_from_ir(&postcard::to_allocvec(&artifact)?)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(found.symbol_payloads(), None);
    assert_eq!(found.digest(), artifact.source_digest);
    Ok(())
}

#[test]
fn versioned_context_reader_rejects_future_outer() -> TestResult {
    let artifact = support::accepted(SchemaVersion::new(3, 0, 0))?;
    assert_eq!(
        decode_compiled_workflow_from_ir(&postcard::to_allocvec(&artifact)?),
        Err(DecodeCompiledWorkflowError::AcceptedArtifactWorkflowPartsDecode)
    );
    Ok(())
}

#[test]
fn versioned_context_reader_accepted_empty_binding() -> TestResult {
    let expected = support::bound(&[])?;
    let artifact = support::accepted_v2(&expected)?;
    let found = decode_compiled_workflow_from_ir(&postcard::to_allocvec(&artifact)?)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        found.symbol_payloads(),
        Some(support::symbols(&[]).as_ref())
    );
    assert_eq!(found, expected);
    Ok(())
}

#[test]
fn versioned_context_reader_accepted_ordered_payloads() -> TestResult {
    let expected = support::bound(&["key", "é", " "])?;
    let artifact = support::accepted_v2(&expected)?;
    let found = decode_compiled_workflow_from_ir(&postcard::to_allocvec(&artifact)?)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        found.symbol_payloads(),
        Some(support::symbols(&["key", "é", " "]).as_ref())
    );
    assert_eq!(found, expected);
    Ok(())
}
