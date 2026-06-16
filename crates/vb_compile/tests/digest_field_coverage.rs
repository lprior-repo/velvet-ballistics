//! Digest field coverage proptest for Together and Reduce (PO-009).

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::WorkflowDigest;

fn together_workflow_yaml(branches_yaml: &str) -> String {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: together-digest-test\nwhen:\n  manual: {}\nsteps:\n",
    );
    yaml.push_str(branches_yaml);
    yaml.push_str("  - id: done\n    finish:\n      result: 0\n");
    yaml
}

fn reduce_workflow_yaml(variable: &str, input: &str, initial: &str, body_yaml: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: reduce-digest-test\nwhen:\n  manual: {{}}\nsteps:\n  - id: reduce\n    reduce:\n      variable: \"{}\"\n      input: \"{}\"\n      initial: \"{}\"\n      steps:\n{}\n  - id: done\n    finish:\n      result: 0
",
        variable, input, initial, body_yaml
    )
}

fn compile_and_digest(yaml: &str) -> Result<WorkflowDigest, String> {
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())
        .map_err(|e| format!("compile error: {e:?}"))?;
    Ok(workflow.digest())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    #[test]
    fn together_different_branch_counts_different_digests(count_a in 1..=4usize, count_b in 1..=4usize) {
        prop_assume!(count_a != count_b);

        let mut yaml_a = String::from("  - id: fanout\n    together:\n      branches:\n");
        let mut yaml_b = String::from("  - id: fanout\n    together:\n      branches:\n");

        for i in 0..count_a {
            yaml_a.push_str(&format!(
                "        - label: \"a\"\n          steps:\n            - id: set_{}\n              set:\n                output: x\n                value: \"1\"\n",
                i
            ));
        }

        for i in 0..count_b {
            yaml_b.push_str(&format!(
                "        - label: \"b\"\n          steps:\n            - id: set_{}\n              set:\n                output: x\n                value: \"1\"\n",
                i
            ));
        }

        let digest_a = compile_and_digest(&together_workflow_yaml(&yaml_a))
            .map_err(TestCaseError::fail)?;
        let digest_b = compile_and_digest(&together_workflow_yaml(&yaml_b))
            .map_err(TestCaseError::fail)?;

        prop_assert_ne!(digest_a, digest_b,
            "Different branch counts ({}, {}) must produce different digests", count_a, count_b);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    #[test]
    fn together_different_labels_different_digests(label_a in "[a-z]", label_b in "[a-z]") {
        prop_assume!(label_a != label_b);

        let yaml_a = format!(
            "  - id: fanout\n    together:\n      branches:\n        - label: \"{}\"\n          steps:\n            - id: set_1\n              set:\n                output: x\n                value: \"1\"\n",
            label_a
        );

        let yaml_b = format!(
            "  - id: fanout\n    together:\n      branches:\n        - label: \"{}\"\n          steps:\n            - id: set_1\n              set:\n                output: x\n                value: \"1\"\n",
            label_b
        );

        let digest_a = compile_and_digest(&together_workflow_yaml(&yaml_a))
            .map_err(TestCaseError::fail)?;
        let digest_b = compile_and_digest(&together_workflow_yaml(&yaml_b))
            .map_err(TestCaseError::fail)?;

        prop_assert_ne!(digest_a, digest_b,
            "Different labels ('{}', '{}') must produce different digests", label_a, label_b);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    // Re-enabled by vb-em8xu (vb-budget-reduce): the budget traversal now
    // uses the cold-AST-conservative iter count of 1 for ReduceStart instead
    // of MAX_LIST_ITEMS_PER_VALUE, so reduce workflows compile within the
    // 1_000-step policy limit.
    #[test]
    fn reduce_different_variables_different_digests(var_a in "[a-z]", var_b in "[a-z]") {
        prop_assume!(var_a != var_b);

        let yaml_a = reduce_workflow_yaml(
            &var_a, "0", "0",
            "            - id: set_acc\n              set:\n                output: acc\n                value: \"1\"\n"
        );

        let yaml_b = reduce_workflow_yaml(
            &var_b, "0", "0",
            "            - id: set_acc\n              set:\n                output: acc\n                value: \"1\"\n"
        );

        let digest_a = compile_and_digest(&yaml_a)
            .map_err(TestCaseError::fail)?;
        let digest_b = compile_and_digest(&yaml_b)
            .map_err(TestCaseError::fail)?;

        prop_assert_ne!(digest_a, digest_b,
            "Different reduce variables ('{}', '{}') must produce different digests", var_a, var_b);
    }
}

#[test]
fn together_same_structure_same_digest() {
    let yaml = r#"
version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: fanout
    together:
      branches:
        - label: "a"
          steps:
            - id: set_1
              set:
                output: "x"
                value: "1"
        - label: "b"
          steps:
            - id: set_2
              set:
                output: "y"
                value: "2"
  - id: done
    finish:
      result: 0
"#;

    let digest_a = compile_and_digest(yaml)
        .expect("together workflow must compile and digest");
    let digest_b = compile_and_digest(yaml)
        .expect("together workflow must compile and digest (second call)");

    assert_eq!(
        digest_a, digest_b,
        "Identical together structures must produce identical digests (idempotence)"
    );
}
