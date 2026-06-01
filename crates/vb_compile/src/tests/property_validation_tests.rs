//! Property validation tests for Together and Reduce structural constraints (PO-003, PO-004, PO-005).

#![forbid(unsafe_code)]

use proptest::prelude::*;
use crate::compile_workflow;

#[test]
fn together_empty_branches() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: fanout\n    together:\n      branches: []\n  - id: finish\n    finish:\n      result: done\n";
    let result = compile_workflow(yaml.as_bytes());
    match result {
        Ok(_) => panic!("GAP EXPOSED: Together with 0 branches compiled successfully. Validation missing."),
        Err(e) => println!("PASS (validation exists): Empty Together rejected: {:?}", e),
    }
}

#[test]
fn reduce_empty_body() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: reduce\n    reduce:\n      variable: acc\n      input: items\n      initial: \"0\"\n      steps: []\n  - id: finish\n    finish:\n      result: done\n";
    let result = compile_workflow(yaml.as_bytes());
    match result {
        Ok(_) => panic!("GAP EXPOSED: Reduce with 0 body steps compiled successfully. Validation missing."),
        Err(e) => println!("PASS (validation exists): Empty Reduce body rejected: {:?}", e),
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    #[test]
    fn together_duplicate_labels(branch_count in 2..=4usize) {
        let mut yaml = String::from(
            "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: fanout\n    together:\n      branches:\n",
        );

        for i in 0..branch_count {
            yaml.push_str(&format!(
                "        - label: \"a\"\n          steps:\n            - id: set_{}\n              set:\n                output: x\n                value: \"{}\"\n",
                i, i
            ));
        }
        yaml.push_str("  - id: finish\n    finish:\n      result: done\n");

        let result = compile_workflow(yaml.as_bytes());
        match result {
            Ok(_) => panic!("GAP EXPOSED: Duplicate branch labels compiled successfully. Validation missing."),
            Err(_) => {}
        }
    }
}
