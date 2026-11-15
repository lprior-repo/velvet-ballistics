use super::*;
use crate::recovery::hydrate_run_frame;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const STEP: StepIdx = StepIdx::ZERO;
const SLOT: SlotIdx = SlotIdx::ZERO;

fn snapshot(run: RunId, taint: Taint) -> TestResult<RunSnapshot> {
    let slots = vec![(SLOT, SlotValue::I64(7), taint)];
    Ok(RunSnapshot {
        run,
        seq: EventSeq::new(10),
        workflow: WorkflowDigest::from_bytes([53; 32]),
        slots: postcard::to_allocvec(&slots)?,
        taint: postcard::to_allocvec(&slots)?,
    })
}

fn slot_events(run: RunId, extra: Option<Vec<u8>>) -> TestResult<Vec<JournalEvent>> {
    let value = postcard::to_allocvec(&SlotValue::I64(99))?;
    Ok(vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(11),
            step: STEP,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(12),
            slot: SLOT,
            value: Some(value),
            extra,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(13),
            step: STEP,
            output: SLOT,
        },
    ])
}

fn input_bytes(snapshot: &RunSnapshot, tail: &[JournalEvent]) -> TestResult<Vec<u8>> {
    Ok(postcard::to_allocvec(&(snapshot, tail))?)
}

fn assert_inputs_unchanged(
    snapshot: &RunSnapshot,
    tail: &[JournalEvent],
    before: &[u8],
) -> TestResult {
    assert_eq!(input_bytes(snapshot, tail)?, before);
    Ok(())
}

fn assert_success(run: RunId, initial: Taint, persisted: Taint) -> TestResult {
    let snapshot = snapshot(run, initial)?;
    let entries: Vec<(SlotIdx, SlotValue, Taint)> = postcard::from_bytes(&snapshot.slots)?;
    assert_eq!(entries.first().map(|entry| entry.2), Some(initial));
    let taint_entries: Vec<(SlotIdx, SlotValue, Taint)> = postcard::from_bytes(&snapshot.taint)?;
    assert_eq!(taint_entries.first().map(|entry| entry.2), Some(initial));
    let extra = crate::encode_slot_written_extra(persisted, None)
        .map_err(|error| format!("slot extra encoding failed: {error:?}"))?;
    let tail = slot_events(run, Some(extra))?;
    let before = input_bytes(&snapshot, &tail)?;
    let frame = hydrate_run_frame(&snapshot, &tail, run)
        .map_err(|error| format!("snapshot tail hydration failed: {error:?}"))?;
    assert_inputs_unchanged(&snapshot, &tail, &before)?;
    assert_eq!(frame.read_slot(SLOT)?, &SlotValue::I64(99));
    assert_eq!(frame.read_taint(SLOT)?, persisted);
    assert_eq!(frame.step_state(STEP)?, vb_core::StepState::Succeeded);
    Ok(())
}

#[test]
fn snapshot_tail_applies_explicit_clean_taint() -> TestResult {
    assert_success(RunId::new(53_001), Taint::Clean, Taint::Clean)
}

#[test]
fn snapshot_tail_applies_explicit_derived_taint() -> TestResult {
    assert_success(RunId::new(53_002), Taint::Clean, Taint::DerivedFromSecret)
}

#[test]
fn snapshot_tail_applies_explicit_secret_taint() -> TestResult {
    assert_success(RunId::new(53_003), Taint::Clean, Taint::Secret)
}

#[test]
fn snapshot_tail_applies_explicit_random_taint() -> TestResult {
    assert_success(RunId::new(53_009), Taint::Clean, Taint::Random)
}

#[test]
fn snapshot_tail_applies_explicit_time_dependent_taint() -> TestResult {
    assert_success(RunId::new(53_010), Taint::Clean, Taint::TimeDependent)
}

#[test]
fn snapshot_tail_allows_explicit_clean_replacement_of_secret() -> TestResult {
    assert_success(RunId::new(53_004), Taint::Secret, Taint::Clean)
}

#[test]
fn snapshot_tail_malformed_vbse_returns_exact_error_without_mutation() -> TestResult {
    let run = RunId::new(53_005);
    let snapshot = snapshot(run, Taint::Clean)?;
    let mut malformed = crate::SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    malformed.extend_from_slice(&[255, 255, 255]);
    let tail = slot_events(run, Some(malformed))?;
    let before = input_bytes(&snapshot, &tail)?;
    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert_inputs_unchanged(&snapshot, &tail, &before)?;
    assert!(matches!(
        result,
        Err(RecoveryError::CorruptSlotTaint { slot }) if slot == SLOT
    ));
    Ok(())
}

#[test]
fn snapshot_tail_preserves_value_decode_error_precedence() -> TestResult {
    let run = RunId::new(53_008);
    let snapshot = snapshot(run, Taint::Clean)?;
    let empty_snapshot = RunSnapshot {
        slots: postcard::to_allocvec(&Vec::<(SlotIdx, SlotValue, Taint)>::new())?,
        taint: postcard::to_allocvec(&Vec::<(SlotIdx, SlotValue, Taint)>::new())?,
        ..snapshot
    };
    let mut malformed = crate::SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    malformed.extend_from_slice(&[255, 255, 255]);
    let mut tail = slot_events(run, Some(malformed))?;
    let value = tail.iter_mut().find_map(|event| match event {
        JournalEvent::SlotWrittenEvent { value, .. } => Some(value),
        _ => None,
    });
    let Some(value) = value else {
        return Err("slot-written event missing from fixture".into());
    };
    *value = Some(vec![255]);
    let before = input_bytes(&empty_snapshot, &tail)?;
    let result = hydrate_run_frame(&empty_snapshot, &tail, run);
    assert_inputs_unchanged(&empty_snapshot, &tail, &before)?;
    let expected_detail = format!("slot value decode failed for run {:?} slot {:?}", run, SLOT);
    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO && detail == expected_detail
    ));
    Ok(())
}

#[test]
fn snapshot_tail_absent_extra_preserves_snapshot_taint() -> TestResult {
    assert_legacy_snapshot_taint(RunId::new(53_006), None)
}

#[test]
fn snapshot_tail_non_envelope_extra_preserves_snapshot_taint() -> TestResult {
    assert_legacy_snapshot_taint(RunId::new(53_007), Some(vec![0, 1, 2, 3]))
}

fn assert_legacy_snapshot_taint(run: RunId, extra: Option<Vec<u8>>) -> TestResult {
    let snapshot = snapshot(run, Taint::Secret)?;
    let tail = slot_events(run, extra)?;
    let before = input_bytes(&snapshot, &tail)?;
    let frame = hydrate_run_frame(&snapshot, &tail, run)
        .map_err(|error| format!("snapshot tail hydration failed: {error:?}"))?;
    assert_inputs_unchanged(&snapshot, &tail, &before)?;
    assert_eq!(frame.read_slot(SLOT)?, &SlotValue::I64(99));
    assert_eq!(frame.read_taint(SLOT)?, Taint::Secret);
    assert_eq!(frame.step_state(STEP)?, vb_core::StepState::Succeeded);
    Ok(())
}
