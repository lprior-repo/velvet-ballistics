#![forbid(unsafe_code)]

//! cat7–cat10 + bonus drive-loop workflow tests: multi-step chain,
//! set-const/nop advance, evidence values, do-awaiting-action, compat,
//! together, zero-budget, evidence ordering.

use super::common::{cpy, dd, dde, don, fin, gr, mkr, mkwf, mkwfc, nop, setc, tog, ws};
use crate::engine::drive::{drive_deterministic_full, drive_with_actions};
use crate::engine::types::{EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal};
use crate::primitives::collect::CollectStates;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::engine::StepBudget;
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::value_store::ValueStore;

#[cfg(test)]
mod tests {
    use super::*;

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
                name: ActionName::new("test-action").map_err(|e| format!("{e}"))?,
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
                name: ActionName::new("test-action").map_err(|e| format!("{e}"))?,
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
        let _ = dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
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
    fn drive_with_actions_empty_contracts_finish_only_returns_finished() -> Result<(), String> {
        let wf = mkwf(vec![fin(0, 0)], 1)?;
        let mut r = mkr(1, 1)?;
        ws(&mut r, 0, SlotValue::I64(42))?;
        let mut b = StepBudget::new(10);
        let sig = drive_with_actions(&wf, &mut r, &mut b, &[], RetryPolicy::NEVER)
            .map_err(|e| format!("{e}"))?;
        match sig {
            RuntimeSignal::Finished(SlotValue::I64(42)) => Ok(()),
            other => Err(format!("expected Finished(I64(42)), got {other:?}")),
        }
    }
}
