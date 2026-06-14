use velvet_ballistics_workspace_tests::quality::current_api_mutation_plan::{
    MissingRequirement, REQUIRED_SECTIONS, validate_plan,
};

const PLAN: &str = include_str!("../../../docs/current-api-mutation-plan.md");
const REMOVED_UI_MODEL: &str = concat!("vb_ui_", "model");

#[test]
fn current_helper_semantics_have_mutation_targets() {
    let report = validate_plan(PLAN);
    assert_eq!(
        report.missing_requirements,
        Vec::<MissingRequirement>::new(),
        "mutation plan must name every current helper semantic and target category"
    );
    assert_eq!(
        report.required_section_count,
        REQUIRED_SECTIONS.len(),
        "validation must cover every required mutation-plan section"
    );
    assert_eq!(
        report.covered_required_sections,
        REQUIRED_SECTIONS.len(),
        "current plan must contain each required section exactly where validation can inspect it"
    );
    assert_eq!(
        report.missing_sections,
        Vec::<&'static str>::new(),
        "current plan must not omit a required section"
    );
    assert_eq!(
        report.duplicate_sections,
        Vec::<&'static str>::new(),
        "current plan must not duplicate required sections"
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
fn misplaced_required_term_fails_section_scoped_validation() {
    let misplaced_plan = format!(
        "# Current API Mutation Plan\n\
## Helper Semantics Mutation Targets\n\
contains\n\
starts_with\n\
ends_with\n\
length\n\
empty\n\
has\n\
exists\n\
sum\n\
count\n\
append_if\n\
merge\n\
unique\n\
ActionCompleted before frame mutation\n\
journal sequence hydration\n\
snapshot hydration\n\
retry state\n\
## Runtime Recovery Mutation Targets\n\
## Generated Rust Parity Mutation Targets\n\
generated-interpreter suspension parity\n\
full final IR equivalence\n\
unsupported generated-mode rejection\n\
## CLI, IPC, and Storage Envelope Mutation Targets\n\
binary IPC frame length\n\
postcard envelope\n\
Fjall journal\n\
CLI accepted artifact path\n\
## UI Model Contract Mutation Targets\n\
{}\n\
certificate\n\
incident\n\
replay\n\
## Owner Beads and Release Blockers\n\
owner bead\n\
critical survivor\n\
release-risk acceptance\n\
cargo mutants --package velvet-ballistics-workspace-tests --test vb_c3k9_current_api_mutation_plan\n\
90% mutation kill rate\n\
exclusion policy",
        REMOVED_UI_MODEL,
    );

    let report = validate_plan(misplaced_plan);

    assert_eq!(
        report.missing_requirements,
        vec![
            MissingRequirement {
                section_id: "runtime-recovery",
                term: "ActionCompleted before frame mutation",
            },
            MissingRequirement {
                section_id: "runtime-recovery",
                term: "journal sequence hydration",
            },
            MissingRequirement {
                section_id: "runtime-recovery",
                term: "snapshot hydration",
            },
            MissingRequirement {
                section_id: "runtime-recovery",
                term: "retry state",
            },
        ],
        "terms in the wrong section must not satisfy runtime recovery requirements"
    );
}

#[test]
fn missing_required_section_reports_actual_coverage() {
    let missing_section_plan = format!(
        "# Current API Mutation Plan\n\
## Helper Semantics Mutation Targets\n\
contains\n\
starts_with\n\
ends_with\n\
length\n\
empty\n\
has\n\
exists\n\
sum\n\
count\n\
append_if\n\
merge\n\
unique\n\
## Generated Rust Parity Mutation Targets\n\
generated-interpreter suspension parity\n\
full final IR equivalence\n\
unsupported generated-mode rejection\n\
## CLI, IPC, and Storage Envelope Mutation Targets\n\
binary IPC frame length\n\
postcard envelope\n\
Fjall journal\n\
CLI accepted artifact path\n\
## UI Model Contract Mutation Targets\n\
{}\n\
certificate\n\
incident\n\
replay\n\
## Owner Beads and Release Blockers\n\
owner bead\n\
critical survivor\n\
release-risk acceptance\n\
cargo mutants --package velvet-ballistics-workspace-tests --test vb_c3k9_current_api_mutation_plan\n\
90% mutation kill rate\n\
exclusion policy",
        REMOVED_UI_MODEL,
    );

    let report = validate_plan(missing_section_plan);

    assert_eq!(report.required_section_count, REQUIRED_SECTIONS.len());
    assert_eq!(
        report.covered_required_sections,
        REQUIRED_SECTIONS.len() - 1
    );
    assert_eq!(report.missing_sections, vec!["runtime-recovery"]);
    assert_eq!(
        report.missing_requirements,
        vec![
            MissingRequirement {
                section_id: "runtime-recovery",
                term: "ActionCompleted before frame mutation",
            },
            MissingRequirement {
                section_id: "runtime-recovery",
                term: "journal sequence hydration",
            },
            MissingRequirement {
                section_id: "runtime-recovery",
                term: "snapshot hydration",
            },
            MissingRequirement {
                section_id: "runtime-recovery",
                term: "retry state",
            },
        ]
    );
}

#[test]
fn duplicated_required_section_reports_exact_duplicate_id() {
    let duplicated_section_plan = format!(
        "{}\n{}\nduplicate body",
        PLAN, "## Runtime Recovery Mutation Targets"
    );

    let report = validate_plan(&duplicated_section_plan);

    assert_eq!(report.duplicate_sections, vec!["runtime-recovery"]);
    assert_eq!(report.covered_required_sections, REQUIRED_SECTIONS.len());
    assert_eq!(
        report.is_valid(),
        false,
        "duplicated section headings must fail validation instead of hiding misplaced terms"
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

#[test]
fn admission_branch_mutation_plan_rejects_unrelated_smoke_substitution() {
    assert!(
        PLAN.contains("Runtime admission branch")
            && PLAN.contains("test_mutation_gate_fails_when_admission_branch_removed")
            && PLAN.contains(
                "cargo mutants --package velvet-ballistics-workspace-tests --test vb_njju_mutation_fuzz_property_closure"
            ),
        "vb-njju admission-branch mutation plan must name exact scope, test, and scoped cargo-mutants command"
    );
    assert!(
        PLAN.contains("diagnostic.rs")
            && PLAN.contains("regression smoke only")
            && PLAN.contains("never satisfies admission-branch closure"),
        "unrelated diagnostic.rs smoke must be documented as insufficient for vb-njju admission closure"
    );
}
