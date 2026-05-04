#![forbid(unsafe_code)]

//! Deterministic drive loop for runtime engine.

use vb_core::action::ActionContract;
use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::execute_node_full;
use crate::engine::helpers::mark_step_after_signal;
use crate::engine::types::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal,
};
use crate::primitives::collect::CollectStates;

fn compute_max_parallel_in_flight(plan: &CompiledWorkflow) -> RuntimeEngineResult<u16> {
    let mut max_branches: u16 = 0;
    for i in 0..plan.node_count() {
        let step = StepIdx::new(i);
        if let Some(node) = plan.node(step)
            && let CompiledNodeKind::TogetherStart { branches, .. } = &node.kind
        {
            let branch_count =
                u16::try_from(branches.len()).map_err(|_| RuntimeEngineError::BranchLimitExceeded {
                    max: u16::MAX.into(),
                    requested: branches.len(),
                })?;
            if branch_count > max_branches {
                max_branches = branch_count;
            }
        }
    }
    Ok(max_branches)
}

/// Enhanced drive loop that handles all node kinds including
/// iteration, compound, action, and suspension primitives.
///
/// Collects evidence events (StepStarted/StepSucceeded) for every step
/// executed during the drive loop. The caller drains these events to emit
/// them to the journal and trace ring.
#[allow(clippy::too_many_arguments)]
pub fn drive_deterministic_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
    evidence: &mut EvidenceCollector,
    collect_states: &mut CollectStates,
    granted: &vb_core::capability::CapabilitySet,
) -> RuntimeEngineResult<RuntimeSignal> {
    let max_parallel = compute_max_parallel_in_flight(plan)?;
    run.set_max_parallel_in_flight(max_parallel)
        .map_err(RuntimeEngineError::Core)?;

    loop {
        if !budget.try_take().map_err(RuntimeEngineError::Core)? {
            return Ok(RuntimeSignal::StepBudgetExhausted);
        }

        let pc = run.pc();
        let node = plan
            .node(pc)
            .ok_or(EngineError::InvalidProgramCounter { step: pc })?;

        // Evidence chain: emit StepStarted before execution.
        evidence.push_step_started(pc);

        run.mark_running(pc).map_err(RuntimeEngineError::Core)?;

        let signal = execute_node_full(
            plan,
            run,
            store,
            node,
            contracts,
            retry_policy,
            collect_states,
            granted,
        )?;

        match mark_step_after_signal(run, pc, &signal) {
            Ok(()) => {}
            Err(e) => return Err(RuntimeEngineError::Core(e)),
        }

        // Evidence chain: emit SlotWritten with actual value for all slot writes,
        // including internal expression evaluations (SetConst, Copy, EvalExpr,
        // BuildObject, BuildList). This satisfies Phase 40/44 requirement.
        if let Some(slot) = node.output
            && let Ok(value) = run.read_slot(slot)
        {
            evidence.push_slot_written(slot, *value);
        }

        // Evidence chain: emit StepSucceeded only when the step actually succeeded.
        // For signals like StepBudgetExhausted, AwaitingAction, AwaitingWait,
        // and AwaitingAsk, the step did not complete successfully, so we must
        // not emit a spurious StepSucceeded event.
        match &signal {
            RuntimeSignal::Continue | RuntimeSignal::Finished(_) => {
                evidence.push_step_succeeded(pc, node.output);
            }
            RuntimeSignal::StepBudgetExhausted
            | RuntimeSignal::AwaitingAction(_)
            | RuntimeSignal::AwaitingWait
            | RuntimeSignal::AwaitingAsk => {}
        }

        match signal {
            RuntimeSignal::Continue => {}
            other => return Ok(other),
        }
    }
}

/// Backward-compatible drive loop matching the original drive_with_actions signature.
pub fn drive_with_actions(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    drive_deterministic_full(
        plan,
        run,
        budget,
        &mut store,
        contracts,
        retry_policy,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
    )
}

#[cfg(test)]
mod tests {
    use vb_core::capability::{Capability, CapabilitySet};
    use vb_core::engine::StepBudget;
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
    use vb_core::value::{ConstValue, SlotValue};
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, SlotBranch, WorkflowParts};
    use crate::engine::drive::{drive_deterministic_full, drive_with_actions};
    use crate::engine::types::{EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal};
    use crate::primitives::collect::CollectStates;

    fn cn(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
        CompiledNode { id: StepIdx::new(id), output: output.map(SlotIdx::new),
            next: next.map(StepIdx::new), on_error: None, error_slot: None, kind }
    }
    fn fin(id: u16, result: u16) -> CompiledNode {
        cn(id, None, None, CompiledNodeKind::Finish { result: SlotIdx::new(result) })
    }
    fn nop(id: u16, nx: u16) -> CompiledNode {
        cn(id, None, Some(nx), CompiledNodeKind::Nop)
    }
    fn setc(id: u16, cid: u16, out: u16, nx: u16) -> CompiledNode {
        cn(id, Some(out), Some(nx), CompiledNodeKind::SetConst { value: ConstIdx::new(cid) })
    }
    fn cpy(id: u16, src: u16, out: u16, nx: u16) -> CompiledNode {
        cn(id, Some(out), Some(nx), CompiledNodeKind::Copy { source: SlotIdx::new(src) })
    }
    fn don(id: u16, action: u16, inp: u16) -> CompiledNode {
        cn(id, None, None, CompiledNodeKind::Do { action: ActionId::new(action), input: SlotIdx::new(inp) })
    }
    fn cslot(id: u16, branches: Box<[SlotBranch]>, otw: Option<u16>) -> CompiledNode {
        cn(id, None, otw, CompiledNodeKind::ChooseSlot { branches, otherwise: otw.map(StepIdx::new) })
    }
    fn askn(id: u16, prompt: u16) -> CompiledNode {
        cn(id, None, None, CompiledNodeKind::Ask { prompt: SlotIdx::new(prompt), timeout_slot: None })
    }
    fn wuntil(id: u16, dl: u16) -> CompiledNode {
        cn(id, None, None, CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::new(dl) })
    }
    fn errh(id: u16, body: u16, handler: u16, eslot: Option<u16>) -> CompiledNode {
        cn(id, None, None, CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(body), handler: StepIdx::new(handler),
            error_slot: eslot.map(SlotIdx::new) })
    }
    fn tog(id: u16, branches: Box<[u16]>, join: u16) -> CompiledNode {
        let br: Box<[StepIdx]> = branches.iter().map(|b| StepIdx::new(*b)).collect();
        cn(id, None, None, CompiledNodeKind::TogetherStart { branches: br, join: StepIdx::new(join) })
    }

    fn mkwf(nodes: Vec<CompiledNode>, sc: u16) -> Result<CompiledWorkflow, String> {
        mkwfc(nodes, sc, vec![])
    }
    fn mkwfc(nodes: Vec<CompiledNode>, sc: u16, consts: Vec<ConstValue>) -> Result<CompiledWorkflow, String> {
        let names: Box<[Box<str>]> = (0..nodes.len()).map(|i| format!("s{i}").into_boxed_str()).collect();
        let parts = WorkflowParts {
            name: "test".into(), digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::from([]), accessors: Box::from([]),
            constants: consts.into_boxed_slice(),
            slot_count: sc, symbols_count: 0, entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: names,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| format!("{e}"))
    }
    fn mkr(steps: u16, slots: u16) -> Result<RunFrame, String> {
        RunFrame::new(RunId::new(1), StepIdx::new(0), steps, slots).map_err(|e| format!("{e}"))
    }
    fn dd(wf: &CompiledWorkflow, r: &mut RunFrame, b: &mut StepBudget) -> Result<RuntimeSignal, String> {
        let mut store = ValueStore::new();
        let mut ev = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        drive_deterministic_full(wf, r, b, &mut store, &[], RetryPolicy::NEVER, &mut ev, &mut cs, &CapabilitySet::empty()).map_err(|e| format!("{e}"))
    }
    fn ddg(wf: &CompiledWorkflow, r: &mut RunFrame, b: &mut StepBudget, g: &CapabilitySet) -> Result<RuntimeSignal, String> {
        let mut store = ValueStore::new();
        let mut ev = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        drive_deterministic_full(wf, r, b, &mut store, &[], RetryPolicy::NEVER, &mut ev, &mut cs, g).map_err(|e| format!("{e}"))
    }
    fn dde(wf: &CompiledWorkflow, r: &mut RunFrame, b: &mut StepBudget, ev: &mut EvidenceCollector, g: &CapabilitySet) -> Result<RuntimeSignal, String> {
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        drive_deterministic_full(wf, r, b, &mut store, &[], RetryPolicy::NEVER, ev, &mut cs, g).map_err(|e| format!("{e}"))
    }
    fn ws(r: &mut RunFrame, s: u16, v: SlotValue) -> Result<(), String> {
        r.write_slot(SlotIdx::new(s), v).map_err(|e| format!("{e}"))
    }
    fn gr(name: &str, action: u16) -> CapabilitySet {
        CapabilitySet::from_grants(Box::from([Capability::new(name.into(), ActionId::new(action))]))
    }

    #[test]
    fn cat1_nop_continues() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(0)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn cat1_set_const_writes() -> Result<(), String> {
        let wf = mkwfc(vec![setc(0, 0, 0, 1), fin(1, 0)], 1, vec![ConstValue::I64(42)])?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(42)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn cat1_copy_propagates() -> Result<(), String> {
        let wf = mkwf(vec![cpy(0, 1, 0, 1), fin(1, 0)], 2)?;
        let mut r = mkr(2, 2)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        ws(&mut r, 1, SlotValue::I64(99))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(99)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn cat1_finish_immediate() -> Result<(), String> {
        let wf = mkwf(vec![fin(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::Bool(true))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::Bool(true)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn cat2_choose_slot_matching() -> Result<(), String> {
        let branches = Box::from([
            SlotBranch { condition: SlotIdx::new(0), target: StepIdx::new(1) },
            SlotBranch { condition: SlotIdx::new(1), target: StepIdx::new(2) },
        ]);
        let wf = mkwf(vec![cslot(0, branches, None), fin(1, 2), fin(2, 2)], 3)?;
        let mut r = mkr(3, 3)?;
        ws(&mut r, 0, SlotValue::Bool(true))?;
        ws(&mut r, 1, SlotValue::Bool(false))?;
        ws(&mut r, 2, SlotValue::I64(10))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(10)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn cat2_choose_slot_no_match_errors() -> Result<(), String> {
        let branches = Box::from([SlotBranch { condition: SlotIdx::new(0), target: StepIdx::new(1) }]);
        let wf = mkwf(vec![cslot(0, branches, None), fin(1, 1)], 2)?;
        let mut r = mkr(2, 2)?;
        ws(&mut r, 0, SlotValue::Bool(false))?;
        ws(&mut r, 1, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let result = dd(&wf, &mut r, &mut b);
        if result.is_ok() { return Err("expected error for no matching branch".into()); }
        Ok(())
    }

    #[test]
    fn cat2_choose_otherwise() -> Result<(), String> {
        let branches = Box::from([SlotBranch { condition: SlotIdx::new(0), target: StepIdx::new(1) }]);
        let wf = mkwf(vec![cslot(0, branches, Some(2)), fin(1, 1), fin(2, 1)], 2)?;
        let mut r = mkr(3, 2)?;
        ws(&mut r, 0, SlotValue::Bool(false))?;
        ws(&mut r, 1, SlotValue::I64(77))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(77)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn cat3_wait_until_awaiting() -> Result<(), String> {
        let wf = mkwf(vec![wuntil(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(1000))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::AwaitingWait => Ok(()), _ => Err("expected AwaitingWait".into()) }
    }

    #[test]
    fn cat3_ask_awaiting() -> Result<(), String> {
        let wf = mkwf(vec![askn(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::Symbol(SymbolId::new(1)))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::AwaitingAsk => Ok(()), _ => Err("expected AwaitingAsk".into()) }
    }

    #[test]
    fn cat4_error_handler_body_succeeds() -> Result<(), String> {
        let wf = mkwf(vec![errh(0, 1, 2, None), nop(1, 3), fin(2, 0), fin(3, 0)], 1)?;
        let mut r = mkr(4, 1)?;
        ws(&mut r, 0, SlotValue::I64(55))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(55)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn cat5_budget_exhausted() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), nop(1, 2), fin(2, 0)], 1)?;
        let mut r = mkr(3, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(2);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::StepBudgetExhausted => Ok(()), other => Err(format!("expected StepBudgetExhausted, got {other:?}")) }
    }

    #[test]
    fn cat6_evidence_step_events() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let started = events.iter().filter(|e| matches!(e, EvidenceEvent::StepStarted { .. })).count();
        if started < 2 { return Err(format!("expected >= 2 StepStarted, got {started}")); }
        Ok(())
    }

    #[test]
    fn cat6_evidence_slot_written() -> Result<(), String> {
        let wf = mkwfc(vec![setc(0, 0, 0, 1), fin(1, 0)], 1, vec![ConstValue::I64(7)])?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let writes: Vec<_> = events.iter().filter(|e| matches!(e, EvidenceEvent::SlotWritten { .. })).collect();
        if writes.is_empty() { return Err("expected at least one SlotWritten".into()); }
        Ok(())
    }

    #[test]
    fn cat7_multi_step_chain() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), nop(1, 2), nop(2, 3), fin(3, 0)], 1)?;
        let mut r = mkr(4, 1)?;
        ws(&mut r, 0, SlotValue::Bool(false))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::Bool(false)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn cat8_set_const_advances_pc() -> Result<(), String> {
        let wf = mkwfc(vec![setc(0, 0, 0, 1), fin(1, 0)], 1, vec![ConstValue::I64(5)])?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(_) => Ok(()), other => Err(format!("expected Finished, got {other:?}")) }
    }

    #[test]
    fn cat8_nop_advances_pc() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::Null)?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::Finished(_) => Ok(()), other => Err(format!("expected Finished, got {other:?}")) }
    }

    #[test]
    fn cat9_set_const_evidence_value() -> Result<(), String> {
        let wf = mkwfc(vec![setc(0, 0, 0, 1), fin(1, 0)], 1, vec![ConstValue::I64(33)])?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let found = events.iter().any(|e| match e {
            EvidenceEvent::SlotWritten { slot, value } => *slot == SlotIdx::new(0) && *value == SlotValue::I64(33),
            _ => false,
        });
        if !found { return Err("expected SlotWritten(0, I64(33))".into()); }
        Ok(())
    }

    #[test]
    fn cat9_copy_evidence_value() -> Result<(), String> {
        let wf = mkwf(vec![cpy(0, 1, 0, 1), fin(1, 0)], 2)?;
        let mut r = mkr(2, 2)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        ws(&mut r, 1, SlotValue::I64(88))?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let found = events.iter().any(|e| match e {
            EvidenceEvent::SlotWritten { slot, value } => *slot == SlotIdx::new(0) && *value == SlotValue::I64(88),
            _ => false,
        });
        if !found { return Err("expected SlotWritten(0, I64(88))".into()); }
        Ok(())
    }

    #[test]
    fn cat10_do_awaiting_action() -> Result<(), String> {
        let wf = mkwf(vec![don(0, 1, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let g = gr("t", 1);
        let sig = ddg(&wf, &mut r, &mut b, &g)?;
        match sig { RuntimeSignal::AwaitingAction(_) => Ok(()), other => Err(format!("expected AwaitingAction, got {other:?}")) }
    }

    // Do node without contract always succeeds (no capability enforcement without contract)
    #[test]
    fn cat10_do_without_contract_succeeds() -> Result<(), String> {
        let wf = mkwf(vec![don(0, 1, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::AwaitingAction(_) => Ok(()), other => Err(format!("expected AwaitingAction, got {other:?}")) }
    }

    #[test]
    fn bonus_compat() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let sig = drive_with_actions(&wf, &mut r, &mut b, &[], RetryPolicy::NEVER).map_err(|e| format!("{e}"))?;
        match sig { RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(0)), _ => return Err("expected Finished".into()) }
        Ok(())
    }

    #[test]
    fn bonus_together() -> Result<(), String> {
        let wf = mkwf(vec![tog(0, Box::from([1u16, 2]), 3), fin(1, 1), fin(2, 1), fin(3, 1)], 2)?;
        let mut r = mkr(4, 2)?;
        ws(&mut r, 1, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        let _ = dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty());
        Ok(())
    }

    #[test]
    fn bonus_zero_budget() -> Result<(), String> {
        let wf = mkwf(vec![fin(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(0);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig { RuntimeSignal::StepBudgetExhausted => Ok(()), other => Err(format!("expected StepBudgetExhausted, got {other:?}")) }
    }

    #[test]
    fn bonus_evidence_order() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::Null)?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        match events.first() {
            Some(EvidenceEvent::StepStarted { step }) => assert_eq!(*step, StepIdx::new(0)),
            other => return Err(format!("expected StepStarted(0) first, got {other:?}")),
        }
        Ok(())
    }

    // =====================================================================
    // drive_with_actions wrapper tests
    // =====================================================================

    /// drive_with_actions with empty contracts and a Finish-only workflow
    /// returns Finished.
    #[test]
    fn dwa_empty_contracts_returns_finished() -> Result<(), String> {
        let wf = mkwf(vec![fin(0, 0)], 2)?;
        let mut r = mkr(1, 2)?;
        ws(&mut r, 0, SlotValue::I64(7))?;
        let mut b = StepBudget::new(10);
        let sig = drive_with_actions(&wf, &mut r, &mut b, &[], RetryPolicy::NEVER)
            .map_err(|e| format!("{e}"))?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(7)),
            other => return Err(format!("expected Finished(I64(7)), got {other:?}")),
        }
        Ok(())
    }

    /// drive_with_actions with a single Nop step and budget=1.
    /// The Nop consumes the one budget unit, then the next loop iteration
    /// fails to take budget and returns StepBudgetExhausted.
    #[test]
    fn dwa_single_nop_budget_one() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 2)?;
        let mut r = mkr(2, 2)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(1);
        let sig = drive_with_actions(&wf, &mut r, &mut b, &[], RetryPolicy::NEVER)
            .map_err(|e| format!("{e}"))?;
        match sig {
            RuntimeSignal::StepBudgetExhausted => Ok(()),
            other => Err(format!("expected StepBudgetExhausted, got {other:?}")),
        }
    }

    /// drive_with_actions budget exhaustion: 3-step workflow with budget=1
    /// exhausts after the first step, never reaching the remaining steps.
    #[test]
    fn dwa_budget_exhaustion() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), nop(1, 2), fin(2, 0)], 2)?;
        let mut r = mkr(3, 2)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(1);
        let sig = drive_with_actions(&wf, &mut r, &mut b, &[], RetryPolicy::NEVER)
            .map_err(|e| format!("{e}"))?;
        assert_eq!(sig, RuntimeSignal::StepBudgetExhausted);
        Ok(())
    }

    /// drive_with_actions with SetConst step writes a constant value to a
    /// slot and the Finish node reads that same slot.
    #[test]
    fn dwa_set_const_step() -> Result<(), String> {
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            2,
            vec![ConstValue::I64(42)],
        )?;
        let mut r = mkr(2, 2)?;
        let mut b = StepBudget::new(10);
        let sig = drive_with_actions(&wf, &mut r, &mut b, &[], RetryPolicy::NEVER)
            .map_err(|e| format!("{e}"))?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(42)),
            other => return Err(format!("expected Finished(I64(42)), got {other:?}")),
        }
        Ok(())
    }
}
