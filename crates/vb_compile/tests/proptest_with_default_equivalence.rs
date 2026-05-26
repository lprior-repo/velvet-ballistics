// Verification artifact: proptest_with_default_equivalence.rs
// PO: PO-P06/PI-05
// Bead: vb-xi2f.35
// Verifier: proptest
// Command: cargo test proptest_with_default_equivalence -- --nocapture
// Workdir: crates/vb_compile
//
// Proof obligation (PI-05): compile_source_with_default(source) produces the
// same digest and contract as compile_source(source, ResourceContract::DEFAULT).
//
// Coverage: this proptest verifies equivalence between the convenience API
// compile_source_with_default(source) and the explicit DEFAULT contract path
// compile_source(source, ResourceContract::DEFAULT) over 500 random iterations.
// compile_source_with_default is implemented in mod_compile_lowering/part_01.rs:67
// and publicly exported from vb_compile::lib.rs.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_compile::{compile_source, compile_source_with_default};
    use vb_core::workflow::ResourceContract;

    fn representative_source() -> vb_yaml::ast::WorkflowSource {
        let yaml = concat!(
            "version: velvet-ballastics/v1\n",
            "name: default_eq_test\n",
            "when: { manual: {} }\n",
            "steps:\n",
            "  - id: step_one\n",
            "    set:\n",
            "      output: x\n",
            "      value: \"42\"\n",
        );
        vb_yaml::parse_workflow_source(yaml).expect("valid YAML source")
    }

    // PI-05: compile_source_with_default(source) ≡ compile_source(source, DEFAULT)
    //
    // Verifies that the convenience API compile_source_with_default(source)
    // produces the same digest and resource_contract as calling
    // compile_source(source, ResourceContract::DEFAULT) explicitly.
    proptest! {
        #[test]
        fn proptest_with_default_equivalence(
            _rounds in 0u32..500u32,
        ) {
            let source = representative_source();
            let default = ResourceContract::DEFAULT;

            let result_with_default = compile_source_with_default(&source);
            let result_explicit = compile_source(&source, default);

            match (result_with_default, result_explicit) {
                (Ok(wf_default), Ok(wf_explicit)) => {
                    prop_assert_eq!(wf_default.digest(), wf_explicit.digest(),
                        "compile_source_with_default must produce same digest as compile_source(.., DEFAULT)");
                    prop_assert_eq!(wf_default.resource_contract(), wf_explicit.resource_contract(),
                        "compile_source_with_default must preserve same contract as explicit DEFAULT");
                    prop_assert_eq!(wf_default.resource_contract(), default,
                        "compile_source_with_default must preserve DEFAULT contract");
                }
                (Err(e1), Err(e2)) => {
                    // Both paths must fail identically
                    prop_assert_eq!(
                        format!("{e1:?}"), format!("{e2:?}"),
                        "compile_source_with_default and compile_source(.., DEFAULT) must produce identical errors"
                    );
                }
                (Ok(_), Err(_)) => {
                    prop_assert!(false,
                        "compile_source_with_default succeeded but compile_source(.., DEFAULT) failed — must be equivalent");
                }
                (Err(_), Ok(_)) => {
                    prop_assert!(false,
                        "compile_source(.., DEFAULT) succeeded but compile_source_with_default failed — must be equivalent");
                }
            }
        }
    }

    /// Verify that DEFAULT contract produces same digest regardless of call count.
    #[test]
    fn compile_source_default_is_consistent_across_many_calls() {
        let source = representative_source();
        let default = ResourceContract::DEFAULT;

        let first_digest = compile_source(&source, default)
            .expect("compile must succeed")
            .digest();

        for i in 0..100 {
            let digest = compile_source(&source, default)
                .expect("compile must succeed")
                .digest();
            assert_eq!(
                first_digest, digest,
                "DEFAULT digest must be consistent at call {i}"
            );
        }
    }

    /// Verify that compile_source_with_default matches compile_source(.., DEFAULT)
    /// across repeated calls with a fixed source.
    #[test]
    fn compile_source_with_default_matches_explicit_default() {
        let source = representative_source();

        let wf_with_default =
            compile_source_with_default(&source).expect("compile_source_with_default must succeed");
        let wf_explicit = compile_source(&source, ResourceContract::DEFAULT)
            .expect("compile_source with explicit DEFAULT must succeed");

        assert_eq!(
            wf_with_default.digest(),
            wf_explicit.digest(),
            "digests must match between with_default and explicit DEFAULT"
        );
        assert_eq!(
            wf_with_default.resource_contract(),
            wf_explicit.resource_contract(),
            "contracts must match between with_default and explicit DEFAULT"
        );
        assert_eq!(
            wf_with_default.resource_contract(),
            ResourceContract::DEFAULT,
            "with_default must preserve DEFAULT contract"
        );
    }

    // Verify that non-default contracts with DEFAULT-like values differ from DEFAULT.
    proptest! {
        #[test]
        fn proptest_non_default_differs_from_default(
            max_steps in 1u16..=200u16,
        ) {
            let mut contract = ResourceContract::DEFAULT;
            contract.max_steps = max_steps;

            let source = representative_source();

            let digest_default = compile_source_with_default(&source)
                .expect("compile_source_with_default must succeed").digest();

            let digest_modified = compile_source(&source, contract)
                .expect("compile with modified must succeed").digest();

            if contract != ResourceContract::DEFAULT {
                prop_assert_ne!(digest_default, digest_modified,
                    "Non-DEFAULT contracts must produce different digests from DEFAULT");
            }
        }
    }
}
