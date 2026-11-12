#![forbid(unsafe_code)]

//! Additional cat1–cat9 variant tests, evidence ordering, and
//! drive_with_actions multi-nop completion.

use super::common::{askn, cpy, cslot, dd, dde, fin, mkr, mkwf, mkwfc, nop, setc, ws, wuntil};
use crate::engine::drive::{compute_max_parallel_in_flight, drive_with_actions};
use crate::engine::types::{EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal};
use vb_core::capability::CapabilitySet;
use vb_core::engine::StepBudget;
use vb_core::ids::{SlotIdx, StepIdx, SymbolId};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::workflow::SlotBranch;

#[cfg(test)]
mod tests {
    use super::*;

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
