#![forbid(unsafe_code)]
//! Together branch failure policy: routing, reservation release, terminal checks.

use super::support::*;
use vb_core::{CompiledNodeKind, SlotValue};
use vb_storage::JournalEvent;
use vb_storage::recovery::{
    RecoveryError, RecoveryTerminalState, recover_runtime_summary_with_expected,
};

#[test]
fn failing_branch_routes_to_handler_and_cold_recovery_rejects_wrong_terminal() -> TestResult {
    let plan = compile_workflow(FAILING_FANOUT)?;
    let zero = plan
        .input_slots()
        .first()
        .map(|input| input.slot)
        .ok_or("input slot missing")?;
    let live = live(FAILING_FANOUT, Box::new([(zero, SlotValue::I64(0))]))?;
    let events = events(&live)?;
    let fault = live
        .plan
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::EvalExpr { expr }
                if live.plan.expression(expr).is_some_and(|program| {
                    program
                        .ops
                        .iter()
                        .any(|op| matches!(op, vb_core::ExprOp::Div))
                }) =>
            {
                Some(node.id)
            }
            _ => None,
        })
        .ok_or("failing division step missing")?;
    let join = live
        .plan
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, CompiledNodeKind::TogetherJoin { .. }).then_some(node.id)
        })
        .ok_or("join missing")?;
    assert_eq!(
        failed(&events),
        vec![fault],
        "only the failing branch fails"
    );
    assert!(
        !succeeded(&events).contains(&join),
        "abandoned join must not record success"
    );
    let skipped = live
        .plan
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::SetConst { value }
                if live.plan.constant(value) == Some(&vb_core::ConstValue::I64(99)) =>
            {
                Some(node.id)
            }
            _ => None,
        })
        .ok_or("skipped step missing")?;
    assert!(
        !succeeded(&events).contains(&skipped),
        "failure policy skips the unreachable step"
    );
    assert!(
        live.state.frame.pc() != skipped,
        "control continues at the error handler, not the unreachable step"
    );
    assert_eq!(
        live.state.frame.parallel_in_flight(),
        0,
        "error routing retires the Together reservation"
    );

    let Live {
        journal, directory, ..
    } = live;
    drop(journal);
    let reopened = open_journal(&directory)?;
    let after = reopened.events_for_run_full(RUN)?;
    let expected = after
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => {
                Some(RecoveryTerminalState::Finished { result: *result })
            }
            _ => None,
        })
        .ok_or("terminal receipt missing")?;
    recover_runtime_summary_with_expected(&reopened, RUN, expected)?;
    assert!(matches!(
        recover_runtime_summary_with_expected(&reopened, RUN, RecoveryTerminalState::Failed),
        Err(RecoveryError::TerminalStateMismatch { .. })
    ));
    Ok(())
}

fn compile_workflow(source: &str) -> TestResult<vb_core::CompiledWorkflow> {
    vb_compile::compile_workflow(source.as_bytes()).map_err(|errors| format!("{errors:?}").into())
}
