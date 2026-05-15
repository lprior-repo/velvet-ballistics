#![forbid(unsafe_code)]

//! Deterministic drive loop for runtime engine.

use vb_core::action::ActionContract;
use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::execute_node_full;
use crate::engine::helpers::mark_step_after_signal;
use crate::engine::types::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal,
};
use crate::primitives::collect::CollectStates;

pub(crate) fn compute_max_parallel_in_flight(plan: &CompiledWorkflow) -> RuntimeEngineResult<u16> {
    let mut max_branches: u16 = 0;
    for i in 0..plan.node_count() {
        let step = StepIdx::new(i);
        if let Some(node) = plan.node(step)
            && let CompiledNodeKind::TogetherStart { branches, .. } = &node.kind
        {
            let branch_count = u16::try_from(branches.len()).map_err(|_| {
                RuntimeEngineError::BranchLimitExceeded {
                    max: u16::MAX.into(),
                    requested: branches.len(),
                }
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
    initialize_drive(run, plan)?;

    loop {
        let Some(step) = begin_drive_step(plan, run, budget, evidence)? else {
            return Ok(RuntimeSignal::StepBudgetExhausted);
        };
        let signal = execute_node_full(
            plan,
            run,
            store,
            step.node,
            contracts,
            retry_policy,
            collect_states,
            granted,
        )?;
        finish_drive_step(run, evidence, collect_states, step, &signal)?;
        match signal {
            RuntimeSignal::Continue => {}
            other => return Ok(other),
        }
    }
}

struct DriveStep<'a> {
    pc: StepIdx,
    node: &'a CompiledNode,
}

fn initialize_drive(run: &mut RunFrame, plan: &CompiledWorkflow) -> RuntimeEngineResult<()> {
    let max_parallel = compute_max_parallel_in_flight(plan)?;
    run.set_max_parallel_in_flight(max_parallel);
    Ok(())
}

fn begin_drive_step<'a>(
    plan: &'a CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    evidence: &mut EvidenceCollector,
) -> RuntimeEngineResult<Option<DriveStep<'a>>> {
    if !budget.try_take().map_err(RuntimeEngineError::Core)? {
        return Ok(None);
    }
    let pc = run.pc();
    let node = plan
        .node(pc)
        .ok_or(EngineError::InvalidProgramCounter { step: pc })?;
    evidence.push_step_started(pc);
    run.mark_running(pc).map_err(RuntimeEngineError::Core)?;
    Ok(Some(DriveStep { pc, node }))
}

fn finish_drive_step(
    run: &mut RunFrame,
    evidence: &mut EvidenceCollector,
    collect_states: &CollectStates,
    step: DriveStep<'_>,
    signal: &RuntimeSignal,
) -> RuntimeEngineResult<()> {
    mark_step_after_signal(run, step.pc, signal).map_err(RuntimeEngineError::Core)?;
    emit_slot_evidence(run, evidence, collect_states, step.node)?;
    if signal_is_success(signal) {
        evidence.push_step_succeeded(step.pc, step.node.output);
    }
    Ok(())
}

fn signal_is_success(signal: &RuntimeSignal) -> bool {
    matches!(signal, RuntimeSignal::Continue | RuntimeSignal::Finished(_))
}

fn emit_slot_evidence(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    collect_states: &CollectStates,
    node: &CompiledNode,
) -> RuntimeEngineResult<()> {
    if let Some(slot) = collect_written_slot(node)
        && let Ok(value) = run.read_slot(slot)
    {
        let extra = collect_states.capture_state(run.run_id(), slot);
        let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
        evidence
            .push_slot_written_with_extra(slot, *value, taint, extra)
            .map_err(RuntimeEngineError::Core)?;
    } else if let Some(slot) = node.output
        && let Ok(value) = run.read_slot(slot)
    {
        let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
        evidence.push_slot_written_with_taint(slot, *value, taint);
    }
    Ok(())
}

fn collect_written_slot(node: &CompiledNode) -> Option<SlotIdx> {
    match &node.kind {
        CompiledNodeKind::CollectStart { source, .. } => match node.output {
            Some(output) => Some(output),
            None => Some(*source),
        },
        CompiledNodeKind::CollectNext { collector_slot, .. }
        | CompiledNodeKind::CollectFinish { collector_slot } => Some(*collector_slot),
        _ => None,
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
    use crate::engine::drive::{drive_deterministic_full, drive_with_actions};
    use crate::engine::types::{
        EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
    };
    use crate::primitives::collect::CollectStates;
    use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
    use vb_core::capability::{Capability, CapabilitySet};
    use vb_core::engine::StepBudget;
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
    use vb_core::value::{ConstValue, SlotValue};
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, SlotBranch,
        WorkflowParts,
    };

    fn cn(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: output.map(SlotIdx::new),
            next: next.map(StepIdx::new),
            on_error: None,
            error_slot: None,
            kind,
        }
    }
    fn fin(id: u16, result: u16) -> CompiledNode {
        cn(
            id,
            None,
            None,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(result),
            },
        )
    }
    fn nop(id: u16, nx: u16) -> CompiledNode {
        cn(id, None, Some(nx), CompiledNodeKind::Nop)
    }
    fn setc(id: u16, cid: u16, out: u16, nx: u16) -> CompiledNode {
        cn(
            id,
            Some(out),
            Some(nx),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(cid),
            },
        )
    }
    fn cpy(id: u16, src: u16, out: u16, nx: u16) -> CompiledNode {
        cn(
            id,
            Some(out),
            Some(nx),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(src),
            },
        )
    }
    fn don(id: u16, action: u16, inp: u16) -> CompiledNode {
        cn(
            id,
            None,
            None,
            CompiledNodeKind::Do {
                action: ActionId::new(action),
                input: SlotIdx::new(inp),
            },
        )
    }
    fn collect_start(id: u16, source: u16, out: u16, body: u16, done: u16) -> CompiledNode {
        cn(
            id,
            Some(out),
            None,
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(source),
                limit: 100,
                page_size: 1,
                body: StepIdx::new(body),
                done: StepIdx::new(done),
            },
        )
    }
    fn cslot(id: u16, branches: Box<[SlotBranch]>, otw: Option<u16>) -> CompiledNode {
        cn(
            id,
            None,
            otw,
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise: otw.map(StepIdx::new),
            },
        )
    }
    fn askn(id: u16, prompt: u16) -> CompiledNode {
        cn(
            id,
            None,
            None,
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(prompt),
                timeout_slot: None,
            },
        )
    }
    fn wuntil(id: u16, dl: u16) -> CompiledNode {
        cn(
            id,
            None,
            None,
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(dl),
            },
        )
    }
    fn errh(id: u16, body: u16, handler: u16, eslot: Option<u16>) -> CompiledNode {
        cn(
            id,
            None,
            None,
            CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(body),
                handler: StepIdx::new(handler),
                error_slot: eslot.map(SlotIdx::new),
            },
        )
    }
    fn tog(id: u16, branches: Box<[u16]>, join: u16) -> CompiledNode {
        let br: Box<[StepIdx]> = branches.iter().map(|b| StepIdx::new(*b)).collect();
        cn(
            id,
            None,
            None,
            CompiledNodeKind::TogetherStart {
                branches: br,
                join: StepIdx::new(join),
            },
        )
    }

    fn mkwf(nodes: Vec<CompiledNode>, sc: u16) -> Result<CompiledWorkflow, String> {
        mkwfc(nodes, sc, vec![])
    }
    fn mkwfc(
        nodes: Vec<CompiledNode>,
        sc: u16,
        consts: Vec<ConstValue>,
    ) -> Result<CompiledWorkflow, String> {
        let names: Box<[Box<str>]> = (0..nodes.len())
            .map(|i| format!("s{i}").into_boxed_str())
            .collect();
        let parts = WorkflowParts {
            name: "test".into(),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: consts.into_boxed_slice(),
            slot_count: sc,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: names,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| format!("{e}"))
    }
    fn mkr(steps: u16, slots: u16) -> Result<RunFrame, String> {
        RunFrame::new(RunId::new(1), StepIdx::new(0), steps, slots).map_err(|e| format!("{e}"))
    }
    fn dd(
        wf: &CompiledWorkflow,
        r: &mut RunFrame,
        b: &mut StepBudget,
    ) -> Result<RuntimeSignal, String> {
        let mut store = ValueStore::new();
        let mut ev = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        drive_deterministic_full(
            wf,
            r,
            b,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut ev,
            &mut cs,
            &CapabilitySet::empty(),
        )
        .map_err(|e| format!("{e}"))
    }
    fn dde(
        wf: &CompiledWorkflow,
        r: &mut RunFrame,
        b: &mut StepBudget,
        ev: &mut EvidenceCollector,
        g: &CapabilitySet,
    ) -> Result<RuntimeSignal, String> {
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        drive_deterministic_full(
            wf,
            r,
            b,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            ev,
            &mut cs,
            g,
        )
        .map_err(|e| format!("{e}"))
    }
    fn ws(r: &mut RunFrame, s: u16, v: SlotValue) -> Result<(), String> {
        r.write_slot(SlotIdx::new(s), v).map_err(|e| format!("{e}"))
    }
    fn gr(name: &str, action: u16) -> CapabilitySet {
        CapabilitySet::from_grants(Box::from([Capability::new(
            name.into(),
            ActionId::new(action),
        )]))
    }

    #[test]
    fn cat1_nop_continues() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(0)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    fn cat1_set_const_writes() -> Result<(), String> {
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            1,
            vec![ConstValue::I64(42)],
        )?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(42)),
            _ => return Err("expected Finished".into()),
        }
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
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(99)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    fn cat1_finish_immediate() -> Result<(), String> {
        let wf = mkwf(vec![fin(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::Bool(true))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::Bool(true)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    fn cat2_choose_slot_matching() -> Result<(), String> {
        let branches = Box::from([
            SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(1),
            },
            SlotBranch {
                condition: SlotIdx::new(1),
                target: StepIdx::new(2),
            },
        ]);
        let wf = mkwf(vec![cslot(0, branches, None), fin(1, 2), fin(2, 2)], 3)?;
        let mut r = mkr(3, 3)?;
        ws(&mut r, 0, SlotValue::Bool(true))?;
        ws(&mut r, 1, SlotValue::Bool(false))?;
        ws(&mut r, 2, SlotValue::I64(10))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(10)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    fn cat2_choose_slot_no_match_errors() -> Result<(), String> {
        let branches = Box::from([SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(1),
        }]);
        let wf = mkwf(vec![cslot(0, branches, None), fin(1, 1)], 2)?;
        let mut r = mkr(2, 2)?;
        ws(&mut r, 0, SlotValue::Bool(false))?;
        ws(&mut r, 1, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let result = dd(&wf, &mut r, &mut b);
        if result.is_ok() {
            return Err("expected error for no matching branch".into());
        }
        Ok(())
    }

    #[test]
    fn cat2_choose_otherwise() -> Result<(), String> {
        let branches = Box::from([SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(1),
        }]);
        let wf = mkwf(vec![cslot(0, branches, Some(2)), fin(1, 1), fin(2, 1)], 2)?;
        let mut r = mkr(3, 2)?;
        ws(&mut r, 0, SlotValue::Bool(false))?;
        ws(&mut r, 1, SlotValue::I64(77))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(77)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    fn cat3_wait_until_awaiting() -> Result<(), String> {
        let wf = mkwf(vec![wuntil(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(1000))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::AwaitingWait => Ok(()),
            _ => Err("expected AwaitingWait".into()),
        }
    }

    #[test]
    fn cat3_ask_awaiting() -> Result<(), String> {
        let wf = mkwf(vec![askn(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::Symbol(SymbolId::new(1)))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::AwaitingAsk => Ok(()),
            _ => Err("expected AwaitingAsk".into()),
        }
    }

    #[test]
    fn cat4_error_handler_body_succeeds() -> Result<(), String> {
        let wf = mkwf(
            vec![errh(0, 1, 2, None), nop(1, 3), fin(2, 0), fin(3, 0)],
            1,
        )?;
        let mut r = mkr(4, 1)?;
        ws(&mut r, 0, SlotValue::I64(55))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(55)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    fn cat5_budget_exhausted() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), nop(1, 2), fin(2, 0)], 1)?;
        let mut r = mkr(3, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(2);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::StepBudgetExhausted => Ok(()),
            other => Err(format!("expected StepBudgetExhausted, got {other:?}")),
        }
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
        let started = events
            .iter()
            .filter(|e| matches!(e, EvidenceEvent::StepStarted { .. }))
            .count();
        if started < 2 {
            return Err(format!("expected >= 2 StepStarted, got {started}"));
        }
        Ok(())
    }

    #[test]
    fn cat6_evidence_slot_written() -> Result<(), String> {
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            1,
            vec![ConstValue::I64(7)],
        )?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let writes: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, EvidenceEvent::SlotWritten { .. }))
            .collect();
        if writes.is_empty() {
            return Err("expected at least one SlotWritten".into());
        }
        Ok(())
    }

    #[test]
    fn collect_pagination_extra_single_authoritative_evidence_write() -> Result<(), String> {
        let wf = mkwf(vec![collect_start(0, 0, 1, 1, 2), fin(1, 1), fin(2, 1)], 2)?;
        let mut run = mkr(3, 2)?;
        let mut store = ValueStore::new();
        let page = Box::from([SlotValue::I64(10), SlotValue::I64(20)]);
        let list_id = store.insert_list(page).map_err(|e| format!("{e}"))?;
        run.write_slot(SlotIdx::new(0), SlotValue::List(list_id))
            .map_err(|e| format!("{e}"))?;
        let mut budget = StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();

        drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        )
        .map_err(|e| format!("{e}"))?;

        let events = evidence.drain();
        let matching_writes = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    EvidenceEvent::SlotWritten {
                        slot,
                        ..
                    } if *slot == SlotIdx::new(1)
                )
            })
            .count();
        let extra_bearing_writes = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    EvidenceEvent::SlotWritten {
                        slot,
                        extra: Some(_),
                        ..
                    } if *slot == SlotIdx::new(1)
                )
            })
            .count();
        assert_eq!(matching_writes, 1);
        assert_eq!(extra_bearing_writes, 1);
        Ok(())
    }

    #[test]
    fn cat7_multi_step_chain() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), nop(1, 2), nop(2, 3), fin(3, 0)], 1)?;
        let mut r = mkr(4, 1)?;
        ws(&mut r, 0, SlotValue::Bool(false))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::Bool(false)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    fn cat8_set_const_advances_pc() -> Result<(), String> {
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            1,
            vec![ConstValue::I64(5)],
        )?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(_) => Ok(()),
            other => Err(format!("expected Finished, got {other:?}")),
        }
    }

    #[test]
    fn cat8_nop_advances_pc() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::Null)?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(_) => Ok(()),
            other => Err(format!("expected Finished, got {other:?}")),
        }
    }

    #[test]
    fn cat9_set_const_evidence_value() -> Result<(), String> {
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            1,
            vec![ConstValue::I64(33)],
        )?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let found = events.iter().any(|e| match e {
            EvidenceEvent::SlotWritten { slot, value, .. } => {
                *slot == SlotIdx::new(0) && *value == SlotValue::I64(33)
            }
            _ => false,
        });
        if !found {
            return Err("expected SlotWritten(0, I64(33))".into());
        }
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
            EvidenceEvent::SlotWritten { slot, value, .. } => {
                *slot == SlotIdx::new(0) && *value == SlotValue::I64(88)
            }
            _ => false,
        });
        if !found {
            return Err("expected SlotWritten(0, I64(88))".into());
        }
        Ok(())
    }

    #[test]
    fn cat10_do_awaiting_action() -> Result<(), String> {
        let wf = mkwf(vec![don(0, 1, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let contracts = [
            ActionContract {
                id: ActionId::new(0),
                input_slot_count: 0,
                output_slot_count: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                timeout_ms: 0,
                idempotency: Idempotency::DeterministicPure,
                side_effect: SideEffect::None,
                retry_safety: RetrySafety::Safe,
                required_capabilities: Box::from([]),
            },
            ActionContract {
                id: ActionId::new(1),
                input_slot_count: 1,
                output_slot_count: 0,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 5000,
                idempotency: Idempotency::DeterministicPure,
                side_effect: SideEffect::None,
                retry_safety: RetrySafety::Safe,
                required_capabilities: Box::from([Capability::new(
                    "__contract_required__".into(),
                    ActionId::new(1),
                )]),
            },
        ];
        let g = gr("__contract_required__", 1);
        let mut store = ValueStore::new();
        let mut ev = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let sig = drive_deterministic_full(
            &wf,
            &mut r,
            &mut b,
            &mut store,
            &contracts,
            RetryPolicy::NEVER,
            &mut ev,
            &mut cs,
            &g,
        )
        .map_err(|e| format!("{e}"))?;
        match sig {
            RuntimeSignal::AwaitingAction(_) => Ok(()),
            other => Err(format!("expected AwaitingAction, got {other:?}")),
        }
    }

    // Do node without contract fails closed.
    #[test]
    fn cat10_do_without_contract_rejects() -> Result<(), String> {
        let wf = mkwf(vec![don(0, 1, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        match dd(&wf, &mut r, &mut b) {
            Err(error) if error.contains("capability denied for action ActionId(1)") => Ok(()),
            other => Err(format!("expected CapabilityDenied, got {other:?}")),
        }
    }

    #[test]
    fn bonus_compat() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let sig = drive_with_actions(&wf, &mut r, &mut b, &[], RetryPolicy::NEVER)
            .map_err(|e| format!("{e}"))?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(0)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    fn bonus_together() -> Result<(), String> {
        let wf = mkwf(
            vec![
                tog(0, Box::from([1u16, 2]), 3),
                fin(1, 1),
                fin(2, 1),
                fin(3, 1),
            ],
            2,
        )?;
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
        match sig {
            RuntimeSignal::StepBudgetExhausted => Ok(()),
            other => Err(format!("expected StepBudgetExhausted, got {other:?}")),
        }
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

    // =====================================================================
    // BranchLimitExceeded error variant tests
    // =====================================================================

    /// Verify that BranchLimitExceeded error carries the correct max and
    /// requested values. The error is a defense-in-depth guard in
    /// compute_max_parallel_in_flight: workflow validation rejects fanout > 64
    /// at construction time, so the u16::MAX branch limit cannot be reached
    /// through the public API. We test the error variant directly.
    #[test]
    fn branch_limit_exceeded_fields_are_correct() {
        let max_val = usize::from(u16::MAX);
        let requested = usize::from(u16::MAX).saturating_add(1);
        let error = RuntimeEngineError::BranchLimitExceeded {
            max: max_val,
            requested,
        };
        match error {
            RuntimeEngineError::BranchLimitExceeded {
                max: got_max,
                requested: got_req,
            } => {
                assert_eq!(got_max, max_val, "max should be u16::MAX as usize");
                assert_eq!(got_req, requested, "requested should be u16::MAX + 1");
            }
            other => {
                let msg = format!("expected BranchLimitExceeded, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    /// Verify that BranchLimitExceeded error display message contains both the
    /// max and requested count.
    #[test]
    fn branch_limit_exceeded_display_message() {
        let max_val = usize::from(u16::MAX);
        let requested = usize::from(u16::MAX).saturating_add(1);
        let error = RuntimeEngineError::BranchLimitExceeded {
            max: max_val,
            requested,
        };
        let msg = format!("{error}");
        assert!(
            msg.contains(&max_val.to_string()),
            "display should contain max value: '{msg}'"
        );
        assert!(
            msg.contains(&requested.to_string()),
            "display should contain requested value: '{msg}'"
        );
    }

    /// Verify that BranchLimitExceeded returns the correct runtime code
    /// from runtime_code().
    #[test]
    fn branch_limit_exceeded_runtime_code_is_set() {
        let error = RuntimeEngineError::BranchLimitExceeded {
            max: usize::from(u16::MAX),
            requested: usize::from(u16::MAX).saturating_add(1),
        };
        match error.runtime_code() {
            Some(code) => assert_eq!(
                code,
                RuntimeEngineError::BRANCH_LIMIT_EXCEEDED_RUNTIME_CODE,
                "BranchLimitExceeded should return its dedicated runtime code"
            ),
            None => {
                let msg = "BranchLimitExceeded should have a runtime code";
                panic!("{msg}");
            }
        }
    }

    /// Verify that compute_max_parallel_in_flight returns the correct u16
    /// branch count for a valid TogetherStart workflow with 2 branches.
    /// This confirms the function works correctly for the happy path
    /// (BranchLimitExceeded is the error path for > u16::MAX branches).
    #[test]
    fn compute_max_parallel_returns_branch_count_for_valid_workflow() -> Result<(), String> {
        use crate::engine::drive::compute_max_parallel_in_flight;
        let wf = mkwf(
            vec![
                tog(0, Box::from([1u16, 2]), 3),
                fin(1, 1),
                fin(2, 1),
                fin(3, 1),
            ],
            2,
        )?;
        let result = compute_max_parallel_in_flight(&wf).map_err(|e| format!("{e}"))?;
        assert_eq!(
            result, 2,
            "max parallel should equal the TogetherStart branch count"
        );
        Ok(())
    }

    /// Verify that compute_max_parallel_in_flight returns BranchLimitExceeded
    /// when a TogetherStart node has more than u16::MAX branches.
    ///
    /// Workflow validation limits fanout to 64, so this path cannot be reached
    /// through the public API. We construct the workflow via
    /// `from_parts_unchecked` to bypass validation and exercise the
    /// defense-in-depth guard in compute_max_parallel_in_flight.
    #[test]
    fn compute_max_parallel_rejects_branch_count_exceeding_u16_max() {
        use crate::engine::drive::compute_max_parallel_in_flight;

        let branch_count = usize::from(u16::MAX).saturating_add(1);
        let branches: Box<[StepIdx]> = std::iter::repeat(StepIdx::new(0))
            .take(branch_count)
            .collect();

        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches,
                join: StepIdx::new(0),
            },
        };

        let parts = WorkflowParts {
            name: "bh_branch_limit".into(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::from([Box::from("s0")]),
        };
        let wf = CompiledWorkflow::from_parts_unchecked(parts);

        let result = compute_max_parallel_in_flight(&wf);
        match result {
            Err(RuntimeEngineError::BranchLimitExceeded { max, requested }) => {
                assert_eq!(
                    max,
                    usize::from(u16::MAX),
                    "max should be u16::MAX as usize"
                );
                assert_eq!(
                    requested, branch_count,
                    "requested should match the TogetherStart branch count"
                );
            }
            other => {
                let msg = format!("expected BranchLimitExceeded, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    /// SetConst with negative I64 propagates correctly.
    #[test]
    fn cat8_set_const_negative_propagates() -> Result<(), String> {
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            1,
            vec![ConstValue::I64(-10)],
        )?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(-10)),
            other => return Err(format!("expected Finished(I64(-10)), got {other:?}")),
        }
        Ok(())
    }

    /// Copy propagates Bool values through the drive loop.
    #[test]
    fn cat1_copy_propagates_bool() -> Result<(), String> {
        let wf = mkwf(vec![cpy(0, 1, 0, 1), fin(1, 0)], 2)?;
        let mut r = mkr(2, 2)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        ws(&mut r, 1, SlotValue::Bool(true))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::Bool(true)),
            other => return Err(format!("expected Finished(Bool(true)), got {other:?}")),
        }
        Ok(())
    }

    /// ChooseSlot second branch matches: slot 0=false, slot 1=true -> target 2.
    #[test]
    fn cat2_choose_slot_second_branch_matches() -> Result<(), String> {
        let branches = Box::from([
            SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(1),
            },
            SlotBranch {
                condition: SlotIdx::new(1),
                target: StepIdx::new(2),
            },
        ]);
        let wf = mkwf(
            vec![cslot(0, branches, Some(3)), fin(1, 3), fin(2, 3), fin(3, 3)],
            4,
        )?;
        let mut r = mkr(4, 4)?;
        ws(&mut r, 0, SlotValue::Bool(false))?;
        ws(&mut r, 1, SlotValue::Bool(true))?;
        ws(&mut r, 2, SlotValue::I64(0))?;
        ws(&mut r, 3, SlotValue::I64(55))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(55)),
            other => return Err(format!("expected Finished(I64(55)), got {other:?}")),
        }
        Ok(())
    }

    /// StepStarted appears before StepSucceeded in evidence ordering.
    #[test]
    fn cat6_evidence_started_before_succeeded() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let started_idx = events
            .iter()
            .position(|e| matches!(e, EvidenceEvent::StepStarted { .. }));
        let succeeded_idx = events
            .iter()
            .position(|e| matches!(e, EvidenceEvent::StepSucceeded { .. }));
        match (started_idx, succeeded_idx) {
            (Some(si), Some(su)) => {
                assert!(si < su, "StepStarted should appear before StepSucceeded")
            }
            _ => return Err("expected both StepStarted and StepSucceeded events".into()),
        }
        Ok(())
    }

    /// Ask node emits StepStarted but not StepSucceeded (suspends).
    #[test]
    fn cat3_ask_evidence_has_started_no_succeeded() -> Result<(), String> {
        let wf = mkwf(vec![askn(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::Symbol(SymbolId::new(1)))?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        let sig = dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        match sig {
            RuntimeSignal::AwaitingAsk => {}
            other => return Err(format!("expected AwaitingAsk, got {other:?}")),
        }
        let events = ev.drain();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EvidenceEvent::StepStarted { .. })),
            "Ask should emit StepStarted"
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EvidenceEvent::StepSucceeded { .. })),
            "Ask should not emit StepSucceeded"
        );
        Ok(())
    }

    /// WaitUntil node emits StepStarted but not StepSucceeded (suspends).
    #[test]
    fn cat3_wait_evidence_has_started_no_succeeded() -> Result<(), String> {
        let wf = mkwf(vec![wuntil(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(9999))?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        let sig = dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        match sig {
            RuntimeSignal::AwaitingWait => {}
            other => return Err(format!("expected AwaitingWait, got {other:?}")),
        }
        let events = ev.drain();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EvidenceEvent::StepStarted { .. })),
            "WaitUntil should emit StepStarted"
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EvidenceEvent::StepSucceeded { .. })),
            "WaitUntil should not emit StepSucceeded"
        );
        Ok(())
    }

    /// Single Finish node completes with budget=1.
    #[test]
    fn cat1_single_finish_budget_one() -> Result<(), String> {
        let wf = mkwf(vec![fin(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(77))?;
        let mut b = StepBudget::new(1);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(77)),
            other => return Err(format!("expected Finished(I64(77)), got {other:?}")),
        }
        Ok(())
    }

    /// 3-step chain produces >= 3 StepStarted and >= 3 StepSucceeded.
    #[test]
    fn cat6_evidence_three_step_chain() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), nop(1, 2), fin(2, 0)], 1)?;
        let mut r = mkr(3, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let sc = events
            .iter()
            .filter(|e| matches!(e, EvidenceEvent::StepStarted { .. }))
            .count();
        let suc = events
            .iter()
            .filter(|e| matches!(e, EvidenceEvent::StepSucceeded { .. }))
            .count();
        assert!(sc >= 3, "expected >= 3 StepStarted, got {sc}");
        assert!(suc >= 3, "expected >= 3 StepSucceeded, got {suc}");
        Ok(())
    }

    /// drive_with_actions with 4 Nop steps and adequate budget completes.
    #[test]
    fn dwa_multi_nop_completes() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), nop(1, 2), nop(2, 3), fin(3, 0)], 1)?;
        let mut r = mkr(4, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let sig = drive_with_actions(&wf, &mut r, &mut b, &[], RetryPolicy::NEVER)
            .map_err(|e| format!("{e}"))?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::I64(0)),
            other => return Err(format!("expected Finished, got {other:?}")),
        }
        Ok(())
    }

    /// compute_max_parallel_in_flight returns 0 when no TogetherStart exists.
    #[test]
    fn compute_max_parallel_returns_zero_without_together_start() -> Result<(), String> {
        use crate::engine::drive::compute_max_parallel_in_flight;
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let result = compute_max_parallel_in_flight(&wf).map_err(|e| format!("{e}"))?;
        assert_eq!(result, 0, "should return 0 when no TogetherStart exists");
        Ok(())
    }

    /// SetConst with Bool(true) produces SlotWritten evidence.
    #[test]
    fn cat9_set_const_bool_evidence() -> Result<(), String> {
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            1,
            vec![ConstValue::Bool(true)],
        )?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let found = events.iter().any(|e| match e {
            EvidenceEvent::SlotWritten { slot, value, .. } => {
                *slot == SlotIdx::new(0) && *value == SlotValue::Bool(true)
            }
            _ => false,
        });
        if !found {
            return Err("expected SlotWritten(0, Bool(true))".into());
        }
        Ok(())
    }
}
