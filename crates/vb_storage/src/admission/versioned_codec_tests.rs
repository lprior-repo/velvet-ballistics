use super::*;
use crate::compat::SchemaVersion;
use vb_core::{CompiledWorkflow, RuntimePolicy, WorkflowDigest};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "versioned_codec_test_support.rs"]
mod support;
use support::*;

fn roundtrip(workflow: CompiledWorkflow, wire: Vec<u8>, held: SchemaVersion) -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = crate::FjallJournal::open(dir.path(), None)?;
    let raw = admit_compiled_artifact(&journal, &workflow)?;
    assert_eq!(raw, WorkflowDigest::from_bytes(blake3::hash(&wire).into()));
    assert_eq!(journal.compiled_ir(raw)?.ok_or("missing raw IR")?.ir, wire);
    let accepted = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)?;
    assert_eq!(accepted.accepted_at_seq, EventSeq::new(1));
    assert_eq!(accepted.ir, wire);
    let mut received = accepted;
    received.artifact_version = held;
    bind_artifact_digest(&mut received)?;
    journal.put_compiled_ir(&CompiledIrRecord {
        digest: received.digest,
        ir: postcard::to_allocvec(&received)?,
    })?;
    journal.persist_strict()?;
    assert_eq!(verify_accepted_artifact(&received)?, workflow);
    read(&journal, &received)?;
    drop(journal);
    let reopened = crate::FjallJournal::open(dir.path(), None)?;
    read(&reopened, &received)?;
    assert_eq!(verify_accepted_artifact(&received)?, workflow);
    Ok(())
}

#[test]
fn versioned_codec_v1_zero_symbol_cold_roundtrip() -> TestResult {
    let workflow = legacy(0)?;
    assert_eq!(
        artifact(&workflow)?.artifact_version,
        SchemaVersion::new(1, 0, 0)
    );
    roundtrip(
        workflow.clone(),
        postcard::to_allocvec(&workflow.to_parts())?,
        SchemaVersion::new(1, 0, 0),
    )
}

#[test]
fn versioned_codec_v1_nonzero_symbols_preserve_raw_wire() -> TestResult {
    let workflow = legacy(1)?;
    roundtrip(
        workflow.clone(),
        postcard::to_allocvec(&workflow.to_parts())?,
        SchemaVersion::new(1, 0, 0),
    )
}

#[test]
fn versioned_codec_v2_bound_payload_cold_roundtrip() -> TestResult {
    let workflow = bound(&["key", "é"])?;
    assert_eq!(
        artifact(&workflow)?.artifact_version,
        SchemaVersion::new(2, 0, 0)
    );
    roundtrip(
        workflow,
        v2_wire(&bound_ir(&["key", "é"])?)?,
        SchemaVersion::new(2, 0, 0),
    )
}

#[test]
fn versioned_codec_v2_some_empty_stays_bound() -> TestResult {
    let workflow = bound(&[])?;
    assert_eq!(
        workflow.symbol_payloads(),
        Some(Box::<[Box<str>]>::default().as_ref())
    );
    assert_eq!(
        artifact(&workflow)?.artifact_version,
        SchemaVersion::new(2, 0, 0)
    );
    roundtrip(
        workflow,
        v2_wire(&bound_ir(&[])?)?,
        SchemaVersion::new(2, 0, 0),
    )
}

#[test]
fn versioned_codec_v1_minor_and_patch_are_not_normalized() -> TestResult {
    let workflow = legacy(0)?;
    [SchemaVersion::new(1, 2, 0), SchemaVersion::new(1, 0, 1)]
        .into_iter()
        .try_for_each(|held| {
            roundtrip(
                workflow.clone(),
                postcard::to_allocvec(&workflow.to_parts())?,
                held,
            )
        })
}

#[test]
fn versioned_codec_v2_minor_and_patch_are_not_normalized() -> TestResult {
    [SchemaVersion::new(2, 1, 0), SchemaVersion::new(2, 0, 1)]
        .into_iter()
        .try_for_each(|held| roundtrip(bound(&["key"])?, v2_wire(&bound_ir(&["key"])?)?, held))
}

#[path = "versioned_codec_error_tests.rs"]
mod errors;

#[path = "versioned_codec_submission_tests.rs"]
mod submissions;
