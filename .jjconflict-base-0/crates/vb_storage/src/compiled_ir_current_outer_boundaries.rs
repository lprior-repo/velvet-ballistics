use super::super::*;
use crate::compiled_ir::decode_compiled_ir_validated;
use std::cell::Cell;

#[test]
fn reserved_escape_header_is_strict() -> TestResult {
    for bytes in [vec![1, 255], vec![1, 255, 86], vec![1, 255, 86, 67]] {
        assert_malformed(decode_compiled_ir(
            &bytes,
            Some(SchemaVersion::new(2, 0, 0)),
        ))?;
    }
    Ok(())
}

#[test]
fn reserved_escape_wire_version_is_strict() -> TestResult {
    let mut ir = bound_ir(&["key"])?;
    ir.wire_version = 3;
    assert_malformed(decode_compiled_ir(
        &wire_v2(&ir)?,
        Some(SchemaVersion::new(2, 0, 0)),
    ))
}

#[test]
fn current_outer_full_consumption() -> TestResult {
    for bytes in [
        encode_compiled_ir(&legacy(0)?)?.into_vec(),
        wire_v2(&bound_ir(&["key"])?)?,
    ] {
        let mut padded = bytes;
        padded.push(0);
        assert_malformed(decode_compiled_ir(
            &padded,
            Some(SchemaVersion::new(2, 0, 0)),
        ))?;
    }
    Ok(())
}

#[test]
fn current_outer_checksum() -> TestResult {
    assert_checksum(decode_compiled_ir(
        &postcard::to_allocvec(&parts(0))?,
        Some(SchemaVersion::new(2, 0, 0)),
    ))?;
    let mut ir = bound_ir(&["key"])?;
    ir.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    assert_checksum(decode_compiled_ir(
        &wire_v2(&ir)?,
        Some(SchemaVersion::new(2, 0, 0)),
    ))
}

fn check_outer(held: SchemaVersion) -> TestResult {
    let result = decode_compiled_ir(&[1, 255, 86, 67], Some(held));
    match result {
        Err(JournalError::ArtifactVersionIncompatible {
            held: actual,
            required,
            class,
        }) => {
            assert_eq!(actual, held);
            assert_eq!(required, SchemaVersion::new(2, 0, 0));
            assert_eq!(class, CompatibilityClass::Rejected);
            Ok(())
        }
        other => Err(unexpected(other)),
    }
}

#[test]
fn outer_authorization_precedes_corrupt_escape() -> TestResult {
    check_outer(SchemaVersion::new(1, 0, 0))?;
    check_outer(SchemaVersion::new(99, 0, 0))
}

fn check_mismatch(result: Result<(CompiledWorkflow, ()), JournalError>) -> TestResult {
    match result {
        Err(JournalError::WorkflowReconstruction(WorkflowError::InvalidSymbolMetadata {
            source,
        })) => {
            assert_eq!(
                source,
                SourceMetadataError::SymbolCountMismatch {
                    declared: 2,
                    actual: 1
                }
            );
            Ok(())
        }
        other => Err(unexpected(other)),
    }
}

#[test]
fn reconstruction_precedes_callback() -> TestResult {
    let called = Cell::new(false);
    let mut ir = bound_ir(&["key"])?;
    ir.parts.symbols_count = 2;
    let result = decode_compiled_ir_validated(
        &wire_v2(&ir)?,
        Some(SchemaVersion::new(2, 0, 0)),
        |_wf| {
            called.set(true);
            Ok(())
        },
        |error| error,
    );
    check_mismatch(result)?;
    assert_eq!(called.get(), false);
    Ok(())
}

#[derive(Debug)]
enum CallbackError {
    Denied,
    Codec(JournalError),
}

#[test]
fn callback_precedes_checksum() -> TestResult {
    let called = Cell::new(false);
    let parts_bytes = postcard::to_allocvec(&parts(0))?;
    let result = decode_compiled_ir_validated(
        &parts_bytes,
        Some(SchemaVersion::new(2, 0, 0)),
        |_wf| {
            called.set(true);
            Err::<(), CallbackError>(CallbackError::Denied)
        },
        CallbackError::Codec,
    );
    match result {
        Err(CallbackError::Denied) => {}
        other => return Err(format!("unexpected callback result: {other:?}").into()),
    }
    assert_eq!(called.get(), true);
    Ok(())
}

#[test]
fn malformed_escape_precedes_callback() -> TestResult {
    let called = Cell::new(false);
    let result = decode_compiled_ir_validated(
        &[1, 255, 86, 67],
        Some(SchemaVersion::new(2, 0, 0)),
        |_| {
            called.set(true);
            Err::<(), _>(CallbackError::Denied)
        },
        CallbackError::Codec,
    );
    match result {
        Err(CallbackError::Codec(error)) => assert_malformed::<()>(Err(error))?,
        other => return Err(format!("unexpected callback result: {other:?}").into()),
    }
    assert_eq!(called.get(), false);
    Ok(())
}
