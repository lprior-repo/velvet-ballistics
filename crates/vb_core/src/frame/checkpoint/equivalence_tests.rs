use super::reference::via_new_and_seed;
use super::{
    CheckpointStepState as State, RunFrame, RunFrameCheckpointV1 as Checkpoint,
    RunFrameCheckpointV1Error as Error,
};
use crate::{
    BlobId, FiniteF64, ListId, ObjectId, RunId, SlotIdx, SlotValue, StepIdx, SymbolId, Taint,
};
use proptest::prelude::*;

const STATES: [State; 8] = [
    State::Pending,
    State::Running,
    State::Succeeded,
    State::Failed,
    State::Skipped,
    State::Waiting,
    State::Asking,
    State::Cancelled,
];
const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::Random,
    Taint::TimeDependent,
];
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn base() -> Checkpoint {
    Checkpoint {
        run_id: RunId::new(u64::MAX),
        pc: StepIdx::ZERO,
        executed: u64::MAX,
        max_parallel_in_flight: u16::MAX,
        parallel_in_flight: u16::MAX,
        states: STATES.to_vec(),
        slots: vec![None],
        taint: vec![Taint::Clean],
    }
}

fn exact(checkpoint: Checkpoint) -> TestResult {
    let expected = via_new_and_seed(checkpoint.clone())
        .map_err(|error| format!("old checked constructor: {error:?}"))?;
    let actual = RunFrame::from_checkpoint_v1(checkpoint.clone())
        .map_err(|error| format!("checked checkpoint factory: {error:?}"))?;
    assert_eq!(actual, expected);
    assert_eq!(actual.checkpoint_v1(), checkpoint);
    Ok(())
}

#[test]
fn all_states_taints_and_dimension_boundaries_match_checked_new() -> TestResult {
    for states in [1_usize, 8, usize::from(u16::MAX)] {
        for slots in [0_usize, 1, usize::from(u16::MAX)] {
            for last_pc in [false, true] {
                let mut cp = base();
                cp.states = STATES.into_iter().cycle().take(states).collect();
                cp.slots = (0..slots)
                    .map(|index| {
                        if index % 2 == 0 {
                            None
                        } else {
                            Some(SlotValue::I64(i64::MIN))
                        }
                    })
                    .collect();
                cp.taint = cp
                    .slots
                    .iter()
                    .zip(TAINTS.into_iter().cycle())
                    .map(|(value, taint)| if value.is_none() { Taint::Clean } else { taint })
                    .collect();
                cp.pc = StepIdx::new(if last_pc {
                    u16::try_from(states.saturating_sub(1))?
                } else {
                    0
                });
                exact(cp)?;
            }
        }
    }
    Ok(())
}

#[test]
fn validation_errors_keep_exact_original_precedence() -> TestResult {
    let mut cp = base();
    cp.states = vec![State::Pending; usize::from(u16::MAX).saturating_add(1)];
    cp.slots = vec![None; usize::from(u16::MAX).saturating_add(1)];
    assert_eq!(
        RunFrame::from_checkpoint_v1(cp),
        Err(Error::StepCountOverflow)
    );
    let mut cp = base();
    cp.states.clear();
    cp.slots = vec![None; usize::from(u16::MAX).saturating_add(1)];
    assert_eq!(
        RunFrame::from_checkpoint_v1(cp),
        Err(Error::SlotCountOverflow)
    );
    let mut cp = base();
    cp.states.clear();
    cp.taint.clear();
    cp.parallel_in_flight = 1;
    cp.max_parallel_in_flight = 0;
    cp.pc = StepIdx::MAX;
    assert_eq!(RunFrame::from_checkpoint_v1(cp), Err(Error::EmptySteps));
    let mut cp = base();
    cp.taint.clear();
    cp.parallel_in_flight = 1;
    cp.max_parallel_in_flight = 0;
    cp.pc = StepIdx::MAX;
    assert_eq!(
        RunFrame::from_checkpoint_v1(cp),
        Err(Error::SlotTaintLengthMismatch)
    );
    let mut cp = base();
    cp.taint = vec![Taint::Secret];
    cp.parallel_in_flight = 1;
    cp.max_parallel_in_flight = 0;
    cp.pc = StepIdx::MAX;
    assert_eq!(
        RunFrame::from_checkpoint_v1(cp),
        Err(Error::TaintWithoutValue)
    );
    let mut cp = base();
    cp.parallel_in_flight = 1;
    cp.max_parallel_in_flight = 0;
    cp.pc = StepIdx::MAX;
    assert_eq!(
        RunFrame::from_checkpoint_v1(cp),
        Err(Error::ParallelCountOutOfBounds)
    );
    let mut cp = base();
    cp.pc = StepIdx::MAX;
    assert_eq!(RunFrame::from_checkpoint_v1(cp), Err(Error::InvalidFrame));
    for count in [1_u16, 8, u16::MAX] {
        let mut cp = base();
        cp.states = vec![State::Pending; usize::from(count)];
        cp.pc = StepIdx::new(count);
        assert_eq!(RunFrame::from_checkpoint_v1(cp), Err(Error::InvalidFrame));
    }
    for taint in TAINTS.into_iter().skip(1) {
        let mut cp = base();
        cp.taint = vec![taint];
        assert_eq!(
            RunFrame::from_checkpoint_v1(cp),
            Err(Error::TaintWithoutValue)
        );
    }
    for bound in [0_u16, 1, u16::MAX] {
        let mut cp = base();
        cp.max_parallel_in_flight = bound;
        cp.parallel_in_flight = bound;
        exact(cp)?;
    }

    Ok(())
}

#[test]
fn all_64_replay_layout_seed_images_match_checked_new() -> TestResult {
    let mut cases = 0_usize;
    for bits in 0_u8..16 {
        let two_branches = bits & 1 != 0;
        let shifted = u16::from(bits & 2 != 0);
        let reverse = bits & 4 != 0;
        let otherwise = bits & 8 != 0;
        let steps = match (two_branches, otherwise) {
            (false, false) => 2,
            (true, true) => 4,
            _ => 3,
        };
        for (a, b) in [(false, false), (false, true), (true, false), (true, true)] {
            let first = SlotIdx::new(shifted.saturating_add(u16::from(reverse)));
            let second = SlotIdx::new(shifted.saturating_add(u16::from(!reverse)));
            let mut reference = RunFrame::new(
                RunId::new(0),
                StepIdx::ZERO,
                steps,
                shifted.saturating_add(2),
            )?;
            reference.write_slot(first, SlotValue::Bool(a))?;
            reference.write_slot(second, SlotValue::Bool(b))?;
            let image = reference.checkpoint_v1();
            let restored = RunFrame::from_checkpoint_v1(image.clone())
                .map_err(|error| format!("checkpoint: {error:?}"))?;
            assert_eq!(restored, reference);
            assert_eq!(restored.checkpoint_v1(), image);
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 64);
    Ok(())
}

fn value() -> impl Strategy<Value = SlotValue> {
    prop_oneof![
        Just(SlotValue::Null),
        any::<bool>().prop_map(SlotValue::Bool),
        any::<i64>().prop_map(SlotValue::I64),
        any::<i32>().prop_map(|v| SlotValue::F64(FiniteF64::from_i64(i64::from(v)))),
        any::<u32>().prop_map(|v| SlotValue::Symbol(SymbolId::new(v))),
        any::<u32>().prop_map(|v| SlotValue::List(ListId::new(v))),
        any::<u32>().prop_map(|v| SlotValue::Object(ObjectId::new(v))),
        any::<u64>().prop_map(|v| SlotValue::Blob(BlobId::new(v)))
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, ..ProptestConfig::default() })]
    #[test]
    fn arbitrary_full_checkpoint_matches_previous_factory(
        states in prop::collection::vec(prop::sample::select(STATES.to_vec()), 0..33),
        slots in prop::collection::vec(prop::option::of(value()), 0..33),
        taint in prop::collection::vec(prop::sample::select(TAINTS.to_vec()), 0..33),
        run in any::<u64>(), executed in any::<u64>(), pc in any::<u16>(),
        max_parallel in any::<u16>(), parallel in any::<u16>(),
    ) {
        let cp = Checkpoint { run_id: RunId::new(run), pc: StepIdx::new(pc), executed, max_parallel_in_flight: max_parallel, parallel_in_flight: parallel, states, slots, taint };
        prop_assert_eq!(RunFrame::from_checkpoint_v1(cp.clone()), via_new_and_seed(cp));
    }

    #[test]
    fn arbitrary_valid_full_checkpoint_keeps_every_field(
        states in prop::collection::vec(prop::sample::select(STATES.to_vec()), 1..33),
        values in prop::collection::vec((prop::option::of(value()), prop::sample::select(TAINTS.to_vec())), 0..33),
        run in any::<u64>(), executed in any::<u64>(), selector in any::<u16>(),
        max_parallel in any::<u16>(), parallel in any::<u16>(),
    ) {
        let pc = usize::from(selector) % states.len();
        let (slots, taint): (Vec<_>, Vec<_>) = values.into_iter().map(|(value, taint)| (value, if value.is_none() { Taint::Clean } else { taint })).unzip();
        let active = u32::from(parallel) % u32::from(max_parallel).saturating_add(1);
        let cp = Checkpoint { run_id: RunId::new(run), pc: StepIdx::new(u16::try_from(pc).map_err(|e| TestCaseError::fail(e.to_string()))?), executed, max_parallel_in_flight: max_parallel, parallel_in_flight: u16::try_from(active).map_err(|e| TestCaseError::fail(e.to_string()))?, states, slots, taint };
        let result = exact(cp);
        prop_assert_eq!(result.map_err(|error| error.to_string()), Ok(()));
    }
}
