#![forbid(unsafe_code)]
//! Bounded, exhaustive ChooseSlot shapes shared by replay proofs and fixture tests.
use super::{ReplayAction, ReplayError, step::replay_step};
use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, SlotBranch, WorkflowParts, node,
};
#[derive(Clone, Copy)]
struct Shape {
    two_branches: bool,
    shifted_roots: bool,
    reverse_roots: bool,
    otherwise: bool,
}
impl Shape {
    fn branch_count(self) -> u16 {
        if self.two_branches { 2 } else { 1 }
    }
    fn root(self, second: bool) -> SlotIdx {
        SlotIdx::new(u16::from(self.shifted_roots) + u16::from(second != self.reverse_roots))
    }
    fn parts(self) -> WorkflowParts {
        let first = SlotBranch {
            condition: self.root(false),
            target: StepIdx::new(1),
        };
        let branches: Box<[SlotBranch]> = if self.two_branches {
            Box::new([
                first,
                SlotBranch {
                    condition: self.root(true),
                    target: StepIdx::new(2),
                },
            ])
        } else {
            Box::new([first])
        };
        let choose = node(
            StepIdx::ZERO,
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise: self
                    .otherwise
                    .then_some(StepIdx::new(self.branch_count() + 1)),
            },
            None,
            None,
        );
        let terminal = |id| node(StepIdx::new(id), CompiledNodeKind::Nop, None, None);
        let nodes: Box<[crate::workflow::CompiledNode]> = match (self.two_branches, self.otherwise)
        {
            (false, false) => Box::new([choose, terminal(1)]),
            (true, true) => Box::new([choose, terminal(1), terminal(2), terminal(3)]),
            _ => Box::new([choose, terminal(1), terminal(2)]),
        };
        let inputs = [false, true].map(|second| CompiledInputSlot {
            slot: SlotIdx::new(u16::from(self.shifted_roots) + u16::from(second)),
            kind: InputSlotKind::Bool,
            constraint: CompiledInputConstraint {
                optional: false,
                ..CompiledInputConstraint::unconstrained()
            },
        });
        let slot_count = 2 + u16::from(self.shifted_roots);
        WorkflowParts {
            semantic_source_identity: None,
            source_metadata: None,
            name: "replay_choose".into(),
            digest: crate::ids::WorkflowDigest::from_bytes([0; 32]),
            nodes,
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract {
                max_slots: slot_count,
                ..ResourceContract::DEFAULT
            },
            step_names: Box::new([]),
            input_slots: Box::from(inputs),
        }
    }
    fn expected(self, a: bool, b: bool) -> Result<StepIdx, ReplayError> {
        let target = if a {
            Some(1)
        } else if self.two_branches && b {
            Some(2)
        } else if self.otherwise {
            Some(self.branch_count() + 1)
        } else {
            None
        };
        target.map(StepIdx::new).ok_or(ReplayError::Internal {
            reason: "choose_slot no branch matched and no otherwise",
        })
    }
}
fn replay(
    plan: &CompiledWorkflow,
    shape: Shape,
    a: bool,
    b: bool,
) -> Result<(Result<StepIdx, ReplayError>, StepIdx), crate::CoreError> {
    let id = RunId::new(0);
    let mut run = RunFrame::new(id, plan.entry(), plan.node_count(), plan.slot_count())?;
    run.write_slot(shape.root(false), SlotValue::Bool(a))?;
    run.write_slot(shape.root(true), SlotValue::Bool(b))?;
    let result = replay_step(
        plan.node(StepIdx::ZERO)
            .ok_or(crate::CoreError::InvalidCompiledWorkflow {
                reason: "replay fixture entry missing",
            })?,
        &mut run,
        &mut ValueStore::new(),
        plan,
    );
    let outcome = match result {
        Ok(ReplayAction::Continue(target)) => Ok(target),
        Err(error) => Err(error),
        _ => Err(ReplayError::Internal {
            reason: "unexpected action variant",
        }),
    };
    Ok((outcome, run.pc()))
}
#[cfg(kani)]
impl kani::Arbitrary for Shape {
    fn any() -> Self {
        Self {
            two_branches: kani::any(),
            shifted_roots: kani::any(),
            reverse_roots: kani::any(),
            otherwise: kani::any(),
        }
    }
}
#[cfg(kani)]
mod verification {
    use super::*;
    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_replay_choose_slot_two_branches_no_panic() {
        let shape: Shape = kani::any();
        let a: bool = kani::any();
        let b: bool = kani::any();
        let construction = CompiledWorkflow::try_from_parts(shape.parts());
        assert_eq!(construction.as_ref().map(|_| ()), Ok(()));
        let Ok(plan) = construction else {
            return;
        };
        let execution = replay(&plan, shape, a, b);
        assert_eq!(execution.as_ref().map(|_| ()), Ok(()));
        let Ok((result, pc)) = execution else {
            return;
        };
        assert_eq!(result, shape.expected(a, b));
        match &result {
            Ok(target) => assert_eq!(pc, *target),
            Err(ReplayError::Internal { reason }) => {
                assert_eq!(*reason, "choose_slot no branch matched and no otherwise")
            }
            _ => assert_eq!(result, shape.expected(a, b)),
        }
        kani::cover!(
            shape.two_branches && result.is_ok(),
            "two-branch replay reached"
        );
        kani::cover!(
            !shape.two_branches && result.is_ok(),
            "one-branch replay reached"
        );
        kani::cover!(
            shape.otherwise && !a && !b && result.is_ok(),
            "otherwise replay reached"
        );
        kani::cover!(
            !shape.otherwise && !a && !b && result.is_err(),
            "typed no-match error reached"
        );
    }
    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_choose_slot_output_in_input_set() {
        let mut shape: Shape = kani::any();
        shape.otherwise = true;
        let a: bool = kani::any();
        let b: bool = kani::any();
        let construction = CompiledWorkflow::try_from_parts(shape.parts());
        assert_eq!(construction.as_ref().map(|_| ()), Ok(()));
        let Ok(plan) = construction else {
            return;
        };
        let execution = replay(&plan, shape, a, b);
        assert_eq!(execution.as_ref().map(|_| ()), Ok(()));
        let Ok((result, pc)) = execution else {
            return;
        };
        assert_eq!(result, shape.expected(a, b));
        match &result {
            Ok(target) => {
                let valid = *target == StepIdx::new(1)
                    || *target == StepIdx::new(2)
                    || *target == StepIdx::new(3);
                assert!(valid);
                assert!(target.get() <= shape.branch_count() + 1);
                assert_eq!(pc, *target);
            }
            Err(_) => assert_eq!(result, shape.expected(a, b)),
        }
        kani::cover!(a, "first branch replay reached");
        kani::cover!(
            !a && b && shape.two_branches,
            "second branch replay reached"
        );
        kani::cover!(!a && !b, "fallback replay reached");
        kani::cover!(
            shape.shifted_roots && shape.reverse_roots,
            "shifted reversed roots reached"
        );
    }
    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_replay_deterministic_for_same_input() {
        let mut shape: Shape = kani::any();
        shape.otherwise = true;
        let a: bool = kani::any();
        let b: bool = kani::any();
        let construction = CompiledWorkflow::try_from_parts(shape.parts());
        assert_eq!(construction.as_ref().map(|_| ()), Ok(()));
        let Ok(plan) = construction else {
            return;
        };
        let execution_a = replay(&plan, shape, a, b);
        let execution_b = replay(&plan, shape, a, b);
        assert_eq!(execution_a.as_ref().map(|_| ()), Ok(()));
        assert_eq!(execution_b.as_ref().map(|_| ()), Ok(()));
        let (Ok(result_a), Ok(result_b)) = (execution_a, execution_b) else {
            return;
        };
        assert_eq!(result_a.0, shape.expected(a, b));
        assert_eq!(result_a, result_b);
        match (&result_a.0, &result_b.0) {
            (Ok(a), Ok(b)) => assert_eq!(a, b),
            _ => assert_eq!(result_a.0, shape.expected(a, b)),
        }
        kani::cover!(
            a && shape.two_branches,
            "deterministic two-branch success reached"
        );
        kani::cover!(
            !a && !b && !shape.two_branches,
            "deterministic one-branch fallback reached"
        );
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    #[test]
    fn every_generated_shape_validates_and_replays_every_boolean_pair()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut cases = 0;
        for bits in 0_u8..16 {
            let shape = Shape {
                two_branches: bits & 1 != 0,
                shifted_roots: bits & 2 != 0,
                reverse_roots: bits & 4 != 0,
                otherwise: bits & 8 != 0,
            };
            let plan = CompiledWorkflow::try_from_parts(shape.parts())?;
            let mut missing_roots = shape.parts();
            missing_roots.input_slots = Box::new([]);
            let Err(crate::workflow::WorkflowError::SlotFlow(error)) =
                CompiledWorkflow::try_from_parts(missing_roots)
            else {
                return Err("undeclared replay condition accepted or wrong rejection".into());
            };
            assert_eq!(error.step, StepIdx::ZERO);
            assert_eq!(error.slot, Some(shape.root(false)));
            assert_eq!(
                error.reason,
                crate::workflow::SlotFlowFailure::UninitializedRead
            );
            for (a, b) in [(false, false), (false, true), (true, false), (true, true)] {
                assert_eq!(replay(&plan, shape, a, b)?.0, shape.expected(a, b));
                cases += 1;
            }
        }
        assert_eq!(cases, 64);
        Ok(())
    }
}
