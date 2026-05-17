use velvet_ballastics_workspace_tests::quality::current_api_mutation_plan::{
    REQUIRED_SECTIONS, validate_plan,
};

const PLAN: &str = include_str!("../../../docs/current-api-mutation-plan.md");

#[test]
fn current_helper_semantics_have_mutation_targets() {
    let report = validate_plan(PLAN);
    assert_eq!(
        report.missing_requirements,
        Vec::<&'static str>::new(),
        "mutation plan must name every current helper semantic and target category"
    );
    assert_eq!(
        report.section_count,
        REQUIRED_SECTIONS.len(),
        "validation must cover every required mutation-plan section"
    );
}

#[test]
fn runtime_recovery_has_mutation_targets() {
    let recovery_section = REQUIRED_SECTIONS
        .iter()
        .find(|section| section.id == "runtime-recovery");
    assert!(
        recovery_section.is_some(),
        "runtime recovery section must be part of the required plan contract"
    );
    assert!(
        PLAN.contains("ActionCompleted before frame mutation")
            && PLAN.contains("journal sequence hydration")
            && PLAN.contains("snapshot hydration"),
        "runtime recovery mutation targets must cover ordering and hydration semantics"
    );
}

#[test]
fn stale_api_target_fails_plan_validation() {
    let stale_plan =
        "# Current API Mutation Plan\n## Helper Semantics Mutation Targets\ngeneric DAG runner";
    let report = validate_plan(stale_plan);
    assert!(
        !report.is_valid(),
        "plan validation must fail stale product/API descriptions"
    );
    assert_eq!(
        report.stale_api_mentions, 1,
        "stale API marker count must identify the invalid target"
    );
}

#[test]
fn critical_survivor_creates_blocker() {
    assert!(
        PLAN.contains("Critical semantic survivor policy")
            && PLAN.contains("BLOCK_LOCAL")
            && PLAN.contains("bd create"),
        "critical mutation survivors must become explicit blocker evidence or follow-up beads"
    );
}
