use super::*;
use vb_core::workflow::WorkflowIrV2;

pub(super) fn bound_fixture(values: &[&str]) -> TestResult<PackedFixture> {
    let mut fixture = PackedFixture::new()?;
    let mut parts = fixture.workflow.to_parts();
    parts.symbols_count = u32::try_from(values.len())?;
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads: values
            .iter()
            .map(|value| Box::<str>::from(*value))
            .collect(),
    };
    ir.parts.digest = WorkflowDigest::from_bytes(blake3::hash(&wire(&ir)?).into());
    fixture.workflow =
        vb_core::CompiledWorkflow::try_from_parts_with_symbols(ir.parts, ir.symbol_payloads)?;
    Ok(fixture)
}

pub(super) fn wire(ir: &WorkflowIrV2) -> TestResult<Vec<u8>> {
    let mut bytes = vec![0x01, 0xff, 0x56, 0x42];
    bytes.extend_from_slice(&postcard::to_allocvec(ir)?);
    Ok(bytes)
}

pub(super) fn empty_packs(artifact: &mut AcceptedArtifact) -> TestResult {
    let ArtifactEvidence::Packed(packed) = &mut artifact.verification.evidence else {
        return Err("expected Packed evidence".into());
    };
    packed.packs = Box::new([]);
    Ok(())
}

pub(super) fn exact_pack_mismatch(
    result: Result<vb_core::CompiledWorkflow, ExecutorError>,
) -> TestResult {
    match result {
        Err(ExecutorError::PackMismatch) => Ok(()),
        other => Err(format!("expected PackMismatch, found {other:?}").into()),
    }
}

pub(super) fn read(
    journal: &FjallJournal,
    fixture: &PackedFixture,
    expected: &AcceptedArtifact,
) -> TestResult {
    let record = journal
        .compiled_ir(expected.digest)?
        .ok_or("missing packed artifact")?;
    let actual: AcceptedArtifact = postcard::from_bytes(&record.ir)?;
    assert_eq!(record.digest, expected.digest);
    assert_eq!(&actual, expected);
    assert_eq!(
        fixture
            .catalog
            .verify_artifact(&actual, RuntimePolicy::Strict)?,
        fixture.workflow
    );
    let ArtifactEvidence::Packed(packed) = &actual.verification.evidence else {
        return Err("lost Packed profile".into());
    };
    assert_eq!(packed.packs.as_ref(), std::slice::from_ref(&fixture.pack));
    Ok(())
}

pub(super) fn cold_roundtrip(fixture: PackedFixture, held: SchemaVersion) -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let mut accepted = fixture.submit(&journal, RuntimePolicy::Strict)?;
    assert_eq!(accepted.accepted_at_seq, crate::EventSeq::new(1));
    accepted.artifact_version = held;
    admission::bind_artifact_digest(&mut accepted)?;
    journal.put_compiled_ir(&crate::CompiledIrRecord {
        digest: accepted.digest,
        ir: postcard::to_allocvec(&accepted)?,
    })?;
    journal.persist_strict()?;
    read(&journal, &fixture, &accepted)?;
    drop(journal);
    let reopened = FjallJournal::open(dir.path(), None)?;
    read(&reopened, &fixture, &accepted)
}
