use proptest::prelude::*;
use vb_core as frame_seed_core;
use vb_core::{
    BlobId, FiniteF64, ListId, ObjectId, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, SymbolId,
    Taint, frame::RunFrameCheckpointV1Error,
};

#[path = "../src/verification/kani/idempotency_frame_seed.rs"]
mod frame_seed;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::Secret,
    Taint::DerivedFromSecret,
    Taint::Random,
    Taint::TimeDependent,
];

fn equivalent(run: u64, steps: u16, pc: u16, entries: &[(SlotValue, Taint)]) -> TestResult {
    let mut original = RunFrame::new(
        RunId::new(run),
        StepIdx::new(pc),
        steps,
        u16::try_from(entries.len())?,
    )?;
    entries
        .iter()
        .enumerate()
        .try_for_each(|(index, (value, taint))| -> TestResult {
            original.write_slot_with_taint(SlotIdx::new(u16::try_from(index)?), *value, *taint)?;
            Ok(())
        })?;
    let restored = frame_seed::checked_frame_from_seed(
        RunId::new(run),
        StepIdx::new(pc),
        steps,
        entries.iter().copied(),
    )
    .map_err(|error| format!("checked seed rejected: {error:?}"))?;
    assert_eq!(restored, original);
    assert_eq!(restored.checkpoint_v1(), original.checkpoint_v1());
    Ok(())
}

fn dimensions() -> impl Iterator<Item = (u16, u16, u16)> {
    (1_u16..=4).flat_map(|steps| {
        (1_u16..=4).flat_map(move |slots| (0..steps).map(move |pc| (steps, slots, pc)))
    })
}

fn payloads() -> Result<[SlotValue; 8], Box<dyn std::error::Error>> {
    Ok([
        SlotValue::Null,
        SlotValue::Bool(true),
        SlotValue::I64(i64::MIN),
        SlotValue::F64(FiniteF64::new(f64::MAX)?),
        SlotValue::Symbol(SymbolId::new(u32::MAX)),
        SlotValue::List(ListId::new(u32::MAX)),
        SlotValue::Object(ObjectId::new(u32::MAX)),
        SlotValue::Blob(BlobId::new(u64::MAX)),
    ])
}

fn selected_cell_cases(steps: u16, slots: u16, pc: u16, selected: u16) -> TestResult {
    payloads()?.into_iter().try_for_each(|value| {
        TAINTS.into_iter().try_for_each(|taint| -> TestResult {
            let mut entries = vec![(SlotValue::Null, Taint::Clean); usize::from(slots)];
            *entries
                .get_mut(usize::from(selected))
                .ok_or("selected cell")? = (value, taint);
            equivalent(0, steps, pc, &entries)?;
            equivalent(u64::MAX, steps, pc, &entries)
        })
    })
}

#[test]
fn every_original_dimension_pc_payload_and_taint_matches_real_new_and_writes() -> TestResult {
    dimensions().try_for_each(|(steps, slots, pc)| {
        (0..slots).try_for_each(|selected| selected_cell_cases(steps, slots, pc, selected))
    })
}

#[test]
fn checked_seed_retains_actual_factory_rejections() {
    let entries = [(SlotValue::I64(1), Taint::Secret)];
    assert_eq!(
        frame_seed::checked_frame_from_seed(RunId::new(1), StepIdx::ZERO, 0, entries),
        Err(RunFrameCheckpointV1Error::EmptySteps)
    );
    assert_eq!(
        frame_seed::checked_frame_from_seed(RunId::new(1), StepIdx::new(1), 1, entries),
        Err(RunFrameCheckpointV1Error::InvalidFrame)
    );
}

fn value() -> impl Strategy<Value = SlotValue> {
    prop_oneof![
        Just(SlotValue::Null),
        any::<bool>().prop_map(SlotValue::Bool),
        any::<i64>().prop_map(SlotValue::I64),
        any::<f64>()
            .prop_filter_map("finite float", |v| FiniteF64::new(v).ok())
            .prop_map(SlotValue::F64),
        any::<u32>().prop_map(|v| SlotValue::Symbol(SymbolId::new(v))),
        any::<u32>().prop_map(|v| SlotValue::List(ListId::new(v))),
        any::<u32>().prop_map(|v| SlotValue::Object(ObjectId::new(v))),
        any::<u64>().prop_map(|v| SlotValue::Blob(BlobId::new(v))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn independent_original_symbolic_inputs_preserve_complete_frame_images(
        run in any::<u64>(),
        dimensions in (1_u16..=4).prop_flat_map(|steps| (Just(steps), 0..steps)),
        entries in prop::collection::vec((value(), prop::sample::select(TAINTS.to_vec())), 1..5),
    ) {
        let (steps, pc) = dimensions;
        prop_assert_eq!(equivalent(run, steps, pc, &entries).map_err(|error| error.to_string()), Ok(()));
    }
}
