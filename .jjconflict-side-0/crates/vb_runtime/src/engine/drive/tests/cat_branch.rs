#![forbid(unsafe_code)]

//! BranchLimitExceeded + compute_max_parallel_in_flight tests.

use super::common::{dd, fin, mkr, mkwf, mkwfc, setc, tog};
use crate::engine::drive::compute_max_parallel_in_flight;
use crate::engine::types::{RuntimeEngineError, RuntimeSignal};
use vb_core::engine::StepBudget;
use vb_core::ids::{StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

#[cfg(test)]
mod tests {
    use super::*;

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
        let wf = mkwf(
            vec![
                tog(0, 0, Box::from([1u16, 2]), 3),
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
}
