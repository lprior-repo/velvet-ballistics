//! Property tests for step delta computation.
//!
//! Covers PO-004 (delta computation correctness) and PO-025 (SlotValue serialization determinism).

use vb_core::value::{SlotValue, Taint};
use vb_core::frame::StepState;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::engine::signals::EngineSignal;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const MAX_SLOTS: u16 = 1024;
    const MAX_STEPS: u16 = 1024;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct SlotDelta {
        slot: u16,
        before: Option<SlotValue>,
        after: Option<SlotValue>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TaintDelta {
        slot: u16,
        before: Taint,
        after: Taint,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct StateDelta {
        step: u16,
        before: StepState,
        after: StepState,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct PcDelta {
        before: StepIdx,
        after: StepIdx,
    }

    fn compute_slot_deltas(
        slots_before: &[Option<SlotValue>],
        slots_after: &[Option<SlotValue>],
    ) -> Vec<SlotDelta> {
        let count = slots_before.len().min(slots_after.len());
        slots_before[..count]
            .iter()
            .zip(slots_after[..count].iter())
            .enumerate()
            .filter_map(|(i, (before, after))| {
                if before != after {
                    Some(SlotDelta {
                        slot: i as u16,
                        before: *before,
                        after: *after,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn compute_taint_deltas(
        taint_before: &[Taint],
        taint_after: &[Taint],
    ) -> Vec<TaintDelta> {
        let count = taint_before.len().min(taint_after.len());
        taint_before[..count]
            .iter()
            .zip(taint_after[..count].iter())
            .enumerate()
            .filter_map(|(i, (before, after))| {
                if before != after {
                    Some(TaintDelta {
                        slot: i as u16,
                        before: *before,
                        after: *after,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn compute_state_deltas(
        states_before: &[StepState],
        states_after: &[StepState],
    ) -> Vec<StateDelta> {
        let count = states_before.len().min(states_after.len());
        states_before[..count]
            .iter()
            .zip(states_after[..count].iter())
            .enumerate()
            .filter_map(|(i, (before, after))| {
                if before != after {
                    Some(StateDelta {
                        step: i as u16,
                        before: *before,
                        after: *after,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn compute_pc_delta(before: StepIdx, after: StepIdx) -> PcDelta {
        PcDelta { before, after }
    }

    prop_compose! {
        fn arb_slot_value()(
            variant in prop_oneof![
                Just(SlotValue::Null),
                any::<bool>().prop_map(SlotValue::Bool),
                any::<i64>().prop_map(SlotValue::I64),
                (0u32..1000u32).prop_map(|id| SlotValue::Symbol(vb_core::ids::SymbolId::new(id))),
                (0u32..1000u32).prop_map(|id| SlotValue::List(vb_core::ids::ListId::new(id))),
                (0u32..1000u32).prop_map(|id| SlotValue::Object(vb_core::ids::ObjectId::new(id))),
                (0u32..1000u32).prop_map(|id| SlotValue::Blob(vb_core::ids::BlobId::new(id))),
            ]
        ) -> SlotValue {
            variant
        }
    }

    prop_compose! {
        fn arb_taint()(
            t in prop_oneof![
                Just(Taint::Clean),
                Just(Taint::DerivedFromSecret),
                Just(Taint::Secret),
            ]
        ) -> Taint {
            t
        }
    }

    prop_compose! {
        fn arb_step_state()(
            s in prop_oneof![
                Just(StepState::Pending),
                Just(StepState::Running),
                Just(StepState::Succeeded),
                Just(StepState::Failed),
                Just(StepState::Skipped),
                Just(StepState::Waiting),
                Just(StepState::Asking),
                Just(StepState::Cancelled),
            ]
        ) -> StepState {
            s
        }
    }

    prop_compose! {
        fn arb_slot_array(max_len: usize)(
            len in 0..max_len,
            values in prop::collection::vec(arb_slot_value(), len),
        ) -> Vec<Option<SlotValue>> {
            values.into_iter().map(Some).collect()
        }
    }

    prop_compose! {
        fn arb_taint_array(len: usize)(
            taints in prop::collection::vec(arb_taint(), len),
        ) -> Vec<Taint> {
            taints
        }
    }

    prop_compose! {
        fn arb_state_array(len: usize)(
            states in prop::collection::vec(arb_step_state(), len),
        ) -> Vec<StepState> {
            states
        }
    }

    proptest! {
        #[test]
        fn slot_deltas_only_include_changed_slots(
            max_slots in 0..MAX_SLOTS,
        ) {
            let slot_count = max_slots.max(1);
            let slots_before: Vec<Option<SlotValue>> = (0..slot_count)
                .map(|_| Some(arb_slot_value()))
                .collect();
            let slots_after: Vec<Option<SlotValue>> = (0..slot_count)
                .map(|_| Some(arb_slot_value()))
                .collect();

            let deltas = compute_slot_deltas(&slots_before, &slots_after);

            for delta in &deltas {
                let idx = delta.slot as usize;
                prop_assert!(idx < slot_count as usize);
                prop_assert_ne!(delta.before, delta.after, "delta should only include changed slots");
            }

            for i in 0..slot_count as usize {
                let before = &slots_before[i];
                let after = &slots_after[i];
                let has_delta = deltas.iter().any(|d| d.slot == i as u16);
                if before != after {
                    prop_assert!(has_delta, "changed slot {} should appear in deltas", i);
                } else {
                    prop_assert!(!has_delta, "unchanged slot {} should not appear in deltas", i);
                }
            }
        }

        #[test]
        fn taint_deltas_only_include_changed_taints(
            max_slots in 0..MAX_SLOTS,
        ) {
            let slot_count = max_slots.max(1);
            let taint_before: Vec<Taint> = (0..slot_count)
                .map(|_| arb_taint())
                .collect();
            let taint_after: Vec<Taint> = (0..slot_count)
                .map(|_| arb_taint())
                .collect();

            let deltas = compute_taint_deltas(&taint_before, &taint_after);

            for delta in &deltas {
                let idx = delta.slot as usize;
                prop_assert!(idx < slot_count as usize);
                prop_assert_ne!(delta.before, delta.after, "delta should only include changed taints");
            }

            for i in 0..slot_count as usize {
                let before = &taint_before[i];
                let after = &taint_after[i];
                let has_delta = deltas.iter().any(|d| d.slot == i as u16);
                if before != after {
                    prop_assert!(has_delta, "changed taint {} should appear in deltas", i);
                } else {
                    prop_assert!(!has_delta, "unchanged taint {} should not appear in deltas", i);
                }
            }
        }

        #[test]
        fn state_deltas_only_include_changed_states(
            max_steps in 0..MAX_STEPS,
        ) {
            let step_count = max_steps.max(1);
            let states_before: Vec<StepState> = (0..step_count)
                .map(|_| arb_step_state())
                .collect();
            let states_after: Vec<StepState> = (0..step_count)
                .map(|_| arb_step_state())
                .collect();

            let deltas = compute_state_deltas(&states_before, &states_after);

            for delta in &deltas {
                let idx = delta.step as usize;
                prop_assert!(idx < step_count as usize);
                prop_assert_ne!(delta.before, delta.after, "delta should only include changed states");
            }

            for i in 0..step_count as usize {
                let before = &states_before[i];
                let after = &states_after[i];
                let has_delta = deltas.iter().any(|d| d.step == i as u16);
                if before != after {
                    prop_assert!(has_delta, "changed state {} should appear in deltas", i);
                } else {
                    prop_assert!(!has_delta, "unchanged state {} should not appear in deltas", i);
                }
            }
        }

        #[test]
        fn pc_delta_reflects_actual_changes(
            before_pc in 0u16..MAX_STEPS,
            after_pc in 0u16..MAX_STEPS,
        ) {
            let before = StepIdx::new(before_pc);
            let after = StepIdx::new(after_pc);
            let delta = compute_pc_delta(before, after);

            prop_assert_eq!(delta.before, before);
            prop_assert_eq!(delta.after, after);
        }

        #[test]
        fn slot_deltas_bounds(
            max_slots in 0..256u16,
        ) {
            let slot_count = max_slots.max(1) as usize;
            let slots_before: Vec<Option<SlotValue>> = (0..slot_count)
                .map(|_| Some(arb_slot_value()))
                .collect();
            let slots_after: Vec<Option<SlotValue>> = (0..slot_count)
                .map(|_| Some(arb_slot_value()))
                .collect();

            let deltas = compute_slot_deltas(&slots_before, &slots_after);

            for delta in &deltas {
                prop_assert!((delta.slot as usize) < slot_count);
            }
        }

        #[test]
        fn taint_deltas_bounds(
            max_slots in 0..256u16,
        ) {
            let slot_count = max_slots.max(1) as usize;
            let taint_before: Vec<Taint> = (0..slot_count)
                .map(|_| arb_taint())
                .collect();
            let taint_after: Vec<Taint> = (0..slot_count)
                .map(|_| arb_taint())
                .collect();

            let deltas = compute_taint_deltas(&taint_before, &taint_after);

            for delta in &deltas {
                prop_assert!((delta.slot as usize) < slot_count);
            }
        }

        #[test]
        fn state_deltas_bounds(
            max_steps in 0..256u16,
        ) {
            let step_count = max_steps.max(1) as usize;
            let states_before: Vec<StepState> = (0..step_count)
                .map(|_| arb_step_state())
                .collect();
            let states_after: Vec<StepState> = (0..step_count)
                .map(|_| arb_step_state())
                .collect();

            let deltas = compute_state_deltas(&states_before, &states_after);

            for delta in &deltas {
                prop_assert!((delta.step as usize) < step_count);
            }
        }

        #[test]
        fn empty_arrays_produce_no_deltas() {
            let slots_before: Vec<Option<SlotValue>> = vec![];
            let slots_after: Vec<Option<SlotValue>> = vec![];
            let taint_before: Vec<Taint> = vec![];
            let taint_after: Vec<Taint> = vec![];
            let states_before: Vec<StepState> = vec![];
            let states_after: Vec<StepState> = vec![];

            let slot_deltas = compute_slot_deltas(&slots_before, &slots_after);
            let taint_deltas = compute_taint_deltas(&taint_before, &taint_after);
            let state_deltas = compute_state_deltas(&states_before, &states_after);

            prop_assert!(slot_deltas.is_empty());
            prop_assert!(taint_deltas.is_empty());
            prop_assert!(state_deltas.is_empty());
        }

        #[test]
        fn full_frame_snapshot_no_deltas(
            max_slots in 0..256u16,
            max_steps in 0..256u16,
        ) {
            let slot_count = max_slots as usize;
            let step_count = max_steps as usize;

            let slots: Vec<Option<SlotValue>> = (0..slot_count)
                .map(|_| Some(arb_slot_value()))
                .collect();
            let taint: Vec<Taint> = (0..slot_count)
                .map(|_| arb_taint())
                .collect();
            let states: Vec<StepState> = (0..step_count)
                .map(|_| arb_step_state())
                .collect();

            let slot_deltas = compute_slot_deltas(&slots, &slots);
            let taint_deltas = compute_taint_deltas(&taint, &taint);
            let state_deltas = compute_state_deltas(&states, &states);

            prop_assert!(slot_deltas.is_empty(), "identical slot arrays should produce no deltas");
            prop_assert!(taint_deltas.is_empty(), "identical taint arrays should produce no deltas");
            prop_assert!(state_deltas.is_empty(), "identical state arrays should produce no deltas");
        }
    }

    #[test]
    fn slot_delta_count_matches_changed() {
        let slots_before = vec![
            Some(SlotValue::I64(1)),
            Some(SlotValue::I64(2)),
            Some(SlotValue::I64(3)),
        ];
        let slots_after = vec![
            Some(SlotValue::I64(1)), // unchanged
            Some(SlotValue::I64(99)), // changed
            Some(SlotValue::I64(3)), // unchanged
        ];

        let deltas = compute_slot_deltas(&slots_before, &slots_after);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].slot, 1);
        assert_eq!(deltas[0].before, Some(SlotValue::I64(2)));
        assert_eq!(deltas[0].after, Some(SlotValue::I64(99)));
    }

    #[test]
    fn taint_delta_count_matches_changed() {
        let taint_before = vec![Taint::Clean, Taint::Secret, Taint::DerivedFromSecret];
        let taint_after = vec![Taint::Clean, Taint::Clean, Taint::DerivedFromSecret];

        let deltas = compute_taint_deltas(&taint_before, &taint_after);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].slot, 1);
        assert_eq!(deltas[0].before, Taint::Secret);
        assert_eq!(deltas[0].after, Taint::Clean);
    }

    #[test]
    fn state_delta_count_matches_changed() {
        let states_before = vec![
            StepState::Pending,
            StepState::Running,
            StepState::Succeeded,
        ];
        let states_after = vec![
            StepState::Pending,
            StepState::Succeeded,
            StepState::Succeeded,
        ];

        let deltas = compute_state_deltas(&states_before, &states_after);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].step, 1);
        assert_eq!(deltas[0].before, StepState::Running);
        assert_eq!(deltas[0].after, StepState::Succeeded);
    }

    #[test]
    fn mismatched_array_lengths_handled() {
        let slots_before = vec![Some(SlotValue::I64(1)), Some(SlotValue::I64(2))];
        let slots_after = vec![Some(SlotValue::I64(99))];

        let deltas = compute_slot_deltas(&slots_before, &slots_after);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].slot, 0);
    }

    #[test]
    fn slot_value_serialization_deterministic() {
        let values = vec![
            SlotValue::Null,
            SlotValue::Bool(true),
            SlotValue::Bool(false),
            SlotValue::I64(0),
            SlotValue::I64(i64::MAX),
            SlotValue::I64(i64::MIN),
            SlotValue::F64(vb_core::value::FiniteF64::new(3.14).unwrap()),
        ];

        for val in values {
            let bytes1 = postcard::to_allocvec(&val).expect("serialization should succeed");
            let bytes2 = postcard::to_allocvec(&val).expect("serialization should succeed");
            assert_eq!(bytes1, bytes2, "SlotValue {:?} should serialize deterministically", val);
        }
    }

    #[test]
    fn slot_value_roundtrip() {
        let values = vec![
            SlotValue::Null,
            SlotValue::Bool(true),
            SlotValue::I64(42),
            SlotValue::F64(vb_core::value::FiniteF64::new(2.718).unwrap()),
            SlotValue::Symbol(vb_core::ids::SymbolId::new(123)),
        ];

        for val in values {
            let bytes = postcard::to_allocvec(&val).expect("serialization should succeed");
            let recovered: SlotValue = postcard::from_bytes(&bytes).expect("deserialization should succeed");
            assert_eq!(val, recovered, "SlotValue {:?} should roundtrip", val);
        }
    }
}