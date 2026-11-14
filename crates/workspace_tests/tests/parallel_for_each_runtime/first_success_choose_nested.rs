use super::*;
use vb_core::workflow::{CompiledNodeKind as K, node};

fn nested_plan(choose: bool) -> Result<CompiledWorkflow, String> {
    let mut parts = checked_plan(false)?.to_parts();
    let failing = if choose {
        K::Choose {
            branches: Box::new([vb_core::workflow::ExprBranch {
                condition: vb_core::ExprIdx::new(0),
                target: StepIdx::new(3),
            }]),
            otherwise: Some(StepIdx::new(3)).into(),
        }
    } else {
        K::EvalExpr {
            expr: vb_core::ExprIdx::new(0),
        }
    };
    let n = |id, kind, output: Option<u16>, next: Option<u16>| {
        node(
            StepIdx::new(id),
            kind,
            output.map(SlotIdx::new),
            next.map(StepIdx::new),
        )
    };
    parts.nodes = Box::new([
        n(
            0,
            K::FirstSuccessStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(6)]),
                join: StepIdx::new(8),
            },
            None,
            None,
        ),
        n(
            1,
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(2)]),
                join: StepIdx::new(4),
            },
            None,
            None,
        ),
        n(
            2,
            failing,
            if choose { None } else { Some(0) },
            if choose { None } else { Some(3) },
        ),
        n(
            3,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(1),
                result: if choose { None } else { Some(SlotIdx::ZERO) },
            },
            None,
            Some(4),
        ),
        n(
            4,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(1),
            },
            Some(1),
            Some(5),
        ),
        n(
            5,
            K::ConcurrentLaneEnd {
                owner: StepIdx::ZERO,
                result: Some(SlotIdx::new(1)),
            },
            None,
            Some(8),
        ),
        n(
            6,
            K::SetConst {
                value: vb_core::ConstIdx::new(1),
            },
            Some(0),
            Some(7),
        ),
        n(
            7,
            K::ConcurrentLaneEnd {
                owner: StepIdx::ZERO,
                result: Some(SlotIdx::ZERO),
            },
            None,
            Some(8),
        ),
        n(
            8,
            K::ConcurrentGroupJoin {
                owner: StepIdx::ZERO,
            },
            Some(2),
            Some(9),
        ),
        n(
            9,
            K::Finish {
                result: SlotIdx::new(2),
            },
            None,
            None,
        ),
    ]);
    CompiledWorkflow::try_from_concurrent_parts(parts).map_err(|e| format!("{e:?}"))
}

fn check_live(plan: &CompiledWorkflow, choose: bool, quantum: u64) -> Result<(), String> {
    let mut run = frame(plan)?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    for _ in 0..24 {
        let result = drive_deterministic_full(
            plan,
            &mut run,
            &mut StepBudget::new(quantum),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &vb_core::capability::CapabilitySet::empty(),
            1,
        );
        if result == Ok(RuntimeSignal::StepBudgetExhausted) {
            continue;
        }
        if choose {
            assert_eq!(
                result,
                Err(vb_runtime::engine::RuntimeEngineError::Core(
                    vb_core::EngineError::DivisionByZero
                ))
            );
            assert_eq!(
                run.step_state(StepIdx::new(6)).map_err(|e| e.to_string())?,
                vb_core::StepState::Pending
            );
            assert_eq!(
                run.read_slot(SlotIdx::ZERO),
                Err(vb_core::EngineError::SlotUninitialized {
                    slot: SlotIdx::ZERO
                })
            );
            assert!(!evidence.drain().iter().any(|event| matches!(event,
                vb_runtime::engine::types::EvidenceEvent::StepStarted { step } if *step == StepIdx::new(6))));
        } else {
            assert_eq!(result, Ok(RuntimeSignal::Finished(SlotValue::I64(20))));
        }
        return Ok(());
    }
    Err("nested failure did not terminate".into())
}

#[test]
fn first_success_nested_together_distinguishes_eval_and_choose_failures() -> Result<(), String> {
    for choose in [false, true] {
        let plan = nested_plan(choose)?;
        for quantum in [1, 100] {
            check_live(&plan, choose, quantum)?;
        }
        let mut run = frame(&plan)?;
        let result = replay_step_with_collect(
            plan.node(plan.entry()).ok_or("entry")?,
            &mut run,
            &mut ValueStore::new(),
            &plan,
            &mut ReplayCollectStates::new(),
        );
        if choose {
            assert_eq!(
                result.err(),
                Some(ReplayError::ExpressionEvalFailed {
                    step: StepIdx::new(2)
                })
            );
            assert_eq!(
                run.read_slot(SlotIdx::ZERO),
                Err(vb_core::EngineError::SlotUninitialized {
                    slot: SlotIdx::ZERO
                })
            );
        } else {
            let vb_core::replay::step::ReplayAction::Continue(next) =
                result.map_err(|e| format!("{e:?}"))?
            else {
                return Err("nested FirstSuccess did not reach its join".into());
            };
            assert_eq!(next, StepIdx::new(8));
            assert_eq!(run.read_slot(SlotIdx::new(2)), Ok(&SlotValue::I64(20)));
        }
    }
    Ok(())
}
