// Fuzz target: Corrupted read-path re-validation defense-in-depth.
//
// Obligation: PO-vb-h09wf-035
// Verifier: cargo-fuzz
// Command: cargo fuzz run ps_012_corrupted_read -- -max_total_time=300
//
// Domain claim: 300s fuzz run: generates valid records, applies fuzz-selected
// digest-mismatch, inner-payload checksum, or trailing-envelope corruption,
// and verifies validation returns a typed error rather than silently returning Ok.
//
// PRODUCTION BINDING:
//   vb_storage::admission::validate_compiled_ir_record

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_storage::{CompiledIrRecord, JournalError};

const ADMISSION_GATE_COUNT: u8 = 15;
const VALID_WORKFLOW: &[u8] = b"version: velvet-ballistics/v1\nname: fuzz_corrupted_read\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
const ALTERNATE_WORKFLOW: &[u8] = b"version: velvet-ballistics/v1\nname: fuzz_corrupted_read_alt\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";

fuzz_target!(|data: &[u8]| {
    let record = match valid_compiled_ir_record() {
        Ok(record) => record,
        Err(error) => panic!("valid compiled-ir setup failed: {error}"),
    };
    assert_valid_record_is_accepted(&record);

    match data.first().copied().map_or(0, |byte| byte % 3) {
        0 => assert_digest_mismatch_is_rejected(record),
        1 => assert_inner_payload_checksum_is_rejected(record),
        _ => assert_trailing_envelope_is_rejected(record, data),
    }
});

fn valid_compiled_ir_record() -> Result<CompiledIrRecord, String> {
    let compiled = vb_compile::compile_workflow(VALID_WORKFLOW)
        .map_err(|error| format!("workflow compile failed: {error}"))?;
    let workflow = normalize_workflow_for_admission(compiled)?;
    let artifact = accepted_artifact_from_workflow(&workflow)?;
    Ok(CompiledIrRecord {
        digest: artifact.digest,
        ir: postcard::to_allocvec(&artifact)
            .map_err(|error| format!("artifact encode failed: {error}"))?,
        metadata_hash: None,
    })
}

fn normalize_workflow_for_admission(
    workflow: vb_core::CompiledWorkflow,
) -> Result<vb_core::CompiledWorkflow, String> {
    let mut parts = workflow.to_parts();
    parts.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let ir = postcard::to_allocvec(&parts)
        .map_err(|error| format!("workflow digest serialization failed: {error}"))?;
    parts.digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|error| format!("normalized workflow rejected: {error}"))
}

fn accepted_artifact_from_workflow(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_storage::AcceptedArtifact, String> {
    Ok(vb_storage::AcceptedArtifact {
        digest: workflow.digest(),
        source_digest: workflow.digest(),
        policy_digest: vb_storage::admission::compute_policy_digest(workflow)
            .map_err(|error| format!("policy digest failed: {error}"))?,
        ir: canonical_workflow_ir(workflow)?,
        verification: vb_storage::VerificationProof::new(
            workflow.digest(),
            ADMISSION_GATE_COUNT,
            false,
        ),
        accepted_at_seq: vb_storage::EventSeq::new(0),
        required_capabilities: Box::new([]),
    })
}

fn canonical_workflow_ir(workflow: &vb_core::CompiledWorkflow) -> Result<Vec<u8>, String> {
    let mut parts = workflow.to_parts();
    parts.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    postcard::to_allocvec(&parts)
        .map_err(|error| format!("workflow canonical serialization failed: {error}"))
}

fn assert_valid_record_is_accepted(record: &CompiledIrRecord) {
    let result = vb_storage::admission::validate_compiled_ir_record(record);
    assert!(
        result.is_ok(),
        "baseline compiled-ir record must validate before corruption: {result:?}"
    );
}

fn assert_digest_mismatch_is_rejected(record: CompiledIrRecord) {
    let digest_bytes = corrupt_digest_bytes(record.digest.as_bytes());
    let corrupted = CompiledIrRecord {
        digest: vb_core::WorkflowDigest::from_bytes(digest_bytes),
        ir: record.ir,
        metadata_hash: None,
    };
    let result = vb_storage::admission::validate_compiled_ir_record(&corrupted);
    assert!(
        matches!(result, Err(JournalError::ArtifactChecksumMismatch)),
        "digest mismatch must be rejected as ArtifactChecksumMismatch: {result:?}"
    );
}

fn corrupt_digest_bytes(mut bytes: [u8; 32]) -> [u8; 32] {
    if let Some(first) = bytes.first_mut() {
        *first ^= 1;
    }
    bytes
}

fn assert_inner_payload_checksum_is_rejected(record: CompiledIrRecord) {
    let corrupted = match inner_payload_checksum_corruption(record) {
        Ok(corrupted) => corrupted,
        Err(error) => panic!("inner payload checksum setup failed: {error}"),
    };
    let result = vb_storage::admission::validate_compiled_ir_record(&corrupted);
    assert!(
        matches!(result, Err(JournalError::ArtifactChecksumMismatch)),
        "inner accepted-artifact IR mutation must be rejected by recomputed checksum: {result:?}"
    );
}

fn inner_payload_checksum_corruption(record: CompiledIrRecord) -> Result<CompiledIrRecord, String> {
    let mut artifact =
        vb_storage::admission::decode_accepted_artifact_envelope(&record.ir)
            .map_err(|error| format!("artifact envelope decode failed: {error}"))?;
    let alternate = alternate_workflow()?;
    reject_policy_digest_drift(&artifact, &alternate)?;
    artifact.ir = canonical_workflow_ir(&alternate)?;
    Ok(CompiledIrRecord {
        digest: record.digest,
        ir: postcard::to_allocvec(&artifact)
            .map_err(|error| format!("corrupted artifact encode failed: {error}"))?,
        metadata_hash: None,
    })
}

fn alternate_workflow() -> Result<vb_core::CompiledWorkflow, String> {
    let compiled = vb_compile::compile_workflow(ALTERNATE_WORKFLOW)
        .map_err(|error| format!("alternate workflow compile failed: {error}"))?;
    normalize_workflow_for_admission(compiled)
}

fn reject_policy_digest_drift(
    artifact: &vb_storage::AcceptedArtifact,
    alternate: &vb_core::CompiledWorkflow,
) -> Result<(), String> {
    let alternate_policy_digest = vb_storage::admission::compute_policy_digest(alternate)
        .map_err(|error| format!("alternate policy digest failed: {error}"))?;
    if artifact.policy_digest == alternate_policy_digest {
        Ok(())
    } else {
        Err(String::from(
            "alternate workflow policy digest differs from baseline",
        ))
    }
}

fn assert_trailing_envelope_is_rejected(mut record: CompiledIrRecord, data: &[u8]) {
    let declared_end = record.ir.len();
    match data.get(1..) {
        Some(bytes) if !bytes.is_empty() => record.ir.extend_from_slice(bytes),
        _ => record.ir.push(0),
    }
    let actual_len = record.ir.len();
    let result = vb_storage::admission::validate_compiled_ir_record(&record);
    assert!(
        matches!(
            result,
            Err(JournalError::UnexpectedTrailingBytes {
                declared_end: found_declared,
                actual_len: found_actual,
            }) if found_declared == declared_end && found_actual == actual_len
        ),
        "trailing accepted-artifact envelope bytes must be rejected: {result:?}"
    );
}
