use super::*;
use crate::compat::CompatibilityClass;
use vb_core::workflow::{WorkflowError, source_metadata::SourceMetadataError};

#[test]
fn versioned_codec_future_major_precedes_undecodable_ir() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    received.artifact_version = SchemaVersion::new(3, 0, 0);
    replace_ir(&mut received, vec![])?;
    assert_error(&received, |error| match error {
        JournalError::ArtifactVersionIncompatible {
            held,
            required,
            class,
        } => {
            assert_eq!(held, SchemaVersion::new(3, 0, 0));
            assert_eq!(required, SchemaVersion::new(2, 0, 0));
            assert_eq!(class, CompatibilityClass::Rejected);
            Ok(())
        }
        other => Err(format!("unexpected version error: {other:?}").into()),
    })
}

#[test]
fn versioned_codec_both_outer_wire_disagreements_fail() -> TestResult {
    let mut v1 = artifact(&legacy(0)?)?;
    replace_ir(&mut v1, v2_wire(&bound_ir(&[])?)?)?;
    assert_error(&v1, malformed)?;
    let mut v2 = artifact(&bound(&[])?)?;
    replace_ir(&mut v2, postcard::to_allocvec(&legacy(0)?.to_parts())?)?;
    assert_error(&v2, malformed)
}

#[test]
fn versioned_codec_bad_inner_version_fails() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    let mut ir = bound_ir(&["key"])?;
    ir.wire_version = 3;
    replace_ir(&mut received, v2_wire(&ir)?)?;
    assert_error(&received, malformed)
}

#[test]
fn versioned_codec_truncated_v2_retains_eof() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    replace_ir(&mut received, vec![0x01, 0xff, 0x56, 0x42])?;
    assert_error(&received, |error| match error {
        JournalError::PostcardDecodeFailed(postcard::Error::DeserializeUnexpectedEnd) => Ok(()),
        other => Err(format!("unexpected truncation error: {other:?}").into()),
    })
}

#[test]
fn versioned_codec_trailing_v2_fails() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    let mut bytes = received.ir.clone();
    bytes.push(0);
    replace_ir(&mut received, bytes)?;
    assert_error(&received, malformed)
}

#[test]
fn versioned_codec_count_error_precedes_stale_digest() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    let mut ir = bound_ir(&["key"])?;
    ir.parts.symbols_count = 2;
    replace_ir(&mut received, v2_wire(&ir)?)?;
    assert_error(&received, |error| match error {
        JournalError::WorkflowReconstruction(actual) => {
            assert_eq!(
                actual,
                WorkflowError::InvalidSymbolMetadata {
                    source: SourceMetadataError::SymbolCountMismatch {
                        declared: 2,
                        actual: 1
                    },
                }
            );
            Ok(())
        }
        other => Err(format!("unexpected metadata error: {other:?}").into()),
    })
}

#[test]
fn versioned_codec_reference_error_precedes_stale_digest() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    let mut ir = bound_ir(&["key"])?;
    ir.parts.constants = Box::from([vb_core::value::ConstValue::Symbol(vb_core::SymbolId::new(
        1,
    ))]);
    replace_ir(&mut received, v2_wire(&ir)?)?;
    assert_error(&received, |error| match error {
        JournalError::WorkflowReconstruction(actual) => {
            assert_eq!(
                actual,
                WorkflowError::SymbolOutOfBounds {
                    symbol: vb_core::SymbolId::new(1)
                }
            );
            Ok(())
        }
        other => Err(format!("unexpected reference error: {other:?}").into()),
    })
}

#[test]
fn versioned_codec_stale_inner_digest_is_not_accepted() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    let mut ir = bound_ir(&["key"])?;
    ir.symbol_payloads = Box::from([Box::from("new")]);
    replace_ir(&mut received, v2_wire(&ir)?)?;
    assert_error(&received, checksum)
}

#[test]
fn versioned_codec_outer_checksum_precedes_version_and_body() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    received.artifact_version = SchemaVersion::new(3, 0, 0);
    received.ir.clear();
    assert_error(&received, checksum)
}

#[test]
fn versioned_codec_evidence_schema_precedes_checksum_and_body() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    let vb_core::artifact::ArtifactEvidence::Verified(evidence) =
        &mut received.verification.evidence
    else {
        return Err("expected verified evidence".into());
    };
    evidence.schema_version = 99;
    received.ir.clear();
    assert_error(&received, |error| match error {
        JournalError::UnsupportedSchemaVersion { version } => {
            assert_eq!(version, 99);
            Ok(())
        }
        other => Err(format!("unexpected schema error: {other:?}").into()),
    })
}

#[test]
fn versioned_codec_policy_mismatch_precedes_body() -> TestResult {
    let mut received = artifact(&bound(&["key"])?)?;
    replace_ir(&mut received, vec![])?;
    match verify_accepted_artifact_for_policy(&received, RuntimePolicy::Relaxed) {
        Err(error) => malformed(error),
        Ok(workflow) => Err(format!("unexpected workflow: {workflow:?}").into()),
    }
}
