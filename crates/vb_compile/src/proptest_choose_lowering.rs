//!
//! Proptest properties for ChooseSlot lowering — supplementary to Kani harnesses.
//!
//! Bead: vb-282my
//! Obligation: PO-vb282my-CL-PROP-001
//!
//! Placed inside the crate (not tests/) because lower_canonical_choose is pub(crate).

#![cfg(test)]

use crate::mod_compile_lowering::{SlotCompiler, lower_canonical_choose};
use crate::CompileError;
use proptest::prelude::*;
use vb_core::StepIdx;
use vb_yaml::ast::ChooseBranch;

proptest! {
    /// PO-vb282my-CL-PROP-001: Label resolution edge cases
    /// Tests that lower_canonical_choose handles varied label sets correctly,
    /// including empty branches, missing otherwise, valid labels, and unknown labels.
    /// Does not panic for any valid or edge-case inputs.
    ///
    /// Deterministic outcomes:
    /// - branch_count == 0 AND !use_otherwise → Err(EmptyBranchTable)
    /// - branch_count > 0 OR use_otherwise → Ok(())
    #[test]
    fn proptest_choose_lowering_label_resolution(
        branch_count in 0usize..=64usize,
        step_names_count in 1usize..=16usize,
        otherwise_label in "[a-zA-Z0-9_]{1,16}",
        use_otherwise in proptest::bool::ANY,
    ) {
        let index = 0usize;
        let id = StepIdx::new(0);
        let next = Some(StepIdx::new(100));

        let mut branches: Vec<ChooseBranch> = Vec::new();
        for i in 0..branch_count {
            branches.push(ChooseBranch {
                when: format!("{}", i),
                steps: Vec::new(),
            });
        }

        let step_names: Vec<Box<str>> = (0..step_names_count)
            .map(|i| format!("label_{i}").into_boxed_str())
            .collect();

        let otherwise = if use_otherwise {
            Some(otherwise_label.as_str())
        } else {
            None
        };

        let mut builder = SlotCompiler::new();
        let result = lower_canonical_choose(
            index, id, &branches, otherwise, next, &step_names, &mut builder,
        );

        // Determine expected outcome:
        // - otherwise.is_some() AND label unknown → Err(UnknownStepLabel)
        // - branch_count == 0 AND no otherwise → Err(EmptyBranchTable)
        // - branch_count > 0 AND (no otherwise OR known otherwise) → Ok(())
        let otherwise_label_known = otherwise
            .map(|lbl| step_names.iter().any(|n| n.as_ref() == lbl))
            .unwrap_or(true); // no otherwise → known (trivially)

        let expect_ok = branch_count > 0 && otherwise_label_known;

        if expect_ok {
            prop_assert!(
                matches!(result, Ok(())),
                "non-empty choose or choose with known otherwise must succeed, got {:?}",
                result
            );
            // Verify at least one node was produced (the choose node itself).
            prop_assert!(builder.nodes.len() >= 1,
                "lower_canonical_choose must produce at least one node, got {}", builder.nodes.len());
        } else if otherwise.is_some() && !otherwise_label_known {
            prop_assert!(
                matches!(result, Err(ref errs) if errs.0.len() == 1
                    && matches!(&errs.0[0],
                        CompileError::UnknownStepLabel { step: 0, .. })),
                "choose with unknown otherwise must return UnknownStepLabel, got {:?}",
                result
            );
        } else {
            prop_assert!(
                matches!(result, Err(ref errs) if errs.0.len() == 1
                    && matches!(&errs.0[0],
                        CompileError::Workflow(we) if matches!(we,
                            vb_core::WorkflowError::EmptyBranchTable))),
                "empty choose without otherwise must return EmptyBranchTable, got {:?}",
                result
            );
        }
    }
}
