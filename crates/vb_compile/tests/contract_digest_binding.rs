// Verification artifact: contract_digest_binding.rs
// Bead: vb-xi2f.35 — Behaviors A1–A8
// Verifier: unit + integration tests
//
// Tests that canonical_digest binds ResourceContract into the workflow digest
// and that compile_source preserves the contract.

#[cfg(test)]
mod tests {
    use vb_compile::compile_source;
    use vb_core::contract_encoding::encode_contract_bytes;
    use vb_core::workflow::ResourceContract;

    /// Canonical YAML source used as a representative workflow.
    fn representative_source() -> vb_yaml::ast::WorkflowSource {
        let yaml = concat!(
            "version: velvet-ballastics/v1\n",
            "name: digest_binding_test\n",
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
    // A1: canonical_digest is deterministic
    // -----------------------------------------------------------------------

    #[test]
    fn canonical_digest_produces_identical_result_when_same_inputs_called_twice() {
        let source = representative_source();
        let contract = ResourceContract::DEFAULT;

        let result_a = compile_source(&source, contract);
        let result_b = compile_source(&source, contract);

        let wf_a = result_a.expect("first compilation must succeed");
        let wf_b = result_b.expect("second compilation must succeed");

        assert_eq!(
            wf_a.digest(),
            wf_b.digest(),
            "Same (source, contract) must produce identical WorkflowDigest"
        );
        assert_eq!(
            wf_a.resource_contract(),
            wf_b.resource_contract(),
            "Same contract must produce identical resource_contract()"
        );
    }

    #[test]
    fn canonical_digest_is_deterministic_with_default_contract() {
        let source = representative_source();
        let contract = ResourceContract::DEFAULT;

        let digests: Vec<_> = (0..50)
            .map(|_| {
                compile_source(&source, contract)
                    .expect("compile must succeed")
                    .digest()
            })
            .collect();

        for d in &digests[1..] {
            assert_eq!(
                digests[0], *d,
                "All digests with DEFAULT contract must be identical"
            );
        }
    }

    // -----------------------------------------------------------------------
    // A2: Single-field sensitivity (each of 17 fields)
    // -----------------------------------------------------------------------

    /// Test helper: verifies that changing a single field changes the digest.
    fn assert_field_changes_digest(contract: ResourceContract) {
        let source = representative_source();
        let digest_original = compile_source(&source, ResourceContract::DEFAULT)
            .expect("compile must succeed")
            .digest();

        let digest_modified = compile_source(&source, contract)
            .expect("compile must succeed")
            .digest();

        assert_ne!(
            digest_original, digest_modified,
            "Changing a ResourceContract field must change the digest"
        );
    }

    #[test]
    fn canonical_digest_differs_when_max_steps_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_steps = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_slots_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_slots = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_constants_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_constants = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_accessors_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_accessors = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_expressions_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_expressions = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_expr_stack_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_expr_stack = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_step_budget_per_tick_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_step_budget_per_tick = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_transitions_per_tick_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_transitions_per_tick = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_input_bytes_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_input_bytes = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_output_bytes_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_output_bytes = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_blob_bytes_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_blob_bytes = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_ipc_payload_bytes_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_ipc_payload_bytes = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_retry_attempts_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_retry_attempts = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_fanout_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_fanout = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_collect_items_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_collect_items = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_queue_depth_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_queue_depth = 1;
        assert_field_changes_digest(c);
    }

    #[test]
    fn canonical_digest_differs_when_max_journal_batch_bytes_differs() {
        let mut c = ResourceContract::DEFAULT;
        c.max_journal_batch_bytes = 1;
        assert_field_changes_digest(c);
    }

    // -----------------------------------------------------------------------
    // A3: Multi-field sensitivity
    // -----------------------------------------------------------------------

    #[test]
    fn canonical_digest_differs_when_multiple_fields_differ() {
        let source = representative_source();
        let digest_default = compile_source(&source, ResourceContract::DEFAULT)
            .expect("compile must succeed")
            .digest();

        let mut contract_altered = ResourceContract::DEFAULT;
        contract_altered.max_steps = 5;
        contract_altered.max_slots = 3;
        contract_altered.max_input_bytes = 42;
        contract_altered.allows_secret_results = true;

        let digest_altered = compile_source(&source, contract_altered)
            .expect("compile must succeed")
            .digest();

        assert_ne!(
            digest_default, digest_altered,
            "Multi-field changed contract must produce different digest"
        );
    }

    // -----------------------------------------------------------------------
    // A4: allows_secret_results digest sensitivity
    // -----------------------------------------------------------------------

    #[test]
    fn canonical_digest_differs_when_allows_secret_results_toggled() {
        let source = representative_source();

        let mut contract_false = ResourceContract::DEFAULT;
        contract_false.allows_secret_results = false;

        let mut contract_true = ResourceContract::DEFAULT;
        contract_true.allows_secret_results = true;

        let digest_false = compile_source(&source, contract_false)
            .expect("compile must succeed")
            .digest();
        let digest_true = compile_source(&source, contract_true)
            .expect("compile must succeed")
            .digest();

        assert_ne!(
            digest_false, digest_true,
            "Digest must differ when allows_secret_results toggles"
        );
    }

    // -----------------------------------------------------------------------
    // A5: Stable field ordering in encode_contract_bytes
    // -----------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_preserves_field_ordering_across_calls() {
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 100;
        contract.allows_secret_results = true;

        let enc1 = encode_contract_bytes(&contract);
        let enc2 = encode_contract_bytes(&contract);

        assert_eq!(
            enc1, enc2,
            "encode_contract_bytes must produce identical byte order for same input"
        );
    }

    // -----------------------------------------------------------------------
    // A6: Domain-tagged fields prevent cross-field collision
    // -----------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_domain_tags_prevent_cross_field_collision() {
        let mut contract_a = ResourceContract::DEFAULT;
        contract_a.max_steps = 42;
        contract_a.max_slots = 0;

        let mut contract_b = ResourceContract::DEFAULT;
        contract_b.max_steps = 0;
        contract_b.max_slots = 42;

        let enc_a = encode_contract_bytes(&contract_a);
        let enc_b = encode_contract_bytes(&contract_b);

        assert_ne!(
            enc_a, enc_b,
            "Different field assignments with same value must not collide — domain tags must differ"
        );
    }

    // -----------------------------------------------------------------------
    // A7: Digest determinism (not dual-path equivalence)
    //
    // COVERAGE: DETERMINISM-ONLY. The compile/mod.rs module is not currently
    // activated in the crate module tree (no `mod compile;` in lib.rs). The
    // only active compilation path is through mod_compile_lowering. This test
    // verifies strong determinism through repeated calls to the single active
    // path. True dual-path equivalence testing (calling part_05::canonical_digest
    // AND compile::mod::canonical_digest independently) is deferred until
    // compile/mod.rs is activated.
    //
    // Proptest obligation PO-P04 is currently satisfied by the determinism
    // proptests in proptest_digest_determinism.rs and proptest_dual_path_equivalence.rs.
    // -----------------------------------------------------------------------

    /// Determinism-only: verifies repeated calls to the single active compilation
    /// path produce identical digests. Does NOT exercise true dual-path equivalence.
    #[test]
    fn canonical_digest_is_deterministic_across_multiple_computations() {
        let source = representative_source();
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 77;
        contract.allows_secret_results = true;

        // Call compile_source many times — every call must produce identical digest
        let first_digest = compile_source(&source, contract)
            .expect("compile must succeed")
            .digest();

        for _ in 0..100 {
            let digest = compile_source(&source, contract)
                .expect("compile must succeed")
                .digest();
            assert_eq!(
                first_digest, digest,
                "Deterministic digest must not vary across calls"
            );
        }
    }

    // -----------------------------------------------------------------------
    // A8: DEFAULT contract digest determinism
    // -----------------------------------------------------------------------

    #[test]
    fn canonical_digest_known_answer_for_default_contract() {
        // Golden-hash KAT: verifies that DEFAULT contract + canonical source
        // produces a specific digest. Any change to ResourceContract::DEFAULT
        // or canonical_digest must update this golden value.
        let source = representative_source();
        let digest = compile_source(&source, ResourceContract::DEFAULT)
            .expect("compile must succeed")
            .digest();

        // Assert a specific golden hash value: if DEFAULT contract constants
        // or canonical_digest changes, this assertion must be updated.
        let bytes = digest.as_bytes();
        assert_eq!(bytes.len(), 32, "Digest must be 32 bytes");
        assert_eq!(
            bytes,
            [
                0x8b, 0x9a, 0x5d, 0x83, 0xe9, 0xcf, 0xee, 0x44, 0xe0, 0xae, 0x68, 0x34, 0x40, 0x8f,
                0x16, 0x6e, 0x34, 0x46, 0x40, 0xd1, 0x10, 0x11, 0x3c, 0xf3, 0xfe, 0x5e, 0xac, 0xd7,
                0x29, 0x80, 0xa1, 0x62,
            ],
            "Golden hash for DEFAULT contract must match; update if DEFAULT constants or canonical_digest changes"
        );

        // Re-compute to verify determinism
        let digest2 = compile_source(&source, ResourceContract::DEFAULT)
            .expect("compile must succeed")
            .digest();
        assert_eq!(
            digest, digest2,
            "DEFAULT digest must be deterministic — KAT consistency check"
        );
    }

    #[test]
    fn compile_source_preserves_resource_contract_roundtrip() {
        let source = representative_source();
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 42;
        contract.max_transitions_per_tick = 77;
        contract.allows_secret_results = true;

        let workflow = compile_source(&source, contract).expect("compile must succeed");

        assert_eq!(
            workflow.resource_contract(),
            contract,
            "ResourceContract must roundtrip through compile_source"
        );
    }

    // -----------------------------------------------------------------------
    // Additional: contract modifies digest but not IR structure
    // -----------------------------------------------------------------------

    #[test]
    fn different_contracts_preserve_same_workflow_structure() {
        let source = representative_source();
        let mut contract_a = ResourceContract::DEFAULT;
        contract_a.max_steps = 100;
        let mut contract_b = ResourceContract::DEFAULT;
        contract_b.max_steps = 200;

        let wf_a = compile_source(&source, contract_a).expect("compile must succeed");
        let wf_b = compile_source(&source, contract_b).expect("compile must succeed");

        // Different contracts → different digests
        assert_ne!(wf_a.digest(), wf_b.digest());

        // But same IR structure
        assert_eq!(
            wf_a.node_count(),
            wf_b.node_count(),
            "Node count must be same regardless of contract"
        );
        assert_eq!(
            wf_a.slot_count(),
            wf_b.slot_count(),
            "Slot count must be same regardless of contract"
        );
    }
}
