use super::{frame, outcome, replay_frame, shapes::Shape};
use crate::ids::StepIdx;
use crate::replay::{ReplayError, replay_step};
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNodeKind, CompiledWorkflow, SlotFlowFailure, WorkflowError};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn shape(bits: u8) -> Shape {
    Shape {
        two_branches: bits & 1 != 0,
        shifted_roots: bits & 2 != 0,
        reverse_roots: bits & 4 != 0,
        otherwise: bits & 8 != 0,
    }
}

fn plan(shape: Shape) -> TestResult<CompiledWorkflow> {
    let parts = shape.parts();
    assert_eq!(Shape::observe(&parts), Some(shape));
    let construction = CompiledWorkflow::try_from_parts(parts);
    assert_eq!(construction.as_ref().map(|_| ()), Ok(()));
    let plan = construction?;
    assert_eq!(plan.node_count(), shape.node_count());
    assert_eq!(plan.slot_count(), shape.slot_count());
    Ok(plan)
}

#[test]
fn public_dispatch_matches_helper_and_independent_oracle_all_64() -> TestResult {
    let mut cases = 0;
    for bits in 0_u8..16 {
        let shape = shape(bits);
        let plan = plan(shape)?;
        let node = plan.node(StepIdx::ZERO).ok_or("missing choice entry")?;
        let CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } = &node.kind
        else {
            return Err("entry is not ChooseSlot".into());
        };
        for (a, b) in [(false, false), (false, true), (true, false), (true, true)] {
            let (helper_result, helper_frame) =
                replay_frame(shape, branches, otherwise.step(), a, b)?;
            let original_frame = frame(shape, a, b)?;
            let checkpoint = shape.checkpoint(a, b);
            assert_eq!(checkpoint, original_frame.checkpoint_v1());
            let restoration = crate::frame::RunFrame::from_checkpoint_v1(checkpoint);
            assert_eq!(restoration.as_ref().map(|_| ()), Ok(()));
            let mut public_frame = restoration.map_err(|error| format!("{error:?}"))?;
            assert_eq!(public_frame, original_frame);
            let mut store = ValueStore::new();
            let _symbol = store.insert_symbol("untouched replay store")?;
            let before_store = store.clone();
            let public_result = outcome(replay_step(node, &mut public_frame, &mut store, &plan));
            assert_eq!(public_result, shape.expected(a, b));
            assert_eq!(helper_result, shape.expected(a, b));
            assert_eq!(public_result, helper_result);
            assert_eq!(public_frame, helper_frame);
            assert_eq!(store, before_store);
            cases += 1;
        }
    }
    assert_eq!(cases, 64);
    Ok(())
}

#[test]
fn missing_declared_roots_keep_exact_constructor_rejection() -> TestResult {
    let mut cases = 0;
    for bits in 0_u8..16 {
        let shape = shape(bits);
        let mut parts = shape.parts();
        parts.input_slots = Box::new([]);
        let Err(WorkflowError::SlotFlow(error)) = CompiledWorkflow::try_from_parts(parts) else {
            return Err("undeclared replay condition accepted or wrong rejection".into());
        };
        assert_eq!(error.step, StepIdx::ZERO);
        assert_eq!(error.slot, Some(shape.root(false)));
        assert_eq!(error.reason, SlotFlowFailure::UninitializedRead);
        cases += 1;
    }
    assert_eq!(cases, 16);
    Ok(())
}

#[test]
fn public_and_helper_reject_non_boolean_conditions_without_state_changes() -> TestResult {
    let mut cases = 0;
    for bits in 0_u8..16 {
        let shape = shape(bits);
        let plan = plan(shape)?;
        let node = plan.node(StepIdx::ZERO).ok_or("missing choice entry")?;
        let CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } = &node.kind
        else {
            return Err("entry is not ChooseSlot".into());
        };
        let mut public_frame = frame(shape, false, true)?;
        public_frame.write_slot(shape.root(false), SlotValue::I64(42))?;
        let before = public_frame.clone();
        let mut helper_frame = before.clone();
        let mut store = ValueStore::new();
        let _symbol = store.insert_symbol("untouched rejected replay")?;
        let before_store = store.clone();
        let public_result = outcome(replay_step(node, &mut public_frame, &mut store, &plan));
        let helper_result = outcome(super::super::choose::replay_choose_slot(
            &mut helper_frame,
            branches,
            otherwise.step(),
        ));
        let expected = Err(ReplayError::Internal {
            reason: "choose_slot condition is not boolean",
        });
        assert_eq!(public_result, expected);
        assert_eq!(helper_result, expected);
        assert_eq!(public_frame, before);
        assert_eq!(helper_frame, before);
        assert_eq!(store, before_store);
        cases += 1;
    }
    assert_eq!(cases, 16);
    Ok(())
}
