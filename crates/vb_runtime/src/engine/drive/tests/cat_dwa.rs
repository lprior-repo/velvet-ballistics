#![forbid(unsafe_code)]

//! drive_with_actions compatibility tests (dwa_*).

use super::common::{fin, mkwf, mkwfc, mkr, nop, ws};
use crate::engine::drive::drive_with_actions;
use crate::engine::types::{RetryPolicy, RuntimeSignal};
use vb_core::engine::StepBudget;
use vb_core::value::SlotValue;

#[cfg(test)]
mod tests {
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
    }
