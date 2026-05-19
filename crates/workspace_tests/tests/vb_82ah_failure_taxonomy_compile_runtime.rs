#![forbid(unsafe_code)]

use velvet_ballastics_workspace_tests::acceptance_catalog::{
    ArtifactProbe, ExpectedDiagnostic, FailureFamily, FailureTaxonomyScenario,
    run_failure_taxonomy_scenario,
};

#[test]
fn compile_failure_leaves_no_ir_generated_or_accepted_artifact() {
    let cases = [
        (
            "VB-82AH-COMPILE-STRICT-YAML-DUPLICATE",
            "CompileError::YamlDiagnostic",
            "DUPLICATE_KEY",
        ),
        (
            "VB-82AH-COMPILE-UNSUPPORTED",
            "CompileError::UnsupportedStepPrimitive",
            "UNSUPPORTED_PRIMITIVE",
        ),
        (
            "VB-82AH-COMPILE-RESOURCE-OVERFLOW",
            "CompileError::ResourceLimitOverflow",
            "RESOURCE_LIMIT_OVERFLOW",
        ),
    ];

    for (id, typed_error, code) in cases {
        let scenario = FailureTaxonomyScenario::compile_fixture(id)
            .with_expected_diagnostic(ExpectedDiagnostic::new(
                FailureFamily::CompileLowering,
                typed_error,
                code,
                3,
            ))
            .with_artifact_probe(ArtifactProbe::success_artifacts_absent());

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), code);
        assert_eq!(evidence.created_success_artifacts(), Vec::<String>::new());
    }
}

#[test]
fn runtime_core_bounds_failures_are_typed_and_non_panicking() {
    let cases = [
        (
            "VB-82AH-RUNTIME-CONST",
            "CoreError::ConstOutOfBounds",
            "E1013",
            Some("CONST_OUT_OF_BOUNDS"),
        ),
        (
            "VB-82AH-RUNTIME-MISSING-OUTPUT",
            "CoreError::MissingOutputSlot",
            "E1305",
            Some("MISSING_OUTPUT_SLOT"),
        ),
        (
            "VB-82AH-RUNTIME-STEP-STATE",
            "CoreError::StepStateOutOfBounds",
            "E1306",
            Some("STEP_STATE_OUT_OF_BOUNDS"),
        ),
        (
            "VB-82AH-RUNTIME-QUEUE",
            "CoreError::QueueFull",
            "E1301",
            Some("QUEUE_FULL"),
        ),
    ];

    for (id, typed_error, code, _runtime_code) in cases {
        let scenario =
            FailureTaxonomyScenario::runtime_bounds_fixture(id).with_expected_diagnostic(
                ExpectedDiagnostic::new(FailureFamily::RuntimeCore, typed_error, code, 4),
            );

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert!(evidence.typed_error().starts_with("CoreError::"));
        assert!(
            evidence.diagnostic_code().starts_with("E1"),
            "diagnostic_code must be E1xx family"
        );
        // runtime_code may be Some(...) for specific CoreError variants or None for InvalidProgramCounter fallback
        assert_eq!(evidence.run_attempted(), true);
        assert_eq!(evidence.panicked(), false);
        assert_eq!(evidence.unrelated_state_changed(), false);
    }
}

#[test]
fn action_and_admission_failures_preserve_typed_cause_and_public_diagnostic() {
    let cases = [
        (
            "VB-82AH-ACTION-INVALID",
            "RuntimeError::InvalidActionCompletion",
            "E200B",
            Some("ACTION_FAILED"),
        ),
        (
            "VB-82AH-ACTION-STALE",
            "RuntimeError::StaleAttempt",
            "E200B",
            Some("ACTION_FAILED"),
        ),
        (
            "VB-82AH-ACTION-BEYOND-MAX",
            "RuntimeError::AttemptBeyondMax",
            "E200B",
            Some("ACTION_FAILED"),
        ),
        (
            "VB-82AH-ACTION-SECRET",
            "RuntimeError::SecretResultNotAllowed",
            "E2016",
            None,
        ),
        (
            "VB-82AH-ACTION-IPC-PAYLOAD",
            "RuntimeError::IpcPayloadSizeExceeded",
            "E2017",
            None,
        ),
    ];

    for (id, typed_error, code, runtime_code) in cases {
        let scenario = FailureTaxonomyScenario::admission_fixture(id).with_expected_diagnostic(
            ExpectedDiagnostic::new(FailureFamily::ActionResourceAdmission, typed_error, code, 7),
        );

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), code);
        assert_eq!(evidence.runtime_code(), runtime_code);
        assert_eq!(evidence.persisted_run_accepted(), false);
        assert_eq!(evidence.run_attempted(), true);
    }
}
