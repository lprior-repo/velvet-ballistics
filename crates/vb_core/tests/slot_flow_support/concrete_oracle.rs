use super::*;
use vb_core::workflow::InputSlotKind;
use vb_core::{EngineSignal, RunFrame, SlotValue, StepBudget, ValueStore, drive_deterministic};

#[test]
fn branch_assignment_matches_concrete_execution_for_each_input() -> TestResult {
    for initialized in [false, true] {
        for true_writes in [false, true] {
            for false_writes in [false, true] {
                let initialize = if initialized {
                    K::SetConst {
                        value: vb_core::ids::ConstIdx::new(0),
                    }
                } else {
                    K::Nop
                };
                let yes = if true_writes {
                    set(2, 0, 1, 4)
                } else {
                    node(2, K::Nop, None, Some(4))
                };
                let no = if false_writes {
                    set(3, 0, 1, 4)
                } else {
                    node(3, K::Nop, None, Some(4))
                };
                let mut p = parts(
                    vec![
                        node(0, initialize, initialized.then_some(1), Some(1)),
                        choose(1, 2, 3),
                        yes,
                        no,
                        finish(4, 1),
                    ],
                    2,
                );
                declare(&mut p, 0, InputSlotKind::Bool);
                let accepted = CompiledWorkflow::try_from_parts(p.clone()).is_ok();
                let unchecked = CompiledWorkflow::from_parts_unchecked(p);
                let mut all_executions_finish = true;
                for condition in [false, true] {
                    let mut run = RunFrame::new(vb_core::ids::RunId::new(1), step(0), 5, 2)?;
                    run.write_slot(slot(0), SlotValue::Bool(condition))?;
                    let result = drive_deterministic(
                        &unchecked,
                        &mut run,
                        &mut StepBudget::new(8),
                        &mut ValueStore::new(),
                    );
                    all_executions_finish &=
                        matches!(result, Ok(EngineSignal::Finished(SlotValue::I64(1), _)));
                }
                assert_eq!(
                    accepted, all_executions_finish,
                    "initialized={initialized}, true_writes={true_writes}, false_writes={false_writes}"
                );
            }
        }
    }
    Ok(())
}
