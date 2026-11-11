use super::ExecutorError;
use super::issuance_test_support::PackedFixture;
use crate::admission;
use crate::compat::{CompatibilityClass, SchemaVersion};
use crate::{AcceptedArtifact, FjallJournal, JournalError};
use vb_core::artifact::ArtifactEvidence;
use vb_core::{RuntimePolicy, WorkflowDigest};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "versioned_codec_test_support.rs"]
mod support;
use support::*;

#[test]
fn versioned_codec_packed_v2_bound_and_empty_cold_reopen() -> TestResult {
    cold_roundtrip(bound_fixture(&["key"])?, SchemaVersion::new(2, 0, 0))?;
    cold_roundtrip(bound_fixture(&[])?, SchemaVersion::new(2, 0, 0))
}

#[test]
fn versioned_codec_packed_held_minor_and_patch_versions() -> TestResult {
    cold_roundtrip(PackedFixture::new()?, SchemaVersion::new(1, 2, 0))?;
    cold_roundtrip(PackedFixture::new()?, SchemaVersion::new(1, 0, 1))?;
    cold_roundtrip(bound_fixture(&["key"])?, SchemaVersion::new(2, 1, 0))?;
    cold_roundtrip(bound_fixture(&[])?, SchemaVersion::new(2, 0, 1))
}

#[test]
fn versioned_codec_packed_invalid_packs_precede_v1_stale_digest() -> TestResult {
    let fixture = PackedFixture::new()?;
    let mut accepted = fixture.prepare(RuntimePolicy::Strict)?;
    let mut parts = fixture.workflow.to_parts();
    parts.digest = WorkflowDigest::from_bytes([9; 32]);
    accepted.ir = postcard::to_allocvec(&parts)?;
    empty_packs(&mut accepted)?;
    admission::bind_artifact_digest(&mut accepted)?;
    exact_pack_mismatch(
        fixture
            .catalog
            .verify_artifact(&accepted, RuntimePolicy::Strict),
    )
}

#[test]
fn versioned_codec_packed_reference_error_precedes_packs() -> TestResult {
    let fixture = PackedFixture::new()?;
    let mut accepted = fixture.prepare(RuntimePolicy::Strict)?;
    let mut parts = fixture.workflow.to_parts();
    parts.constants = Box::from([vb_core::value::ConstValue::Symbol(vb_core::SymbolId::new(
        0,
    ))]);
    accepted.ir = postcard::to_allocvec(&parts)?;
    empty_packs(&mut accepted)?;
    admission::bind_artifact_digest(&mut accepted)?;
    match fixture
        .catalog
        .verify_artifact(&accepted, RuntimePolicy::Strict)
    {
        Err(ExecutorError::Artifact(JournalError::WorkflowReconstruction(actual))) => {
            assert_eq!(
                actual,
                vb_core::workflow::WorkflowError::SymbolOutOfBounds {
                    symbol: vb_core::SymbolId::new(0)
                }
            );
            Ok(())
        }
        other => Err(format!("unexpected reference error: {other:?}").into()),
    }
}

#[test]
fn versioned_codec_packed_future_version_precedes_bad_body_and_packs() -> TestResult {
    let fixture = PackedFixture::new()?;
    let mut accepted = fixture.prepare(RuntimePolicy::Strict)?;
    accepted.artifact_version = SchemaVersion::new(3, 0, 0);
    accepted.ir.clear();
    empty_packs(&mut accepted)?;
    admission::bind_artifact_digest(&mut accepted)?;
    match fixture
        .catalog
        .verify_artifact(&accepted, RuntimePolicy::Strict)
    {
        Err(ExecutorError::Artifact(JournalError::ArtifactVersionIncompatible {
            held,
            required,
            class,
        })) => {
            assert_eq!(held, SchemaVersion::new(3, 0, 0));
            assert_eq!(required, SchemaVersion::new(2, 0, 0));
            assert_eq!(class, CompatibilityClass::Rejected);
            Ok(())
        }
        other => Err(format!("unexpected version error: {other:?}").into()),
    }
}

#[test]
fn versioned_codec_packed_trailing_body_retains_corrupt() -> TestResult {
    let fixture = bound_fixture(&["key"])?;
    let mut accepted = fixture.prepare(RuntimePolicy::Strict)?;
    accepted.ir.push(0);
    admission::bind_artifact_digest(&mut accepted)?;
    match fixture
        .catalog
        .verify_artifact(&accepted, RuntimePolicy::Strict)
    {
        Err(ExecutorError::Corrupt) => Ok(()),
        other => Err(format!("expected Corrupt, found {other:?}").into()),
    }
}

#[test]
fn versioned_codec_packed_truncated_body_retains_codec() -> TestResult {
    let fixture = bound_fixture(&["key"])?;
    let mut accepted = fixture.prepare(RuntimePolicy::Strict)?;
    accepted.ir = vec![0x01, 0xff, 0x56, 0x42];
    admission::bind_artifact_digest(&mut accepted)?;
    match fixture
        .catalog
        .verify_artifact(&accepted, RuntimePolicy::Strict)
    {
        Err(ExecutorError::Codec(postcard::Error::DeserializeUnexpectedEnd)) => Ok(()),
        other => Err(format!("expected codec EOF, found {other:?}").into()),
    }
}
