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

mod scenarios;

use scenarios::SCENARIOS;

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

// ──────────────────────────────────────────────────────────────────────────────
// OBL-CAT-001 through OBL-CAT-004: unit tests for catalog validation
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_scenario(id: &'static str) -> Scenario {
        Scenario {
            id,
            master_behavior: "test behavior",
            given: "a scenario",
            when: "the catalog gate validates it",
            then: "the scenario is accepted",
            public_surface: "workspace test boundary",
            fixture: "isolated fixture",
            expected_outcome: Some("accepted"),
            expected_error: None,
            durability_profile: "none",
            related_bead: "vb-hs9m",
            executable_evidence_target: Some("crates/workspace_tests/tests/test.rs"),
            deferred_follow_up_bead: None,
        }
    }

    // OBL-CAT-001: validate_catalog returns Ok for valid catalog.
    #[test]
    fn validate_catalog_valid() {
        let scenarios = &[valid_scenario("CAT-001"), valid_scenario("CAT-002")];
        assert_eq!(validate_catalog(scenarios), Ok(()));
    }

    // OBL-CAT-002: validate_catalog returns Err(DuplicateScenarioId) on duplicate ids.
    #[test]
    fn validate_catalog_duplicate_id() {
        let scenarios = &[valid_scenario("CAT-DUP"), valid_scenario("CAT-DUP")];
        assert_eq!(
            validate_catalog(scenarios),
            Err(CatalogValidationError::DuplicateScenarioId {
                scenario_id: "CAT-DUP".to_owned(),
            })
        );
    }

    // OBL-CAT-003: validate_catalog returns Err(MissingGivenWhenThen) for empty GWT.
    #[test]
    fn validate_catalog_missing_gwt() {
        // Missing given
        let mut s1 = valid_scenario("CAT-MISS-GIVEN");
        s1.given = "";
        assert_eq!(
            validate_catalog(&[s1]),
            Err(CatalogValidationError::MissingGivenWhenThen {
                scenario_id: "CAT-MISS-GIVEN".to_owned(),
            })
        );

        // Missing when
        let mut s2 = valid_scenario("CAT-MISS-WHEN");
        s2.when = "";
        assert_eq!(
            validate_catalog(&[s2]),
            Err(CatalogValidationError::MissingGivenWhenThen {
                scenario_id: "CAT-MISS-WHEN".to_owned(),
            })
        );

        // Missing then
        let mut s3 = valid_scenario("CAT-MISS-THEN");
        s3.then = "";
        assert_eq!(
            validate_catalog(&[s3]),
            Err(CatalogValidationError::MissingGivenWhenThen {
                scenario_id: "CAT-MISS-THEN".to_owned(),
            })
        );
    }

    // OBL-CAT-004: validate_catalog returns Err(MissingExactAssertion) when
    //              scenario has neither expected_outcome nor expected_error.
    #[test]
    fn validate_catalog_missing_assertion() {
        let mut scenario = valid_scenario("CAT-MISS-ASSERT");
        scenario.expected_outcome = None;
        scenario.expected_error = None;
        assert_eq!(
            validate_catalog(&[scenario]),
            Err(CatalogValidationError::MissingExactAssertion {
                scenario_id: "CAT-MISS-ASSERT".to_owned(),
            })
        );
    }
}
