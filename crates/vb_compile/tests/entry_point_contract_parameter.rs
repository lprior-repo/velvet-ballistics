// Verification artifact: entry_point_contract_parameter.rs
// Bead: vb-xi2f.35 — Behaviors C1–C6
// Verifier: integration tests
//
// Tests that compile_source accepts and preserves a ResourceContract parameter
// through compilation, and that different contracts produce different digests.

#[cfg(test)]
mod tests {
    use vb_compile::{compile_source, compile_source_with_default};
    use vb_core::workflow::ResourceContract;

    /// Canonical YAML source used as a representative workflow.
    fn representative_source() -> vb_yaml::ast::WorkflowSource {
        let yaml = concat!(
            "version: velvet-ballastics/v1\n",
            "name: entry_point_test\n",
            "when: { manual: {} }\n",
            "steps:\n",
            "  - id: step_one\n",
            "    set:\n",
            "      output: x\n",
            "      value: \"42\"\n",
        );
        vb_yaml::parse_workflow_source(yaml).expect("valid representative YAML source")
    }

    // -----------------------------------------------------------------------
    // C1: compile_source accepts contract parameter
    // -----------------------------------------------------------------------

    #[test]
    fn compile_source_accepts_contract_parameter() {
        let source = representative_source();
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 500;

        // This proves compile_source(&source, contract) compiles, runs,
        // AND preserves the contract through compilation.
        let workflow = compile_source(&source, contract)
            .expect("compile_source must accept a ResourceContract parameter");
        assert_eq!(
            workflow.resource_contract(),
            contract,
            "compile_source must preserve the contract parameter (max_steps={})",
            contract.max_steps
        );
    }

    #[test]
    fn compile_source_accepts_extreme_contract_values() {
        let source = representative_source();
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 10_000;
        contract.max_transitions_per_tick = 1_000;
        contract.max_step_budget_per_tick = 10_000;
        contract.allows_secret_results = true;

        let workflow = compile_source(&source, contract)
            .expect("compile_source must accept edge-case contract values");
        assert_eq!(
            workflow.resource_contract(),
            contract,
            "compile_source must preserve extreme contract values (max_steps={}, max_transitions={}, max_step_budget={}, allows_secret={})",
            contract.max_steps,
            contract.max_transitions_per_tick,
            contract.max_step_budget_per_tick,
            contract.allows_secret_results
        );
        // Verify individual extreme fields roundtrip
        assert_eq!(workflow.resource_contract().max_steps, 10_000);
        assert_eq!(workflow.resource_contract().max_transitions_per_tick, 1_000);
        assert_eq!(
            workflow.resource_contract().max_step_budget_per_tick,
            10_000
        );
        assert_eq!(workflow.resource_contract().allows_secret_results, true);
    }

    // -----------------------------------------------------------------------
    // C2: compile_source with DEFAULT preserves DEFAULT
    // -----------------------------------------------------------------------

    #[test]
    fn compile_source_with_default_contract_preserves_default() {
        let source = representative_source();
        let default = ResourceContract::DEFAULT;

        let workflow = compile_source(&source, default).expect("compile must succeed");

        assert_eq!(
            workflow.resource_contract(),
            default,
            "Compile with DEFAULT must preserve DEFAULT contract"
        );
    }

    // -----------------------------------------------------------------------
    // C3: compile_source preserves non-default contract
    // -----------------------------------------------------------------------

    #[test]
    fn compile_source_preserves_non_default_contract_after_compilation() {
        let source = representative_source();
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 42;
        contract.max_slots = 7;
        contract.max_input_bytes = 1024;
        contract.allows_secret_results = true;

        let workflow = compile_source(&source, contract).expect("compile must succeed");

        assert_eq!(
            workflow.resource_contract(),
            contract,
            "Non-default contract must be preserved through compilation"
        );
    }

    #[test]
    fn compile_source_preserves_all_contract_fields_individually() {
        let source = representative_source();

        // Test each of the 17 fields independently
        let test_fields: &[(fn(&mut ResourceContract), &str)] = &[
            (|c: &mut ResourceContract| c.max_steps = 77, "max_steps"),
            (|c: &mut ResourceContract| c.max_slots = 77, "max_slots"),
            (
                |c: &mut ResourceContract| c.max_constants = 77,
                "max_constants",
            ),
            (
                |c: &mut ResourceContract| c.max_accessors = 77,
                "max_accessors",
            ),
            (
                |c: &mut ResourceContract| c.max_expressions = 77,
                "max_expressions",
            ),
            (
                |c: &mut ResourceContract| c.max_expr_stack = 32,
                "max_expr_stack",
            ),
            (
                |c: &mut ResourceContract| c.max_step_budget_per_tick = 7777,
                "max_step_budget_per_tick",
            ),
            (
                |c: &mut ResourceContract| c.max_transitions_per_tick = 7777,
                "max_transitions_per_tick",
            ),
            (
                |c: &mut ResourceContract| c.max_input_bytes = 7777,
                "max_input_bytes",
            ),
            (
                |c: &mut ResourceContract| c.max_output_bytes = 7777,
                "max_output_bytes",
            ),
            (
                |c: &mut ResourceContract| c.max_blob_bytes = 7777,
                "max_blob_bytes",
            ),
            (
                |c: &mut ResourceContract| c.max_ipc_payload_bytes = 7777,
                "max_ipc_payload_bytes",
            ),
            (
                |c: &mut ResourceContract| c.max_retry_attempts = 7,
                "max_retry_attempts",
            ),
            (|c: &mut ResourceContract| c.max_fanout = 7, "max_fanout"),
            (
                |c: &mut ResourceContract| c.max_collect_items = 7777,
                "max_collect_items",
            ),
            (
                |c: &mut ResourceContract| c.max_queue_depth = 512,
                "max_queue_depth",
            ),
            (
                |c: &mut ResourceContract| c.max_journal_batch_bytes = 7777,
                "max_journal_batch_bytes",
            ),
        ];

        for (setter, field_name) in test_fields {
            let mut contract = ResourceContract::DEFAULT;
            setter(&mut contract);

            let workflow = compile_source(&source, contract)
                .unwrap_or_else(|e| panic!("compile must succeed for field {field_name}: {e:?}"));

            assert_eq!(
                workflow.resource_contract(),
                contract,
                "Field {} must roundtrip through compile_source",
                field_name
            );
        }

        // allows_secret_results (bool) tested separately
        let mut contract = ResourceContract::DEFAULT;
        contract.allows_secret_results = true;
        let workflow = compile_source(&source, contract)
            .expect("compile must succeed with allows_secret_results=true");
        assert_eq!(workflow.resource_contract().allows_secret_results, true);
    }

    // -----------------------------------------------------------------------
    // C4: Both compilation paths accept contract
    //
    // Note: Only one compilation path (mod_compile_lowering) is active.
    // When compile/mod.rs is activated, add a dual-path test here.
    // -----------------------------------------------------------------------

    #[test]
    fn compile_source_path_preserves_resource_contract() {
        // Test that the active compilation path preserves the contract
        let source = representative_source();
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 99;
        contract.max_transitions_per_tick = 500;
        contract.allows_secret_results = true;

        let workflow = compile_source(&source, contract).expect("compile must succeed");

        assert_eq!(
            workflow.resource_contract(),
            contract,
            "Active compilation path must preserve ResourceContract"
        );
    }

    // -----------------------------------------------------------------------
    // C5: compile_source_with_default(source) ≡ compile_source(source, DEFAULT)
    //
    // Verifies that the convenience API compile_source_with_default(source)
    // produces the same digest and contract as calling
    // compile_source(source, ResourceContract::DEFAULT) explicitly.
    // -----------------------------------------------------------------------

    #[test]
    fn compile_source_with_default_equivalent_to_explicit_default() {
        let source = representative_source();

        let wf_with_default =
            compile_source_with_default(&source).expect("compile_source_with_default must succeed");
        let wf_explicit = compile_source(&source, ResourceContract::DEFAULT)
            .expect("compile_source with explicit DEFAULT must succeed");

        assert_eq!(
            wf_with_default.digest(),
            wf_explicit.digest(),
            "compile_source_with_default must produce same digest as compile_source(.., DEFAULT)"
        );
        assert_eq!(
            wf_with_default.resource_contract(),
            wf_explicit.resource_contract(),
            "compile_source_with_default must preserve same contract as explicit DEFAULT"
        );
        assert_eq!(
            wf_with_default.resource_contract(),
            ResourceContract::DEFAULT,
            "compile_source_with_default must preserve DEFAULT contract"
        );
    }

    // -----------------------------------------------------------------------
    // C5a: DEFAULT contract determinism (explicit DEFAULT path)
    // -----------------------------------------------------------------------

    #[test]
    fn compile_source_with_explicit_default_is_deterministic() {
        let source = representative_source();
        let default = ResourceContract::DEFAULT;

        let wf_a =
            compile_source(&source, default).expect("compile with explicit DEFAULT must succeed");
        let wf_b =
            compile_source(&source, default).expect("compile with explicit DEFAULT must succeed");

        assert_eq!(
            wf_a.digest(),
            wf_b.digest(),
            "Explicit DEFAULT must produce deterministic digests"
        );
        assert_eq!(
            wf_a.resource_contract(),
            wf_b.resource_contract(),
            "Explicit DEFAULT must preserve identical contracts"
        );
    }

    // -----------------------------------------------------------------------
    // C6: Different contracts → different CompiledWorkflow
    // -----------------------------------------------------------------------

    #[test]
    fn compile_source_produces_different_digest_and_contract_when_contract_differs() {
        let source = representative_source();

        let mut contract_a = ResourceContract::DEFAULT;
        contract_a.max_steps = 100;
        contract_a.max_input_bytes = 1024;

        let mut contract_b = ResourceContract::DEFAULT;
        contract_b.max_steps = 200;
        contract_b.allows_secret_results = true;

        let wf_a =
            compile_source(&source, contract_a).expect("compile with contract A must succeed");
        let wf_b =
            compile_source(&source, contract_b).expect("compile with contract B must succeed");

        assert_ne!(
            wf_a.digest(),
            wf_b.digest(),
            "Different contracts must produce different digests"
        );
        assert_ne!(
            wf_a.resource_contract(),
            wf_b.resource_contract(),
            "Different contracts must produce different resource_contract() values"
        );
    }

    // -----------------------------------------------------------------------
    // Additional: compile_source rejects valid contract with invalid source
    // -----------------------------------------------------------------------

    #[test]
    fn compile_source_rejects_invalid_source_with_contract_parameter() {
        // Source with empty steps
        let yaml =
            "version: velvet-ballastics/v1\nname: empty_test\nwhen: { manual: {} }\nsteps: []\n";
        let source = vb_yaml::parse_workflow_source(yaml).expect("valid YAML parse");

        let result = compile_source(&source, ResourceContract::DEFAULT);
        match result {
            Err(vb_compile::CompileErrors(errors)) => {
                let has_empty_steps = errors
                    .iter()
                    .any(|e| matches!(e, vb_compile::CompileError::EmptySteps));
                assert!(
                    has_empty_steps,
                    "compile_source must return CompileError::EmptySteps for empty steps, got: {errors:?}"
                );
            }
            Ok(_) => {
                panic!("compile_source must reject empty steps regardless of contract parameter")
            }
        }
    }
}
