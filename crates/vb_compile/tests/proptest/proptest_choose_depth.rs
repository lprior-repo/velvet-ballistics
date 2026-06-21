// Verification artifact: proptest_choose_depth.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-PROPTEST-004 — Deeply nested/varied choose: compiles or errors gracefully (no panic)
// Command: cargo test -p vb_compile --test proptest_choose_depth -- --nocapture
//
// GOD RULE 1: Uses proptest strategies for varied choose configurations.
// GOD RULE 2: Binds to compile_workflow.

#![forbid(unsafe_code)]
use proptest::prelude::*;

/// Build a YAML workflow with a single choose step with varying complexity.
/// Varies: branch count, body step count per branch, with/without otherwise.
fn varied_choose_yaml(branch_count: u8, max_body: u8, has_otherwise: bool) -> String {
    let mut yaml =
        String::from("version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n");
    yaml.push_str("  - id: pick\n    choose:\n      branches:\n");

    let actual_count = branch_count.min(64);
    for i in 0..actual_count {
        yaml.push_str(&format!("        - when: \"{}\"\n", i));
        if max_body == 0 {
            yaml.push_str("          steps: []\n");
        } else {
            yaml.push_str("          steps:\n");
            for j in 0..max_body.min(5) {
                yaml.push_str(&format!(
                    "            - id: b{i}s{j}\n              set:\n                output: out_{i}_{j}\n                value: \"1\"\n"
                ));
            }
        }
    }
    if has_otherwise {
        yaml.push_str("      otherwise: done\n");
    }
    yaml.push_str("  - id: done\n    finish:\n      result: 0\n");
    yaml
}

proptest! {
    #[test]
    fn varied_choose_no_panic(
        branch_count in 1u8..=64u8,
        max_body in 0u8..=5u8,
        has_otherwise in proptest::bool::ANY,
    ) {
        let yaml = varied_choose_yaml(branch_count, max_body, has_otherwise);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            vb_compile::compile_workflow(yaml.as_bytes())
        }));
        prop_assert!(
            result.is_ok(),
            "compile_workflow must never panic; catch_unwind returned Ok"
        );

           // The YAML produced by varied_choose_yaml is structurally valid, but
        // large inputs may exceed the YAML mapping size limit (1024 entries).
        // The primary goal is to verify the compiler never panics; compilation
        // may fail with a limit_exceeded error for very large inputs.
        #[allow(clippy::unwrap_used)]
        let inner = result.unwrap();
        match &inner {
            Ok(workflow) => {
                prop_assert!(
                    workflow.node_count() >= 2,
                    "varied_choose_yaml Ok must compile to a workflow with >= 2 nodes \
                     (ChooseStart + at least one branch), got node_count={} for input {:?}",
                    workflow.node_count(),
                    workflow
                );
            }
            Err(_) => {
                // Acceptable: YAML mapping limit or other limit_exceeded error.
                // Production code never panics on bad input.
            }
        }
    }

  #[test]
    fn varied_choose_result_is_well_typed(
        branch_count in 1u8..=64u8,
        max_body in 0u8..=5u8,
        has_otherwise in proptest::bool::ANY,
    ) {
        let yaml = varied_choose_yaml(branch_count, max_body, has_otherwise);
        let result = vb_compile::compile_workflow(yaml.as_bytes());

        // Valid YAML compiles to >= 2 nodes (choose + finish) or fails with
        // a graceful error. When max_body == 0, there are no body steps so
        // only 2 nodes are emitted. Large inputs may exceed YAML mapping
        // size limit.
        prop_assert!(
            matches!(result, Ok(ref wf) if wf.node_count() >= 2)
                || matches!(&result, Err(e) if e.0.iter().any(|err| {
                    matches!(err, vb_compile::CompileError::CanonicalYaml { category, .. } if *category == "limit_exceeded")
                })),
            "valid choose yaml compiles to >= 2 nodes or errors gracefully, got {:?}",
            result
        );
    }
}
