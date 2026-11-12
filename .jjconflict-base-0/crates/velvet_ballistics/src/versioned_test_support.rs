use vb_core::workflow::{VersionedWorkflowParts, WorkflowIrV2};
use vb_core::{CompiledWorkflow, RuntimePolicy, WorkflowDigest};
use vb_storage::compat::SchemaVersion;

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(crate) const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: callers\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set:\n      output: answer\n      value: \"42\"\n  - id: done\n    finish:\n      result: \"$seed\"\n";

pub(crate) fn legacy() -> TestResult<CompiledWorkflow> {
    Ok(vb_compile::compile_workflow(SOURCE)?)
}

pub(crate) fn symbols(values: &[&str]) -> Box<[Box<str>]> {
    values.iter().map(|s| Box::<str>::from(*s)).collect()
}

pub(crate) fn bound(values: &[&str]) -> TestResult<CompiledWorkflow> {
    let mut parts = legacy()?.to_parts();
    parts.symbols_count = u32::try_from(values.len())?;
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads: symbols(values),
    };
    ir.parts.digest =
        vb_compile::compute_compiled_ir_digest(&VersionedWorkflowParts::V2(ir.clone()))?;
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts,
        ir.symbol_payloads,
    )?)
}

pub(crate) fn raw_record(workflow: &CompiledWorkflow) -> TestResult<vb_storage::CompiledIrRecord> {
    let bytes = vb_compile::encode_compiled_ir(workflow)?;
    Ok(vb_storage::CompiledIrRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(&bytes).into()),
        ir: bytes.into_vec(),
    })
}

pub(crate) fn accepted(version: SchemaVersion) -> TestResult<vb_storage::AcceptedArtifact> {
    let mut artifact =
        vb_storage::admission::prepare_compiled_artifact(&legacy()?, RuntimePolicy::Relaxed, &[])?;
    artifact.artifact_version = version;
    artifact.digest = vb_storage::admission::accepted_artifact_digest(&artifact)?;
    artifact.verification.digest = artifact.digest;
    Ok(artifact)
}

pub(crate) fn accepted_record(
    artifact: &vb_storage::AcceptedArtifact,
) -> TestResult<vb_storage::CompiledIrRecord> {
    Ok(vb_storage::CompiledIrRecord {
        digest: artifact.digest,
        ir: postcard::to_allocvec(artifact)?,
    })
}

pub(crate) fn accepted_v2(workflow: &CompiledWorkflow) -> TestResult<vb_storage::AcceptedArtifact> {
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(workflow, RuntimePolicy::Relaxed, &[])?;
    assert_eq!(artifact.artifact_version, SchemaVersion::new(2, 0, 0));
    assert_eq!(artifact.source_digest, workflow.digest());
    assert_eq!(artifact.ir, independent_v2_bytes(workflow)?);
    assert_envelope_digest(&artifact)?;
    assert_eq!(
        vb_storage::admission::verify_accepted_artifact(&artifact)?,
        *workflow
    );
    Ok(artifact)
}

fn independent_v2_bytes(workflow: &CompiledWorkflow) -> TestResult<Vec<u8>> {
    let VersionedWorkflowParts::V2(ir) = workflow.to_versioned_parts() else {
        return Err("expected a bound V2 fixture".into());
    };
    let mut bytes = vec![0x01, 0xff, 0x56, 0x42];
    bytes.extend_from_slice(&postcard::to_allocvec(&ir)?);
    Ok(bytes)
}

fn assert_envelope_digest(artifact: &vb_storage::AcceptedArtifact) -> TestResult {
    let mut canonical = artifact.clone();
    canonical.digest = WorkflowDigest::from_bytes([0; 32]);
    canonical.verification.digest = WorkflowDigest::from_bytes([0; 32]);
    let bytes = postcard::to_allocvec(&canonical)?;
    let expected = WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes());
    assert_eq!(artifact.digest, expected);
    assert_eq!(artifact.verification.digest, expected);
    Ok(())
}
