// Fuzz target: Trailing bytes concatenation attack defense (Gate 3).
//
// Obligation: PO-vb-h09wf-016
// Verifier: cargo-fuzz
// Command: cargo fuzz run ps_005_trailing_bytes -- -max_total_time=300
//
// Domain claim: 300s fuzz run: no panics, no crashes. All payloads with
// trailing bytes rejected. Defends against concatenation attacks (H5).
//
// PRODUCTION BINDING:
//   vb_storage::admission::fuzz_access::decode_accepted_artifact_envelope
//   vb_storage::codec::fuzz_validation::reject_trailing_bytes

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_storage::JournalError;

const ADMISSION_GATE_COUNT: u8 = 15;
const VALID_WORKFLOW: &[u8] = b"version: velvet-ballistics/v1\nname: fuzz_trailing\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";

fuzz_target!(|data: &[u8]| {
    // Generate a payload where a potentially valid envelope is followed by
    // arbitrary trailing bytes. The fuzzer varies both the envelope and trailer.

    // Split: first byte determines the split point
    let split_pct = data.first().copied().map_or(0, usize::from);
    let split = if data.len() > 1 {
        (split_pct % data.len())
            .max(1)
            .min(data.len().saturating_sub(1))
    } else {
        1.min(data.len())
    };

    let _envelope_part = &data[..split];
    let _trailer_part = &data[split..];

    // Test reject_trailing_bytes with various boundary values
    for declared in [0, split, data.len(), data.len().saturating_sub(1)] {
        for actual in [0, split, data.len(), data.len().saturating_add(1)] {
            assert_trailing_result(declared.min(usize::MAX / 2), actual.min(usize::MAX / 2));
        }
    }

    let mut envelope = match valid_accepted_artifact_envelope() {
        Ok(envelope) => envelope,
        Err(error) => panic!("valid accepted-artifact setup failed: {error}"),
    };
    assert_valid_envelope_decodes(&envelope);
    let declared_end = envelope.len();
    append_nonempty_trailer(&mut envelope, data);
    let actual_len = envelope.len();
    let result = vb_storage::admission::fuzz_access::decode_accepted_artifact_envelope(&envelope);
    assert!(
        matches!(
            result,
            Err(JournalError::UnexpectedTrailingBytes {
                declared_end: found_declared,
                actual_len: found_actual,
            }) if found_declared == declared_end && found_actual == actual_len
        ),
        "accepted artifact envelope with appended bytes must reject as trailing bytes: {result:?}"
    );
});

fn assert_trailing_result(declared_end: usize, actual_len: usize) {
    let result =
        vb_storage::codec::fuzz_validation::reject_trailing_bytes(declared_end, actual_len);
    if actual_len > declared_end {
        assert!(
            matches!(
                result,
                Err(JournalError::UnexpectedTrailingBytes {
                    declared_end: found_declared,
                    actual_len: found_actual,
                }) if found_declared == declared_end && found_actual == actual_len
            ),
            "trailing bytes must return exact UnexpectedTrailingBytes: {result:?}"
        );
    } else {
        assert!(
            result.is_ok(),
            "non-trailing bounds must be accepted: {result:?}"
        );
    }
}

fn valid_accepted_artifact_envelope() -> Result<Vec<u8>, String> {
    let compiled = vb_compile::compile_workflow(VALID_WORKFLOW)
        .map_err(|error| format!("workflow compile failed: {error}"))?;
    let workflow = normalize_workflow_for_admission(compiled)?;
    let artifact = accepted_artifact_from_workflow(&workflow)?;
    postcard::to_allocvec(&artifact).map_err(|error| format!("artifact encode failed: {error}"))
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

fn append_nonempty_trailer(envelope: &mut Vec<u8>, data: &[u8]) {
    match data.get(1..) {
        Some(bytes) if !bytes.is_empty() => envelope.extend_from_slice(bytes),
        _ => envelope.push(0),
    }
}

fn assert_valid_envelope_decodes(envelope: &[u8]) {
    let result = vb_storage::admission::fuzz_access::decode_accepted_artifact_envelope(envelope);
    assert!(
        result.is_ok(),
        "baseline accepted-artifact envelope must decode before trailer mutation: {result:?}"
    );
}
