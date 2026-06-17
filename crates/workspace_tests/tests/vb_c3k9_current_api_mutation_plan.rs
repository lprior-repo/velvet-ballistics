#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]

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

    let report = validate_plan(&misplaced_plan);

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

    let report = validate_plan(&missing_section_plan);

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
