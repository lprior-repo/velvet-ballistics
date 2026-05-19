#![forbid(unsafe_code)]

use velvet_ballastics_workspace_tests::acceptance_catalog::{
    FailureFamily, FailureTaxonomyScenario, GeneratedParityExpectation,
    run_failure_taxonomy_scenario,
};

#[test]
fn generated_and_ir_negative_outcomes_are_equivalent() {
    let scenario = FailureTaxonomyScenario::generated_parity_fixture("VB-82AH-GEN-UNSAFE-BAIT")
        .with_source(
            "version: velvet-ballastics/v1\nname: bad id\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish: {}\n",
        )
        .with_expected_path("vb_compile::compile_workflow + vb_codegen::emit_rust_workflow")
        .with_generated_parity_expectation(
            GeneratedParityExpectation::BothRejectEquivalentOrUnsupported,
        );

    let evidence = run_failure_taxonomy_scenario(&scenario);

    assert_eq!(
        evidence.ir_diagnostic_family(),
        Some(FailureFamily::Validation)
    );
    assert_eq!(evidence.generated_unsupported_diagnostic(), None);
    assert_eq!(
        evidence.generated_diagnostic_family(),
        Some(FailureFamily::Validation)
    );
    assert_eq!(evidence.diagnostic_code(), "COMPILE_ERROR");
    assert_eq!(
        evidence.diagnostic_path(),
        "vb_compile::compile_workflow + vb_codegen::emit_rust_workflow"
    );
    assert_eq!(
        evidence.generated_and_ir_diagnostic_family_equivalent(),
        true
    );
    assert_eq!(evidence.generated_succeeded_while_ir_rejected(), false);
    assert_eq!(
        evidence.ir_succeeded_while_generated_rejected_without_unsupported(),
        false
    );
}

#[test]
fn cli_negative_diagnostics_use_real_binary_process_surface() {
    let scenario = FailureTaxonomyScenario::cli_fixture("VB-82AH-CLI-VALIDATION")
        .with_generated_parity_expectation(
            GeneratedParityExpectation::BothRejectEquivalentOrUnsupported,
        );

    let evidence = run_failure_taxonomy_scenario(&scenario);

    assert_eq!(evidence.diagnostic_code(), "ValidationFailed");
    assert_eq!(evidence.diagnostic_path(), "public-surface");
    assert_eq!(evidence.cli_exit_code(), Some(1));
}
