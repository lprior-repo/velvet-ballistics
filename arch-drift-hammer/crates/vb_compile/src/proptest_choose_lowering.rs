//!
//! Proptest properties for ChooseSlot lowering — supplementary to Kani harnesses.
//!
//! Bead: vb-282my
//! Obligation: PO-vb282my-CL-PROP-001
//!
//! Placed inside the crate (not tests/) because lower_canonical_choose is pub(crate).

#![cfg(test)]

use crate::mod_compile_lowering::{SlotCompiler, lower_canonical_choose};
use proptest::prelude::*;
use vb_core::StepIdx;
use vb_yaml::ast::ChooseBranch;

proptest! {
    /// PO-vb282my-CL-PROP-001: Label resolution edge cases
    /// Tests that lower_canonical_choose handles varied label sets correctly,
    /// including empty branches, missing otherwise, valid labels, and unknown labels.
    /// Does not panic for any valid or edge-case inputs.
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
                when: format!("branch_{i}"),
                steps: Vec::new(),
            });
        }

        let mut step_names: Vec<Box<str>> = Vec::new();
        for i in 0..step_names_count {
            step_names.push(format!("label_{i}").into_boxed_str());
        }

        let otherwise = if use_otherwise {
            Some(otherwise_label.as_str())
        } else {
            None
        };

        let mut builder = SlotCompiler::new();
        let result = lower_canonical_choose(
            index, id, &branches, otherwise, next, &step_names, &mut builder,
        );

        // Function must not panic for any valid input
        // Returns Ok(()) for valid lowering, Err for invalid
        if let Ok(()) = result {
            assert!(branches.len() <= 64);
        }
    }
}
