#![forbid(unsafe_code)]

use velvet_ballastics_workspace_tests::acceptance_catalog::{
    CatalogValidationError, DiagnosticMismatch, FailureFamily, FailureTaxonomyContractError,
    FailureTaxonomyRow, OracleKind, PublicSurface, Scenario, ScenarioField, catalog,
    failure_taxonomy_catalog, validate_catalog, validate_failure_taxonomy,
};

fn complete_rows() -> Vec<FailureTaxonomyRow> {
    failure_taxonomy_catalog()
        .iter()
        .filter(|row| row.bead == "vb-82ah")
        .cloned()
        .collect()
}

fn acceptance_scenario(id: &'static str) -> Scenario {
    Scenario {
        id,
        master_behavior: "failure taxonomy validation mutation guard",
        given: "Given an isolated fixture",
        when: "When the public validation surface runs",
        then: "Then exact diagnostic evidence is asserted",
        public_surface: "velvet_ballastics_workspace_tests::acceptance_catalog",
        fixture: "isolated mutation guard fixture",
        expected_outcome: Some("accepted exact evidence"),
        expected_error: Some("exact diagnostic"),
        durability_profile: "no persistent runtime state",
        related_bead: "vb-82ah",
        executable_evidence_target: Some(
            "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_catalog.rs",
        ),
        deferred_follow_up_bead: None,
    }
}

#[test]
fn acceptance_catalog_rejects_each_missing_gwt_field_independently() {
    let cases = [
        (
            Scenario {
                given: "",
                ..acceptance_scenario("VB-82AH-ACCEPTANCE-MISSING-GIVEN")
            },
            "VB-82AH-ACCEPTANCE-MISSING-GIVEN",
        ),
        (
            Scenario {
                when: "",
                ..acceptance_scenario("VB-82AH-ACCEPTANCE-MISSING-WHEN")
            },
            "VB-82AH-ACCEPTANCE-MISSING-WHEN",
        ),
        (
            Scenario {
                then: "",
                ..acceptance_scenario("VB-82AH-ACCEPTANCE-MISSING-THEN")
            },
            "VB-82AH-ACCEPTANCE-MISSING-THEN",
        ),
    ];

    for (scenario, scenario_id) in cases {
        let result = validate_catalog(&[scenario]);

        assert_eq!(
            result,
            Err(CatalogValidationError::MissingGivenWhenThen {
                scenario_id: scenario_id.to_owned(),
            })
        );
    }
}

#[test]
fn acceptance_catalog_rejects_each_invalid_executable_target_shape() {
    let cases = [
        (
            "VB-82AH-TARGET-WRONG-ROOT",
            "crates/workspace_tests/src/not_a_test.rs",
        ),
        (
            "VB-82AH-TARGET-WRONG-SUFFIX",
            "crates/workspace_tests/tests/not_rust.txt",
        ),
        ("VB-82AH-TARGET-EMPTY", ""),
    ];

    for (scenario_id, target) in cases {
        let result = validate_catalog(&[Scenario {
            executable_evidence_target: Some(target),
            ..acceptance_scenario(scenario_id)
        }]);

        assert_eq!(
            result,
            Err(CatalogValidationError::InvalidExecutableEvidenceTarget {
                scenario_id: scenario_id.to_owned(),
            })
        );
    }

    assert_eq!(validate_catalog(catalog()), Ok(()));
}

#[test]
fn acceptance_catalog_rejects_private_and_helper_surfaces_independently() {
    let cases = [
        ("VB-82AH-PRIVATE-SURFACE", "private runtime slot reader"),
        ("VB-82AH-HELPER-SURFACE", "fixture helper only"),
    ];

    for (scenario_id, public_surface) in cases {
        let result = validate_catalog(&[Scenario {
            public_surface,
            ..acceptance_scenario(scenario_id)
        }]);

        assert_eq!(
            result,
            Err(CatalogValidationError::PrivateSurface {
                scenario_id: scenario_id.to_owned(),
            })
        );
    }
}

#[test]
fn catalog_requires_given_when_then_and_executable_target() {
    let missing_gwt_cases = [
        (
            FailureTaxonomyRow::fixture("VB-82AH-MISSING-GIVEN").without_given(),
            ScenarioField::Given,
            "VB-82AH-MISSING-GIVEN",
        ),
        (
            FailureTaxonomyRow::fixture("VB-82AH-MISSING-WHEN").without_when(),
            ScenarioField::When,
            "VB-82AH-MISSING-WHEN",
        ),
        (
            FailureTaxonomyRow::fixture("VB-82AH-MISSING-THEN").without_then(),
            ScenarioField::Then,
            "VB-82AH-MISSING-THEN",
        ),
    ];

    for (bad_row, missing_field, scenario_id) in missing_gwt_cases {
        let mut rows = complete_rows();
        rows.push(
            bad_row
                .with_family(FailureFamily::Yaml)
                .with_public_surface(PublicSurface::Cli)
                .with_oracle(OracleKind::ExactDiagnosticCode("DUPLICATE_KEY"))
                .with_executable_target(
                    "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_yaml_validation.rs",
                ),
        );

        let result = validate_failure_taxonomy(&rows);

        assert_eq!(
            result,
            Err(FailureTaxonomyContractError::MissingGwt {
                scenario_id: scenario_id.to_owned(),
                missing_field,
            })
        );
    }

    let mut rows = complete_rows();
    rows.push(
        FailureTaxonomyRow::fixture("VB-82AH-MISSING-EXECUTABLE")
            .with_family(FailureFamily::Yaml)
            .with_public_surface(PublicSurface::Cli)
            .with_oracle(OracleKind::ExactDiagnosticCode("DUPLICATE_KEY"))
            .without_executable_target(),
    );

    let result = validate_failure_taxonomy(&rows);

    assert_eq!(
        result,
        Err(FailureTaxonomyContractError::MissingExecutableTarget {
            scenario_id: "VB-82AH-MISSING-EXECUTABLE".to_owned(),
        })
    );
}

#[test]
fn failure_catalog_maps_each_error_family_to_public_diagnostic() {
    let required = [
        FailureFamily::Yaml,
        FailureFamily::Validation,
        FailureFamily::CompileLowering,
        FailureFamily::RuntimeCore,
        FailureFamily::ActionResourceAdmission,
        FailureFamily::StorageRecovery,
        FailureFamily::Ipc,
        FailureFamily::Replay,
        FailureFamily::GeneratedParity,
        FailureFamily::CliDiagnostics,
    ];

    for missing in required {
        let rows: Vec<FailureTaxonomyRow> = complete_rows()
            .into_iter()
            .filter(|row| row.failure_family != missing)
            .collect();

        let result = validate_failure_taxonomy(&rows);

        assert_eq!(
            result,
            Err(FailureTaxonomyContractError::MissingFailureFamily { family: missing })
        );
    }
}

#[test]
fn catalog_rejects_private_surface_only_negative_scenario() {
    let rows = [FailureTaxonomyRow::fixture("VB-82AH-PRIVATE-ONLY")
        .with_family(FailureFamily::RuntimeCore)
        .with_public_surface(PublicSurface::PrivateHelper(
            "vb_core::frame::read_slot_unchecked",
        ))
        .with_oracle(OracleKind::ExactDiagnosticCode("INVALID_COMPILED_WORKFLOW"))
        .with_executable_target(
            "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_compile_runtime.rs",
        )];

    let result = validate_failure_taxonomy(&rows);

    assert_eq!(
        result,
        Err(FailureTaxonomyContractError::PrivateSurfaceOnly {
            scenario_id: "VB-82AH-PRIVATE-ONLY".to_owned(),
            surface: "vb_core::frame::read_slot_unchecked".to_owned(),
        })
    );
}

#[test]
fn catalog_rejects_weak_negative_assertions() {
    let weak_oracles = [
        OracleKind::BooleanIsErrOnly,
        OracleKind::SubstringOnly("error"),
        OracleKind::ProseOnly("should fail somehow"),
    ];

    for oracle in weak_oracles {
        let rows = [FailureTaxonomyRow::fixture("VB-82AH-WEAK-ORACLE")
            .with_family(FailureFamily::CliDiagnostics)
            .with_public_surface(PublicSurface::Cli)
            .with_oracle(oracle)
            .with_executable_target(
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_generated_cli.rs",
            )];

        let result = validate_failure_taxonomy(&rows);

        assert_eq!(
            result,
            Err(FailureTaxonomyContractError::WeakOracle {
                scenario_id: "VB-82AH-WEAK-ORACLE".to_owned(),
                oracle_kind: oracle,
            })
        );
    }
}

#[test]
fn taxonomy_matrix_requires_typed_error_or_stable_code() {
    let rows = [FailureTaxonomyRow::fixture("VB-82AH-NO-TYPED-CODE")
        .with_family(FailureFamily::StorageRecovery)
        .with_public_surface(PublicSurface::StorageApi)
        .with_oracle(OracleKind::ArtifactAbsenceOnly)
        .with_executable_target(
            "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_storage_ipc_replay.rs",
        )];

    let result = validate_failure_taxonomy(&rows);

    assert_eq!(
        result,
        Err(FailureTaxonomyContractError::MissingTypedMapping {
            scenario_id: "VB-82AH-NO-TYPED-CODE".to_owned(),
            family: FailureFamily::StorageRecovery,
        })
    );
}

#[test]
fn failure_taxonomy_scenarios_are_fixture_isolated_and_order_independent() {
    let normal_rows = complete_rows();
    let mut reversed_rows = normal_rows.clone();
    reversed_rows.reverse();

    let normal_result = validate_failure_taxonomy(&normal_rows);
    let reversed_result = validate_failure_taxonomy(&reversed_rows);

    assert_eq!(normal_result, Ok(normal_rows.clone()));
    assert_eq!(reversed_result, Ok(normal_rows.clone()));

    for row in normal_rows {
        let scenario_id = row.scenario_id.clone();
        let normal_observation = row
            .clone()
            .with_temp_fixture_root("target/vb-82ah-order-normal")
            .fixture_isolation_observation();
        let reversed_observation = row
            .clone()
            .with_temp_fixture_root("target/vb-82ah-order-reversed")
            .fixture_isolation_observation();

        assert_eq!(
            normal_observation.scenario_id,
            reversed_observation.scenario_id
        );
        assert_eq!(
            normal_observation.expected_diagnostic,
            reversed_observation.expected_diagnostic
        );
        assert_eq!(
            normal_observation.artifact_probes,
            reversed_observation.artifact_probes
        );
        assert_eq!(
            normal_observation.evidence_path_without_temp_root,
            reversed_observation.evidence_path_without_temp_root
        );
        assert_eq!(normal_observation.storage_namespace, scenario_id);
        assert_eq!(normal_observation.ipc_namespace, scenario_id);
        assert_eq!(normal_observation.output_namespace, scenario_id);
        assert_eq!(normal_observation.generated_artifact_namespace, scenario_id);
        assert_eq!(reversed_observation.storage_namespace, scenario_id);
        assert_eq!(reversed_observation.ipc_namespace, scenario_id);
        assert_eq!(reversed_observation.output_namespace, scenario_id);
        assert_eq!(
            reversed_observation.generated_artifact_namespace,
            scenario_id
        );
    }
}

#[test]
fn catalog_reports_exact_mismatch_context_and_evidence_path() {
    let rows = [FailureTaxonomyRow::fixture("VB-82AH-MISMATCH-DIAGNOSTIC")
        .with_family(FailureFamily::Yaml)
        .with_public_surface(PublicSurface::Cli)
        .with_oracle(OracleKind::ExactDiagnosticCode("DUPLICATE_KEY"))
        .with_observed_diagnostic_code("UNKNOWN_REFERENCE")
        .with_evidence_path(".evidence/vb-82ah/VB-82AH-MISMATCH-DIAGNOSTIC/diagnostic.json")
        .with_executable_target(
            "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_yaml_validation.rs",
        )];

    let result = validate_failure_taxonomy(&rows);

    assert_eq!(
        result,
        Err(FailureTaxonomyContractError::DiagnosticMismatch {
            mismatch: Box::new(DiagnosticMismatch {
                scenario_id: "VB-82AH-MISMATCH-DIAGNOSTIC".to_owned(),
                failure_family: FailureFamily::Yaml,
                public_surface: PublicSurface::Cli,
                expected: "DUPLICATE_KEY".to_owned(),
                actual: "UNKNOWN_REFERENCE".to_owned(),
                evidence_path: ".evidence/vb-82ah/VB-82AH-MISMATCH-DIAGNOSTIC/diagnostic.json"
                    .to_owned(),
            }),
        })
    );
}

#[test]
fn failure_taxonomy_evidence_traces_to_bead_and_master_sections() {
    let proof_obligations = include_str!("../../../.beads/vb-82ah/proof-obligations.planned.jsonl");

    for row in complete_rows() {
        assert_eq!(row.bead, "vb-82ah");
        assert_eq!(row.master_sections.is_empty(), false);
        assert_eq!(row.executable_target.is_empty(), false);
        assert_eq!(row.proof_obligation_id.is_empty(), false);

        let compact_proof_id_json = format!("\"id\":\"{}\"", row.proof_obligation_id);
        let spaced_proof_id_json = format!("\"id\": \"{}\"", row.proof_obligation_id);
        assert_eq!(
            proof_obligations.contains(&compact_proof_id_json)
                || proof_obligations.contains(&spaced_proof_id_json),
            true
        );
    }
}
