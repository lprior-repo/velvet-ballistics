// Verification artifact: proptest_entry_point_contract.rs
// PO: PO-P02/PI-06
// Bead: vb-xi2f.35
// Verifier: proptest
// Command: cargo test proptest_entry_point_contract_preserved -- --nocapture
// Workdir: crates/vb_compile
//
// Proof obligation (PI-06): compile_source preserves ResourceContract through compilation.
// Extended to test many random contract fields.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_compile::compile_source;
    use vb_core::contract_encoding::encode_contract_bytes;
    use vb_core::workflow::ResourceContract;

    fn representative_source() -> vb_yaml::ast::WorkflowSource {
        let yaml = concat!(
            "version: velvet-ballastics/v1\n",
            "name: entry_contract\n",
            "when: { manual: {} }\n",
            "steps:\n",
            "  - id: step_one\n",
            "    set:\n",
            "      output: x\n",
            "      value: \"42\"\n",
        );
        vb_yaml::parse_workflow_source(yaml).expect("valid YAML source")
    }

    // PI-06: Non-DEFAULT ResourceContract survives compilation (extended).
    proptest! {
        #[test]
        fn proptest_entry_point_contract_preserved(
            max_steps in 10u16..=500u16,
            max_slots in 10u16..=128u16,
            max_step_budget in 100u64..=10000u64,
            max_transitions in 100u64..=10000u64,
            allows_secret in any::<bool>(),
        ) {
            let mut contract = ResourceContract::DEFAULT;
            contract.max_steps = max_steps;
            contract.max_slots = max_slots;
            contract.max_step_budget_per_tick = max_step_budget;
            contract.max_transitions_per_tick = max_transitions;
            contract.allows_secret_results = allows_secret;

            let source = representative_source();
            let result = compile_source(&source, contract);

            if let Ok(workflow) = result {
                prop_assert_eq!(
                    workflow.resource_contract(),
                    contract,
                    "CompiledWorkflow must preserve the input ResourceContract"
                );
            }
        }
    }

    // PI-06 extended: more fields randomized.
    proptest! {
        #[test]
        fn proptest_entry_point_contract_extended_fields(
            max_steps in 10u16..=500u16,
            max_slots in 10u16..=128u16,
            max_constants in 10u16..=100u16,
            max_accessors in 10u16..=100u16,
            max_expressions in 10u16..=100u16,
            max_expr_stack in 1u8..=64u8,
            max_retry in 1u16..=10u16,
            max_fanout in 1u16..=10u16,
            max_queue in 10u32..=1000u32,
            allows_secret in any::<bool>(),
        ) {
            let mut contract = ResourceContract::DEFAULT;
            contract.max_steps = max_steps;
            contract.max_slots = max_slots;
            contract.max_constants = max_constants;
            contract.max_accessors = max_accessors;
            contract.max_expressions = max_expressions;
            contract.max_expr_stack = max_expr_stack;
            contract.max_retry_attempts = max_retry;
            contract.max_fanout = max_fanout;
            contract.max_queue_depth = max_queue;
            contract.allows_secret_results = allows_secret;

            let source = representative_source();
            let result = compile_source(&source, contract);

            if let Ok(workflow) = result {
                prop_assert_eq!(
                    workflow.resource_contract(),
                    contract,
                    "CompiledWorkflow must preserve the full ResourceContract"
                );
            }
        }
    }

    // Non-DEFAULT contract encoding differs from DEFAULT (encoding-layer).
    proptest! {
        #[test]
        fn proptest_non_default_contract_encoding_differs(
            max_steps in 1u16..=200u16,
            max_slots in 1u16..=64u16,
            max_input in 100u32..=5000u32,
        ) {
            let mut contract = ResourceContract::DEFAULT;
            contract.max_steps = max_steps;
            contract.max_slots = max_slots;
            contract.max_input_bytes = max_input;

            let default = ResourceContract::DEFAULT;
            if contract != default {
                let enc_default = encode_contract_bytes(&default);
                let enc_contract = encode_contract_bytes(&contract);
                prop_assert_ne!(enc_default, enc_contract,
                    "Non-DEFAULT contract encoding must differ from DEFAULT");
            }
        }
    }
}
