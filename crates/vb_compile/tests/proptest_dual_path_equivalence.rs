// Verification artifact: proptest_dual_path_equivalence.rs
// PO: PO-P04/PI-04
// Bead: vb-xi2f.35
// Verifier: proptest
// Command: cargo test proptest_digest_determinism_varied_contract -- --nocapture
// Workdir: crates/vb_compile
//
// Proof obligation (PI-04): Originally scoped as dual-path digest equivalence.
// Current coverage is DETERMINISM-ONLY — verifies that repeated calls to the
// single active compilation path (mod_compile_lowering) produce identical digests.
//
// GOD RULE: Calls actual production compile_source.
//
// NOTE: Only one compilation path (mod_compile_lowering) is active.
// True dual-path testing (calling part_05::canonical_digest AND
// compile::mod::canonical_digest independently) is deferred until
// compile/mod.rs is activated. When that happens, extend this proptest
// to exercise both paths independently and compare results.
//
// Renamed from proptest_dual_path_digest_equivalence to accurately reflect
// current determinism-only coverage (PF-BR-001).

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_compile::compile_source;
    use vb_core::workflow::ResourceContract;

    fn representative_source() -> vb_yaml::ast::WorkflowSource {
        let yaml = concat!(
            "version: velvet-ballastics/v1\n",
            "name: dual_path\n",
            "when: { manual: {} }\n",
            "steps:\n",
            "  - id: step_one\n",
            "    set:\n",
            "      output: x\n",
            "      value: \"42\"\n",
        );
        vb_yaml::parse_workflow_source(yaml).expect("valid YAML source")
    }

    // PI-04 (DETERMINISM-ONLY): compile_source produces deterministic digests
    // across many varied contract inputs. Does NOT exercise dual-path equivalence.
    proptest! {
        #[test]
        fn proptest_digest_determinism_varied_contract(
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

            let result_a = compile_source(&source, contract);
            let result_b = compile_source(&source, contract);

            if let (Ok(wf_a), Ok(wf_b)) = (result_a, result_b) {
                prop_assert_eq!(wf_a.digest(), wf_b.digest(),
                    "compile_source must produce identical digest on repeated calls");
                prop_assert_eq!(wf_a.resource_contract(), wf_b.resource_contract(),
                    "Contract must be preserved identically on repeated calls");
                prop_assert_eq!(
                    wf_a.resource_contract(), contract,
                    "Resource contract must roundtrip"
                );
            } else {
                prop_assert!(false, "compile_source failed unexpectedly");
            }
        }
    }

    // Verify determinism across many DEFAULT contract calls.
    // (DETERMINISM-ONLY — no dual-path assertion.)
    proptest! {
        #[test]
        fn proptest_default_contract_determinism(
            _rounds in 0u32..10u32,
        ) {
            let source = representative_source();
            let default = ResourceContract::DEFAULT;

            let wf_a = compile_source(&source, default)
                .expect("compile must succeed");
            let wf_b = compile_source(&source, default)
                .expect("compile must succeed");

            prop_assert_eq!(wf_a.digest(), wf_b.digest(),
                "DEFAULT contract must produce deterministic digests");
            prop_assert_eq!(wf_a.resource_contract(), wf_b.resource_contract(),
                "DEFAULT contract must be preserved identically");
        }
    }

    // Determinism across many (source, contract) pairs — extended coverage.
    // (DETERMINISM-ONLY — no dual-path assertion.)
    proptest! {
        #[test]
        fn proptest_determinism_extended_coverage(
            a_s in 1u16..=200u16,
            a_l in 1u16..=64u16,
            a_c in 1u16..=32u16,
            a_x in 1u8..=32u8,
            a_b in 1u64..=100u64,
            a_t in 1u64..=100u64,
            a_secret in any::<bool>(),
            b_s in 1u16..=200u16,
        ) {
            let mut contract_a = ResourceContract::DEFAULT;
            contract_a.max_steps = a_s;
            contract_a.max_slots = a_l;
            contract_a.max_constants = a_c;
            contract_a.max_expr_stack = a_x;
            contract_a.max_step_budget_per_tick = a_b;
            contract_a.max_transitions_per_tick = a_t;
            contract_a.allows_secret_results = a_secret;

            let mut contract_b = ResourceContract::DEFAULT;
            contract_b.max_steps = b_s;

            let source = representative_source();

            // Determinism for contract_a
            let digest_a1 = compile_source(&source, contract_a)
                .expect("compile must succeed").digest();
            let digest_a2 = compile_source(&source, contract_a)
                .expect("compile must succeed").digest();
            prop_assert_eq!(digest_a1, digest_a2,
                "Deterministic digest for contract_a");

            // Determinism for contract_b
            let digest_b1 = compile_source(&source, contract_b)
                .expect("compile must succeed").digest();
            let digest_b2 = compile_source(&source, contract_b)
                .expect("compile must succeed").digest();
            prop_assert_eq!(digest_b1, digest_b2,
                "Deterministic digest for contract_b");

            // Different contracts → different digests
            if contract_a != contract_b {
                prop_assert_ne!(digest_a1, digest_b1,
                    "Different contracts must produce different digests");
            }
        }
    }
}
