use super::*;
use vb_core::workflow::node;
use vb_core::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, InputSlotKind, StepIdx,
    WorkflowParts,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn workflow() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    // Retention validates ports without executing or reading optional inputs.
    let mut parts = WorkflowParts {
        name: "retained_input".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::from([
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::unconstrained(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::unconstrained(),
            },
        ]),
        nodes: Box::from([node(StepIdx::new(0), CompiledNodeKind::Nop, None, None)]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn record(
    workflow: &CompiledWorkflow,
    inputs: Box<[(SlotIdx, SlotValue)]>,
) -> Result<RunInputsRecord, JournalError> {
    RunInputsRecord::new(
        RunId::new(7),
        WorkflowDigest::from_bytes([1; 32]),
        WorkflowDigest::from_bytes([2; 32]),
        workflow,
        inputs,
    )
}

#[test]
fn retained_mapping_rejects_duplicate_undeclared_out_of_range_and_wrong_kind() -> TestResult {
    let workflow = workflow()?;
    let invalid: [Box<[(SlotIdx, SlotValue)]>; 4] = [
        Box::from([
            (SlotIdx::new(0), SlotValue::I64(1)),
            (SlotIdx::new(0), SlotValue::I64(2)),
        ]),
        Box::from([(SlotIdx::new(2), SlotValue::I64(1))]),
        Box::from([(SlotIdx::new(u16::MAX), SlotValue::I64(1))]),
        Box::from([(SlotIdx::new(0), SlotValue::Bool(true))]),
    ];
    for inputs in invalid {
        assert!(matches!(
            record(&workflow, inputs),
            Err(JournalError::InputSchemaMismatch)
        ));
    }
    Ok(())
}

#[test]
fn retained_mapping_preserves_extreme_values_and_empty_mapping() -> TestResult {
    let workflow = workflow()?;
    for value in [i64::MIN, -1, 0, 42, i64::MAX] {
        let expected: Box<[(SlotIdx, SlotValue)]> =
            Box::from([(SlotIdx::new(0), SlotValue::I64(value))]);
        let retained = record(&workflow, expected.clone())?;
        retained.validate_workflow(&workflow)?;
        assert_eq!(retained.inputs(), expected.as_ref());
    }
    assert!(record(&workflow, Box::new([]))?.inputs().is_empty());
    Ok(())
}

#[test]
fn retained_record_envelope_rejects_kind_sequence_trailing_bytes_and_checksum_corruption()
-> TestResult {
    let retained = record(
        &workflow()?,
        Box::from([(SlotIdx::new(0), SlotValue::I64(9))]),
    )?;
    let encode = |kind, seq| {
        crate::codec::encode_record(
            MAGIC_INDEX_RECORD,
            kind,
            seq,
            &retained,
            MAX_RUN_INPUT_BYTES,
        )
    };
    let valid = encode(RecordKind::RunInputs, 7)?;
    let (_, decoded): (_, RunInputsRecord) =
        crate::decode_record(&valid, MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES)?;
    assert_eq!(decoded, retained);
    assert!(matches!(
        crate::decode_record::<RunInputsRecord>(
            &encode(RecordKind::RunHeader, 7)?,
            MAGIC_INDEX_RECORD,
            MAX_RUN_INPUT_BYTES
        ),
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: 3,
            payload_kind: 4
        })
    ));
    assert!(matches!(
        crate::decode_record::<RunInputsRecord>(
            &encode(RecordKind::RunInputs, 8)?,
            MAGIC_INDEX_RECORD,
            MAX_RUN_INPUT_BYTES
        ),
        Err(JournalError::PayloadDigestMismatch)
    ));
    let mut trailing = valid.clone();
    trailing.push(0);
    assert!(matches!(
        crate::decode_record::<RunInputsRecord>(&trailing, MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES),
        Err(JournalError::PostcardDecodeFailed(
            postcard::Error::DeserializeBadEncoding
        ))
    ));
    let mut corrupted = valid;
    let last = corrupted.last_mut().ok_or("empty encoding")?;
    *last ^= 1;
    assert!(matches!(
        crate::decode_record::<RunInputsRecord>(
            &corrupted,
            MAGIC_INDEX_RECORD,
            MAX_RUN_INPUT_BYTES
        ),
        Err(JournalError::PayloadDigestMismatch)
    ));
    Ok(())
}

#[test]
fn retained_mapping_rejects_unbacked_arena_handles_and_zero_run() -> TestResult {
    let mut parts = workflow()?.to_parts();
    for input in &mut parts.input_slots {
        input.kind = InputSlotKind::Any;
    }
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    for value in [
        SlotValue::List(vb_core::ListId::new(0)),
        SlotValue::Object(vb_core::ObjectId::new(0)),
        SlotValue::Blob(vb_core::BlobId::new(0)),
        SlotValue::Symbol(vb_core::SymbolId::new(0)),
    ] {
        assert!(matches!(
            record(&workflow, Box::from([(SlotIdx::new(0), value)])),
            Err(JournalError::InputSchemaMismatch)
        ));
    }
    assert!(matches!(
        RunInputsRecord::new(
            RunId::ZERO,
            WorkflowDigest::from_bytes([0; 32]),
            WorkflowDigest::from_bytes([0; 32]),
            &workflow,
            Box::new([])
        ), Err(JournalError::InvalidRunId { run }) if run == RunId::ZERO
    ));
    Ok(())
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 64,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0xe2_2026),
        ..proptest::test_runner::Config::default()
    })]
    #[test]
    fn arbitrary_i64_retained_values_roundtrip_exactly(value in proptest::prelude::any::<i64>()) {
        let workflow = workflow().map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let retained = record(&workflow, Box::from([(SlotIdx::new(0), SlotValue::I64(value))]))
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let bytes = crate::encode_record(MAGIC_INDEX_RECORD, RecordKind::RunInputs, 7, &retained, MAX_RUN_INPUT_BYTES)
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let (_, decoded): (_, RunInputsRecord) = crate::decode_record(&bytes, MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES)
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        proptest::prop_assert_eq!(decoded.inputs(), &[(SlotIdx::new(0), SlotValue::I64(value))]);
        proptest::prop_assert_eq!(decoded, retained);
    }
}

#[test]
fn journal_input_read_rejects_wrong_run_and_legacy_absence_is_explicit() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    assert_eq!(journal.run_inputs(RunId::new(7))?, None);
    let retained = record(
        &workflow()?,
        Box::from([(SlotIdx::new(0), SlotValue::I64(8))]),
    )?;
    let bytes = crate::encode_record(
        MAGIC_INDEX_RECORD,
        RecordKind::RunInputs,
        7,
        &retained,
        MAX_RUN_INPUT_BYTES,
    )?;
    journal
        .run_inputs
        .insert(crate::keys::run_header_key(RunId::new(8))?, bytes)?;
    assert!(matches!(journal.run_inputs(RunId::new(8)),
        Err(JournalError::WrongRun { expected, actual }) if expected == RunId::new(8) && actual == RunId::new(7)));
    assert!(
        matches!(journal.run_inputs(RunId::ZERO), Err(JournalError::InvalidRunId { run }) if run == RunId::ZERO)
    );
    Ok(())
}

fn constrained_workflow(
    kind: InputSlotKind,
    constraint: CompiledInputConstraint,
) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    // Retention validates ports without executing or reading optional inputs.
    let mut parts = WorkflowParts {
        name: "constrained_input".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::from([CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind,
            constraint,
        }]),
        nodes: Box::from([node(StepIdx::new(0), CompiledNodeKind::Nop, None, None)]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn required_number() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    constrained_workflow(InputSlotKind::I64, CompiledInputConstraint::required())
}

#[test]
fn required_input_missing_is_rejected_before_acceptance() -> TestResult {
    let workflow = required_number()?;
    assert!(matches!(
        record(&workflow, Box::new([])),
        Err(JournalError::InputSchemaMismatch)
    ));
    let retained = record(&workflow, Box::from([(SlotIdx::new(0), SlotValue::I64(3))]))?;
    assert_eq!(retained.inputs(), &[(SlotIdx::new(0), SlotValue::I64(3))]);
    Ok(())
}

#[test]
fn optional_input_missing_is_accepted() -> TestResult {
    let workflow =
        constrained_workflow(InputSlotKind::I64, CompiledInputConstraint::unconstrained())?;
    let retained = record(&workflow, Box::new([]))?;
    assert!(retained.inputs().is_empty());
    Ok(())
}

#[test]
fn scalar_default_is_filled_and_bound_to_identity() -> TestResult {
    let constraint = CompiledInputConstraint {
        presence: false.into(),
        default_value: Some(SlotValue::I64(7)),
        ..CompiledInputConstraint::unconstrained()
    };
    let workflow = constrained_workflow(InputSlotKind::I64, constraint)?;
    let retained = record(&workflow, Box::new([]))?;
    assert_eq!(retained.inputs(), &[(SlotIdx::new(0), SlotValue::I64(7))]);
    // A supplied value wins over the default.
    let retained = record(&workflow, Box::from([(SlotIdx::new(0), SlotValue::I64(9))]))?;
    assert_eq!(retained.inputs(), &[(SlotIdx::new(0), SlotValue::I64(9))]);
    Ok(())
}

#[test]
fn null_accepted_only_when_nullable() -> TestResult {
    let strict = required_number()?;
    assert!(matches!(
        record(&strict, Box::from([(SlotIdx::new(0), SlotValue::Null)])),
        Err(JournalError::InputSchemaMismatch)
    ));
    let nullable = constrained_workflow(
        InputSlotKind::I64,
        CompiledInputConstraint {
            presence: false.into(),
            nullability: true.into(),
            ..CompiledInputConstraint::unconstrained()
        },
    )?;
    let retained = record(&nullable, Box::from([(SlotIdx::new(0), SlotValue::Null)]))?;
    assert_eq!(retained.inputs(), &[(SlotIdx::new(0), SlotValue::Null)]);
    Ok(())
}

#[test]
fn integer_range_bounds_are_enforced() -> TestResult {
    let bounded = constrained_workflow(
        InputSlotKind::I64,
        CompiledInputConstraint {
            presence: false.into(),
            min: Some(1),
            max: Some(10),
            ..CompiledInputConstraint::unconstrained()
        },
    )?;
    for value in [i64::MIN, 0, 11, i64::MAX] {
        assert!(
            matches!(
                record(
                    &bounded,
                    Box::from([(SlotIdx::new(0), SlotValue::I64(value))])
                ),
                Err(JournalError::InputSchemaMismatch)
            ),
            "out-of-range value {value} must be rejected"
        );
    }
    for value in [1, 5, 10] {
        let retained = record(
            &bounded,
            Box::from([(SlotIdx::new(0), SlotValue::I64(value))]),
        )?;
        assert_eq!(
            retained.inputs(),
            &[(SlotIdx::new(0), SlotValue::I64(value))]
        );
    }
    Ok(())
}

#[test]
fn corrupted_constraint_identity_fails_revalidation() -> TestResult {
    let workflow = required_number()?;
    let retained = record(&workflow, Box::from([(SlotIdx::new(0), SlotValue::I64(4))]))?;
    let mut parts = workflow.to_parts();
    let slot = parts.input_slots.first_mut().ok_or("missing port")?;
    slot.constraint.min = Some(100);
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    assert!(matches!(
        retained.validate_workflow(&CompiledWorkflow::try_from_parts(parts)?),
        Err(JournalError::PayloadDigestMismatch)
    ));
    Ok(())
}

#[test]
fn retained_mapping_rejects_workflow_digest_drift_before_execution() -> TestResult {
    let original = workflow()?;
    let retained = record(&original, Box::from([(SlotIdx::new(0), SlotValue::I64(8))]))?;
    let mut parts = original.to_parts();
    parts.name = "other_artifact".into();
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    assert!(matches!(
        retained.validate_workflow(&CompiledWorkflow::try_from_parts(parts)?),
        Err(JournalError::PayloadDigestMismatch)
    ));
    Ok(())
}
