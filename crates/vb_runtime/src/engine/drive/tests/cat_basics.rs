#![forbid(unsafe_code)]

//! cat1–cat5 drive-loop workflow tests: nop, set-const, copy, finish,
//! choose-slot, wait/ask, error-handler, budget.

use super::common::{
    askn, cpy, cslot, dd, errh, fin, mkwf, mkwfc, mkr, nop, setc, ws, wuntil,
};
use vb_core::engine::StepBudget;
use vb_core::value::SlotValue;
use crate::engine::types::RuntimeSignal;

#[cfg(test)]
mod tests {
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
    }
