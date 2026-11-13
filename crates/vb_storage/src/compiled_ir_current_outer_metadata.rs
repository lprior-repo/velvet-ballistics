#![forbid(unsafe_code)]

use super::super::*;
use vb_core::workflow::source_metadata::{CompiledSecretRequirement, WorkflowSourceMetadata};

fn metadata() -> TestResult<WorkflowSourceMetadata> {
    Ok(WorkflowSourceMetadata::try_new(
        payloads(&["clé", "TOKEN"]),
        Box::from([CompiledSecretRequirement::new(
            SymbolId::new(0),
            SymbolId::new(1),
        )]),
    )?)
}

fn roundtrip(wf: CompiledWorkflow) -> TestResult {
    let bytes = encode_compiled_ir(&wf)?;
    let decoded = decode_compiled_ir(&bytes, Some(SchemaVersion::new(2, 0, 0)))?;
    assert_eq!(decoded, wf);
    assert_eq!(decoded.source_metadata(), Some(&metadata()?));
    assert_eq!(
        decoded.to_parts().semantic_source_identity,
        Some(WorkflowDigest::from_bytes([7; 32])),
    );
    Ok(())
}

fn metadata_parts() -> TestResult<WorkflowParts> {
    let mut parts = parts(2);
    parts.source_metadata = Some(Box::new(metadata()?));
    parts.semantic_source_identity = Some(WorkflowDigest::from_bytes([7; 32]));
    Ok(parts)
}

#[test]
fn current_outer_raw_metadata_roundtrip() -> TestResult {
    let stamped = stamp_v1(metadata_parts()?)?;
    let wf = CompiledWorkflow::try_from_parts(stamped)?;
    roundtrip(wf.clone())?;
    let literal = WorkflowDigest::from_bytes([7; 32]);
    assert_eq!(wf.to_parts().semantic_source_identity, Some(literal));
    assert_eq!(wf.symbol_payloads(), None);
    Ok(())
}

#[test]
fn current_outer_bound_metadata_roundtrip() -> TestResult {
    let ir = WorkflowIrV2 {
        wire_version: 2,
        parts: metadata_parts()?,
        symbol_payloads: payloads(&["clé", "TOKEN"]),
    };
    let stamped = stamp_v2(ir)?;
    let wf = CompiledWorkflow::try_from_parts_with_symbols(stamped.parts, stamped.symbol_payloads)?;
    roundtrip(wf.clone())?;
    let literal = WorkflowDigest::from_bytes([7; 32]);
    assert_eq!(wf.to_parts().semantic_source_identity, Some(literal));
    assert_eq!(
        wf.symbol_payloads(),
        Some(payloads(&["clé", "TOKEN"]).as_ref())
    );
    Ok(())
}
