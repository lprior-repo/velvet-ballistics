// Verification artifact: proptest_digest_determinism.rs
// PO: PO-P05/PI-03
// Bead: vb-xi2f.35
// Verifier: proptest
// Command: cargo test proptest_digest_determinism_all_contracts -- --nocapture
// Workdir: crates/vb_compile
//
// Proof obligation (PI-03): canonical_digest is a pure, deterministic function.
// Tests determinism through compile_source (public API).
// Consolidates PO-P04/P05/P06 into single determinism proptest per PF-BR-005.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_compile::compile_source;
    use vb_core::workflow::ResourceContract;

    fn representative_source() -> vb_yaml::ast::WorkflowSource {
        let yaml = concat!(
            "version: velvet-ballastics/v1\n",
            "name: determinism_test\n",
            "when: { manual: {} }\n",
            "steps:\n",
            "  - id: step_one\n",
            "    set:\n",
            "      output: x\n",
            "      value: \"42\"\n",
        );
        vb_yaml::parse_workflow_source(yaml).expect("valid YAML source")
    }

    // PI-03: compile_source is deterministic — a pure function for any contract.
    proptest! {
        #[test]
        fn proptest_digest_determinism_all_contracts(
            max_steps in 10u16..=500u16,
            max_slots in 10u16..=128u16,
            max_step_budget in 100u64..=10000u64,
            max_transitions in 100u64..=10000u64,
            max_input in 100u32..=5000u32,
            max_output in 100u32..=5000u32,
            max_blob in 100u64..=5000u64,
            allows_secret in any::<bool>(),
        ) {
            let mut contract = ResourceContract::DEFAULT;
            contract.max_steps = max_steps;
            contract.max_slots = max_slots;
            contract.max_step_budget_per_tick = max_step_budget;
            contract.max_transitions_per_tick = max_transitions;
            contract.max_input_bytes = max_input;
            contract.max_output_bytes = max_output;
            contract.max_blob_bytes = max_blob;
            contract.allows_secret_results = allows_secret;

            let source = representative_source();

            let result_a = compile_source(&source, contract);
            let result_b = compile_source(&source, contract);

            if let (Ok(wf_a), Ok(wf_b)) = (result_a, result_b) {
                prop_assert_eq!(wf_a.digest(), wf_b.digest(),
                    "compile_source must be deterministic for identical inputs");
                prop_assert_eq!(wf_a.resource_contract(), wf_b.resource_contract(),
                    "Contract preservation must be deterministic");
                // Also verify contract roundtrips
                prop_assert_eq!(wf_a.resource_contract(), contract,
                    "Resource contract must roundtrip through compilation");
            }
        }
    }

    // DEFAULT contract determinism at scale.
    proptest! {
        #[test]
        fn proptest_default_determinism_extended(
            _dummy in 0u32..1000u32,
        ) {
            let source = representative_source();
            let default = ResourceContract::DEFAULT;

            let result_a = compile_source(&source, default);
            let result_b = compile_source(&source, default);

            if let (Ok(wf_a), Ok(wf_b)) = (result_a, result_b) {
                prop_assert_eq!(wf_a.digest(), wf_b.digest(),
                    "DEFAULT must produce deterministic digests at scale");
                prop_assert_eq!(wf_a.resource_contract(), default,
                    "DEFAULT must roundtrip through compilation");
            } else {
                prop_assert!(false, "compile_source failed with DEFAULT contract");
            }
        }
    }

    // Determinism holds for extreme contract values.
    proptest! {
        #[test]
        fn proptest_extreme_values_determinism(
            max_steps in 1u16..=10000u16,
            max_slots in 1u16..=1024u16,
        ) {
            let mut contract = ResourceContract::DEFAULT;
            contract.max_steps = max_steps;
            contract.max_slots = max_slots;

            let source = representative_source();

            let result_a = compile_source(&source, contract);
            let result_b = compile_source(&source, contract);

            if let (Ok(wf_a), Ok(wf_b)) = (result_a, result_b) {
                prop_assert_eq!(wf_a.digest(), wf_b.digest(),
                    "Deterministic digests for extreme contract values");
            }
        }
    }
}
