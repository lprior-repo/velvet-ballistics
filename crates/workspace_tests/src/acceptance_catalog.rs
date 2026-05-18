#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scenario {
    pub id: &'static str,
    pub master_behavior: &'static str,
    pub given: &'static str,
    pub when: &'static str,
    pub then: &'static str,
    pub public_surface: &'static str,
    pub fixture: &'static str,
    pub expected_outcome: Option<&'static str>,
    pub expected_error: Option<&'static str>,
    pub durability_profile: &'static str,
    pub related_bead: &'static str,
    pub executable_evidence_target: Option<&'static str>,
    pub deferred_follow_up_bead: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogValidationError {
    EmptyCatalog,
    MissingGivenWhenThen { scenario_id: String },
    MissingExactAssertion { scenario_id: String },
    MissingEvidenceDisposition { scenario_id: String },
    ConflictingEvidenceDisposition { scenario_id: String },
    InvalidExecutableEvidenceTarget { scenario_id: String },
    InvalidDeferredFollowUpBead { scenario_id: String },
    PrivateSurface { scenario_id: String },
    SharedFixture { scenario_id: String },
    DuplicateScenarioId { scenario_id: String },
}

const SCENARIOS: &[Scenario] = &[
    Scenario {
        id: "BDD-KYYF-001",
        master_behavior: "cross-run determinism through public runtime evidence",
        given: "the same accepted workflow artifact, input, durability profile, and isolated stores",
        when: "public runtime submission, inspection, storage read, and recovery replay are executed twice",
        then: "terminal result, taint, event signature, digest status, and normalized digest match",
        public_surface: "vb_runtime public API",
        fixture: "isolated durable runtime store per scenario",
        expected_outcome: Some("normalized digest emitted"),
        expected_error: Some("NondeterministicObservation mismatch"),
        durability_profile: "strict durable journal evidence",
        related_bead: "vb-kyyf",
        executable_evidence_target: Some(".evidence/vb-kyyf/bdd-cross-run-determinism.md"),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "BDD-KYYF-002",
        master_behavior: "persisted replay stays reproducible after reopen",
        given: "a strict persisted run with durable journal evidence",
        when: "events_for_run, recovery summary, frame seed, and CLI replay/events/inspect run repeatedly",
        then: "normalized replay digest is stable and sequence numbers stay contiguous",
        public_surface: "vb_storage journal and recovery APIs plus CLI replay/events/inspect",
        fixture: "isolated durable replay store per scenario",
        expected_outcome: Some("normalized replay digest emitted"),
        expected_error: Some("ReplaySequenceViolation mismatch"),
        durability_profile: "strict durable journal evidence",
        related_bead: "vb-kyyf",
        executable_evidence_target: Some(".evidence/vb-kyyf/storage-replay-resume.md"),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "BDD-KYYF-003",
        master_behavior: "non-replay-safe actions are not re-executed",
        given: "a durable scheduled action boundary already resolved by the replay tracker",
        when: "public recovery replays the journal repeatedly",
        then: "ReplayPolicyBlocked is emitted and the scheduled side-effect count is unchanged",
        public_surface: "vb_runtime recovery API",
        fixture: "isolated durable action journal fixture",
        expected_outcome: Some("ReplayPolicyBlocked normalized digest emitted"),
        expected_error: Some("ReplayPolicyBlocked"),
        durability_profile: "strict durable action journal evidence",
        related_bead: "vb-kyyf",
        executable_evidence_target: Some(".evidence/vb-kyyf/non-replay-safe-actions.md"),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "BDD-KYYF-004",
        master_behavior: "corrupt replay evidence fails deterministically",
        given: "gapped, duplicate, out-of-order, corrupt, and digest-mismatched journal evidence",
        when: "public storage and recovery surfaces replay each fixture",
        then: "ReplayDigestMismatch or ReplaySequenceViolation is returned without silent continuation",
        public_surface: "vb_storage journal and recovery APIs",
        fixture: "isolated corrupt replay evidence fixture",
        expected_outcome: Some("ReplayDigestMismatch mismatch emitted"),
        expected_error: Some("ReplayDigestMismatch ReplaySequenceViolation"),
        durability_profile: "strict durable journal evidence",
        related_bead: "vb-kyyf",
        executable_evidence_target: Some(".evidence/vb-kyyf/recovery-bdd-errors.md"),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "BDD-KYYF-005",
        master_behavior: "generated mode and IR mode are observationally equivalent for supported workflows",
        given: "a workflow accepted by validate_generated_subset",
        when: "IR mode durable replay and generated Rust execution/replay are compared",
        then: "generated replay parity digest matches terminal result, taint, journal signature, and errors",
        public_surface: "vb_codegen and vb_runtime public surfaces",
        fixture: "isolated generated-vs-IR durable fixture",
        expected_outcome: Some("generated replay parity digest emitted"),
        expected_error: Some("GeneratedIrDivergence mismatch"),
        durability_profile: "strict durable IR plus generated replay evidence",
        related_bead: "vb-kyyf",
        executable_evidence_target: Some(".evidence/vb-kyyf/generated-ir-parity.md"),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "BDD-KYYF-006",
        master_behavior: "unsupported generated subset fails closed",
        given: "a workflow containing an unsupported generated-mode IR family",
        when: "generated subset validation is requested",
        then: "UnsupportedGeneratedSubset is emitted and no parity credit is granted",
        public_surface: "vb_codegen generated-subset validation API",
        fixture: "isolated unsupported generated subset fixture",
        expected_outcome: Some("UnsupportedGeneratedSubset mismatch emitted"),
        expected_error: Some("UnsupportedGeneratedSubset"),
        durability_profile: "no persistent runtime state",
        related_bead: "vb-kyyf",
        executable_evidence_target: Some(".evidence/vb-kyyf/generated-subset-fail-closed.md"),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "BDD-KYYF-007",
        master_behavior: "vb-kyyf scenario evidence is traceable and strong",
        given: "the release acceptance catalog contains the vb-kyyf group",
        when: "the runner validates each scenario row",
        then: "scenario id, Given/When/Then, public surface, mismatch or normalized digest, and evidence path are present",
        public_surface: "velvet_ballastics_workspace_tests::acceptance_catalog",
        fixture: "isolated vb-kyyf catalog fixture",
        expected_outcome: Some("normalized digest evidence path emitted"),
        expected_error: Some("EvidenceArtifactMissing mismatch"),
        durability_profile: "catalog evidence only",
        related_bead: "vb-kyyf",
        executable_evidence_target: Some(".evidence/vb-kyyf/acceptance-catalog-traceability.md"),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "VB-BDD-CATALOG-001",
        master_behavior: "canonical naming and workspace spelling gates",
        given: "a repository checkout with canonical package, crate, bead, and language names",
        when: "the acceptance catalog audits naming coverage",
        then: "the catalog points to executable spelling tests and rejects drift",
        public_surface: "workspace cargo package names and test binaries",
        fixture: "isolated checkout fixture with canonical and misspelled names",
        expected_outcome: Some("canonical spelling accepted"),
        expected_error: Some("canonical_spelling_violation"),
        durability_profile: "no persistent runtime state",
        related_bead: "vb-37lc",
        executable_evidence_target: Some(
            "crates/workspace_tests/tests/vb_37lc_canonical_spelling_red.rs",
        ),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "VB-BDD-CATALOG-002",
        master_behavior: "validation gates reject malformed workflow parts",
        given: "compiled workflow parts with bounded slots, nodes, accessors, and contracts",
        when: "the validation public API runs all gates",
        then: "exact typed validation errors identify the violated gate",
        public_surface: "vb_validate::shared::validate_with_contracts",
        fixture: "isolated in-memory workflow fixtures",
        expected_outcome: Some("valid workflow accepted"),
        expected_error: Some("ValidationError gate variant"),
        durability_profile: "no persistent runtime state",
        related_bead: "vb-qi37.8",
        executable_evidence_target: Some("crates/workspace_tests/tests/bdd_validation_tests.rs"),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "VB-BDD-CATALOG-003",
        master_behavior: "YAML strict admission emits accepted artifacts before execution",
        given: "a strict YAML workflow and accepted artifact requirement",
        when: "the compile/admission path validates the artifact",
        then: "accepted artifacts run and rejected artifacts return exact diagnostics",
        public_surface: "vb_compile tests and runtime admission APIs",
        fixture: "isolated YAML workflow fixture",
        expected_outcome: Some("accepted artifact certificate emitted"),
        expected_error: Some("AdmissionRequired"),
        durability_profile: "artifact file persisted before runtime admission",
        related_bead: "vb-core-yaml-e2e-chain",
        executable_evidence_target: Some(
            "crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs",
        ),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "VB-BDD-CATALOG-004",
        master_behavior: "runtime direct API exposes submit, inspect, cancel, trace, and shutdown",
        given: "a direct Rust API client and isolated runtime state",
        when: "the client drives a run through public runtime functions",
        then: "the run result, taint, events, and typed errors match the scenario",
        public_surface: "vb_runtime public API",
        fixture: "isolated runtime fixture per scenario",
        expected_outcome: Some("run reaches expected terminal state"),
        expected_error: Some("RuntimeError variant"),
        durability_profile: "journal events captured when storage is enabled",
        related_bead: "vb-vt2f",
        executable_evidence_target: Some(
            "crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs",
        ),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "VB-BDD-CATALOG-005",
        master_behavior: "binary IPC rejects malformed frames before payload allocation",
        given: "a public IPC client with bounded frame fixtures",
        when: "the client submits valid and malformed frames",
        then: "responses preserve correlation ids and exact typed errors",
        public_surface: "vb_ipc public frame boundary",
        fixture: "isolated IPC frame fixture per scenario",
        expected_outcome: Some("valid frame round-trips"),
        expected_error: Some("bad_magic"),
        durability_profile: "no storage unless command triggers a run",
        related_bead: "vb-te1i",
        executable_evidence_target: None,
        deferred_follow_up_bead: Some("vb-te1i"),
    },
    Scenario {
        id: "VB-BDD-CATALOG-006",
        master_behavior: "storage recovery preserves journal evidence and rejects corrupt records",
        given: "an isolated Fjall-backed journal with crash-point fixtures",
        when: "recovery replays records and injected corruptions",
        then: "valid records hydrate state and corrupt records return exact errors",
        public_surface: "vb_storage journal and recovery APIs",
        fixture: "isolated temporary storage directory per scenario",
        expected_outcome: Some("journal replay summary matches expected state"),
        expected_error: Some("record codec/recovery error variant"),
        durability_profile: "persistent journal and snapshot evidence",
        related_bead: "vb-rpch",
        executable_evidence_target: None,
        deferred_follow_up_bead: Some("vb-rpch"),
    },
    Scenario {
        id: "VB-BDD-CATALOG-007",
        master_behavior: "generated Rust remains semantically equivalent to IR mode",
        given: "a workflow accepted for generated and IR execution",
        when: "both execution modes run the same fixture",
        then: "results, taint, journal events, and typed errors are identical",
        public_surface: "vb_codegen and vb_runtime public surfaces",
        fixture: "isolated generated-vs-IR fixture",
        expected_outcome: Some("mode parity evidence emitted"),
        expected_error: Some("parity_mismatch"),
        durability_profile: "journal evidence when workflow has durable effects",
        related_bead: "vb-0sps",
        executable_evidence_target: None,
        deferred_follow_up_bead: Some("vb-0sps"),
    },
    Scenario {
        id: "VB-BDD-CATALOG-008",
        master_behavior: "capability admission fails closed for missing action grants",
        given: "a workflow action requiring an explicit capability",
        when: "admission runs with and without the grant",
        then: "missing grants are rejected and granted runs are accepted",
        public_surface: "vb_runtime admission APIs",
        fixture: "isolated capability contract fixture",
        expected_outcome: Some("granted capability accepted"),
        expected_error: Some("CapabilityDenied"),
        durability_profile: "admission certificate records required grants",
        related_bead: "vb-ssei",
        executable_evidence_target: Some(
            "crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs",
        ),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "VB-BDD-CATALOG-009",
        master_behavior: "resource bounds and hot-path budgets fail before unbounded allocation",
        given: "budgeted runtime, IPC, and storage fixtures",
        when: "a scenario exceeds slots, buffers, queue capacity, or payload limits",
        then: "the public surface returns exact budget diagnostics",
        public_surface: "runtime, IPC, and storage admission APIs",
        fixture: "isolated bounded-resource fixture",
        expected_outcome: Some("within-budget run accepted"),
        expected_error: Some("budget_exceeded"),
        durability_profile: "pre-admission only unless accepted",
        related_bead: "vb-e4mt",
        executable_evidence_target: Some(
            "crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs",
        ),
        deferred_follow_up_bead: None,
    },
    Scenario {
        id: "VB-BDD-CATALOG-010",
        master_behavior: "test evidence must be executable and assertion-strong",
        given: "Rust tests containing table loops and scenario labels",
        when: "the quality inventory scans test files",
        then: "weak prose-only or unlabeled scenarios are rejected",
        public_surface: "velvet_ballastics_workspace_tests::quality::test_loop_inventory",
        fixture: "isolated source-text fixture per scan",
        expected_outcome: Some("strongly labeled scenario accepted"),
        expected_error: Some("AmbiguousCaseLabel"),
        durability_profile: "no persistent runtime state",
        related_bead: "vb-5xs4",
        executable_evidence_target: Some(
            "crates/workspace_tests/tests/vb_5xs4_test_loop_inventory_red.rs",
        ),
        deferred_follow_up_bead: None,
    },
];

pub fn catalog() -> &'static [Scenario] {
    SCENARIOS
}

pub fn validate_catalog(scenarios: &[Scenario]) -> Result<(), CatalogValidationError> {
    if scenarios.is_empty() {
        return Err(CatalogValidationError::EmptyCatalog);
    }

    let mut seen = BTreeSet::new();
    for scenario in scenarios {
        validate_scenario(*scenario, &mut seen)?;
    }

    Ok(())
}

fn validate_scenario(
    scenario: Scenario,
    seen: &mut BTreeSet<&'static str>,
) -> Result<(), CatalogValidationError> {
    if scenario.given.is_empty() || scenario.when.is_empty() || scenario.then.is_empty() {
        return Err(CatalogValidationError::MissingGivenWhenThen {
            scenario_id: scenario.id.to_owned(),
        });
    }

    if scenario.expected_outcome.is_none() && scenario.expected_error.is_none() {
        return Err(CatalogValidationError::MissingExactAssertion {
            scenario_id: scenario.id.to_owned(),
        });
    }

    match (
        scenario.executable_evidence_target,
        scenario.deferred_follow_up_bead,
    ) {
        (None, None) => {
            return Err(CatalogValidationError::MissingEvidenceDisposition {
                scenario_id: scenario.id.to_owned(),
            });
        }
        (Some(_), Some(_)) => {
            return Err(CatalogValidationError::ConflictingEvidenceDisposition {
                scenario_id: scenario.id.to_owned(),
            });
        }
        (Some(target), None) => validate_executable_target(scenario.id, target)?,
        (None, Some(bead)) => validate_deferred_follow_up(scenario, bead)?,
    }

    if scenario.public_surface.contains("private") || scenario.public_surface.contains("helper") {
        return Err(CatalogValidationError::PrivateSurface {
            scenario_id: scenario.id.to_owned(),
        });
    }

    if !scenario.fixture.contains("isolated") {
        return Err(CatalogValidationError::SharedFixture {
            scenario_id: scenario.id.to_owned(),
        });
    }

    if !seen.insert(scenario.id) {
        return Err(CatalogValidationError::DuplicateScenarioId {
            scenario_id: scenario.id.to_owned(),
        });
    }

    Ok(())
}

fn validate_executable_target(
    scenario_id: &'static str,
    target: &'static str,
) -> Result<(), CatalogValidationError> {
    let rust_test_target =
        target.starts_with("crates/workspace_tests/tests/") && target.ends_with(".rs");
    let evidence_target = target.starts_with(".evidence/") && target.ends_with(".md");
    if rust_test_target || evidence_target {
        Ok(())
    } else {
        Err(CatalogValidationError::InvalidExecutableEvidenceTarget {
            scenario_id: scenario_id.to_owned(),
        })
    }
}

fn validate_deferred_follow_up(
    scenario: Scenario,
    bead: &'static str,
) -> Result<(), CatalogValidationError> {
    if bead.starts_with("vb-") && bead == scenario.related_bead {
        Ok(())
    } else {
        Err(CatalogValidationError::InvalidDeferredFollowUpBead {
            scenario_id: scenario.id.to_owned(),
        })
    }
}
