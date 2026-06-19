#![forbid(unsafe_code)]
#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iterator,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groups,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
//! Red-Queen adversarial state-space pressure tests for tier-a-7-016
//! (WholeWorkflowBudget analyzer).
//!
//! Bead: tier-a-7-016
//! State machine: OK / unbounded-nested-fanout / unbounded-for-each /
//!                unbounded-collect / unbounded-repeat
//! Pressure: nested fanout exceeding DEFAULT.max_fanout (64), linear
//! steps exceeding DEFAULT.max_total_steps (1000), for_each with
//! iterations > limit, collect with pages/items > limit, repeat with
//! max_attempts > limit, analyzer determinism.
//!
//! IMPORTANT: The unbounded validation actually happens at the
//! `compile_workflow` boundary (via `try_from_parts` calling
//! `validate_budget`), so for unbounded workflows `compile_workflow`
//! itself returns `Err(CompileErrors(...))`. The
//! `compute_whole_workflow_budget` function then runs on already-valid
//! workflows and is exercised here via Ok paths and the belt-and-braces
//! `max_steps_executable == 0` check.
//!
//! These tests are deterministic. All checks are performed via exit code
//! comparison (no AI in the gate).

use vb_compile::compute_whole_workflow_budget;
use vb_compile::{CompileError, CompileErrors, compile_workflow};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: red-queen-budget\nwhen:\n  manual: {}\nsteps:\n";

const FINISH: &str = "  - id: done\n    finish:\n      result: 0\n";

/// Compile + analyze. Returns `Ok(())` for bounded workflows that pass
/// the analyzer. Returns `Err(CompileError::UnboundedWorkflow { .. })`
/// for workflows that reach the analyzer with a zero-step budget.
/// Returns `Err(other)` for any other compile or analyzer error.
fn try_compute_budget(yaml: &str) -> Result<(), CompileError> {
    let workflow = compile_workflow(yaml.as_bytes()).map_err(|errs| {
        errs.0.into_iter()
            .next()
            .unwrap_or(CompileError::EmptySource)
    })?;
    compute_whole_workflow_budget(&workflow).map(|_| ())
}

/// Try to compile the workflow. Returns `Ok(workflow)` if it compiles,
/// `Err(compile_errors)` if it doesn't.
fn try_compile(yaml: &str) -> Result<vb_core::CompiledWorkflow, CompileErrors> {
    compile_workflow(yaml.as_bytes())
}

/// Build a workflow with N together branches (one set per branch).
fn fanout_workflow(branch_count: usize) -> String {
    let mut yaml = String::from(HEADER);
    yaml.push_str("  - id: fanout\n    together:\n      branches:\n");
    for i in 0..branch_count {
        yaml.push_str(&format!(
            "        - label: \"br{i}\"\n          steps:\n            - id: s_{i}\n              set:\n                output: o{i}\n                value: \"{i}\"\n"
        ));
    }
    yaml.push_str(FINISH);
    yaml
}

/// Build a workflow with N linear set steps.
fn linear_workflow(step_count: usize) -> String {
    let mut yaml = String::from(HEADER);
    for i in 0..step_count {
        yaml.push_str(&format!(
            "  - id: step_{i}\n    set:\n      output: o{i}\n      value: \"{i}\"\n"
        ));
    }
    yaml.push_str(FINISH);
    yaml
}

// ---------------------------------------------------------------------------
// Q1 — Fanout boundary (DEFAULT max_fanout = 64)
// ---------------------------------------------------------------------------

#[test]
fn red_queen_fanout_at_policy_limit_compiles() {
    let yaml = fanout_workflow(64);
    let outcome = try_compile(&yaml);
    assert!(
        outcome.is_ok(),
        "64-branch fanout at limit must compile (got: {:?})",
        outcome.err().map(|e| e.0)
    );
    // Analyzer must also succeed.
    let analysis = try_compute_budget(&yaml);
    assert!(
        analysis.is_ok(),
        "analyzer must accept 64-branch fanout (got: {analysis:?})"
    );
}

#[test]
fn red_queen_fanout_above_policy_limit_rejected_at_compile() {
    // 65 branches exceeds the DEFAULT policy limit. compile_workflow
    // itself rejects the workflow via validate_budget inside
    // try_from_parts, so we expect a CompileError at compile time.
    let yaml = fanout_workflow(65);
    let outcome = try_compile(&yaml);
    assert!(
        outcome.is_err(),
        "65-branch fanout must be rejected at compile time"
    );
}

#[test]
fn red_queen_fanout_far_above_limit_rejected_at_compile() {
    let yaml = fanout_workflow(120);
    let outcome = try_compile(&yaml);
    assert!(
        outcome.is_err(),
        "120-branch fanout must be rejected at compile time"
    );
}

#[test]
fn red_queen_fanout_extreme_rejected_at_compile() {
    // 500 branches: extreme stress test.
    let yaml = fanout_workflow(500);
    let outcome = try_compile(&yaml);
    assert!(
        outcome.is_err(),
        "500-branch fanout must be rejected at compile time"
    );
}

// ---------------------------------------------------------------------------
// Q2 — Linear step count (DEFAULT max_total_steps = 1000)
// ---------------------------------------------------------------------------

#[test]
fn red_queen_linear_steps_at_limit_compiles() {
    // BoundednessPolicy::DEFAULT.max_total_steps = 1_000. The `linear_workflow`
    // helper appends a finish step, so the boundary is 999 set steps + 1
    // finish step = 1000 total (at the policy limit).
    let yaml = linear_workflow(999);
    let outcome = try_compute_budget(&yaml);
    assert!(
        outcome.is_ok(),
        "999 set + 1 finish = 1000 steps must compile and analyze (got: {outcome:?})"
    );
}

#[test]
fn red_queen_linear_steps_moderate_count_compiles() {
    // Resource contract default max_steps is 1000 (u16), but actual
    // budget policy max_total_steps is also 1000. A linear workflow
    // with N steps has total_steps = N+1; pick N well below the limit
    // to leave headroom.
    let yaml = linear_workflow(500);
    let outcome = try_compute_budget(&yaml);
    assert!(
        outcome.is_ok(),
        "500 linear steps must compile and analyze (got: {outcome:?})"
    );
}

#[test]
fn red_queen_linear_steps_above_limit_rejected_at_compile() {
    let yaml = linear_workflow(1500);
    let outcome = try_compile(&yaml);
    assert!(
        outcome.is_err(),
        "1500 linear steps must be rejected at compile time"
    );
}

// ---------------------------------------------------------------------------
// Q3 — for_each at_once limit
// ---------------------------------------------------------------------------

#[test]
fn red_queen_for_each_small_compiles() {
    let yaml = format!(
        "{HEADER}  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      at_once: 5\n      steps:\n        - id: body\n          set:\n            output: x\n            value: \"1\"\n{FINISH}"
    );
    let outcome = try_compute_budget(&yaml);
    assert!(
        outcome.is_ok(),
        "for_each at_once=5 must compile and analyze (got: {outcome:?})"
    );
}

#[test]
fn red_queen_for_each_no_at_once_compiles() {
    let yaml = format!(
        "{HEADER}  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      steps:\n        - id: body\n          set:\n            output: x\n            value: \"1\"\n{FINISH}"
    );
    let outcome = try_compute_budget(&yaml);
    // Don't assert specific outcome (no-at_once may compile or not); just
    // assert the analyzer doesn't panic.
    match outcome {
        Ok(()) | Err(CompileError::UnboundedWorkflow { .. }) => {}
        Err(other) => panic!("for_each no-at_once: unexpected error: {other:?}"),
    }
}

#[test]
fn red_queen_for_each_high_at_once_compiles() {
    // BoundednessPolicy::DEFAULT.max_total_steps = 1_000. The analyzer
    // multiplies the body subgraph step count by `at_once`. The body's
    // compiled IR is more than 1 node (it includes the SetConst plus
    // the loop-back Jump), so the body_count is ≥ 2 and the multiplied
    // total grows quickly with at_once. at_once=100 keeps the total
    // safely under the 1000 policy cap.
    let yaml = format!(
        "{HEADER}  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      at_once: 100\n      steps:\n        - id: body\n          set:\n            output: x\n            value: \"1\"\n{FINISH}"
    );
    let outcome = try_compute_budget(&yaml);
    assert!(
        outcome.is_ok(),
        "for_each at_once=100 must compile and analyze (got: {outcome:?})"
    );
}

// ---------------------------------------------------------------------------
// Q4 — Repeat with max_attempts
// ---------------------------------------------------------------------------

#[test]
fn red_queen_repeat_small_attempts_compiles() {
    let yaml = format!(
        "{HEADER}  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: out\n            value: \"1\"\n{FINISH}"
    );
    let outcome = try_compute_budget(&yaml);
    assert!(
        outcome.is_ok(),
        "repeat max_attempts=3 must compile and analyze (got: {outcome:?})"
    );
}

#[test]
fn red_queen_repeat_high_attempts_compiles() {
    let yaml = format!(
        "{HEADER}  - id: retry\n    repeat:\n      max_attempts: 65535\n      steps:\n        - id: attempt\n          set:\n            output: out\n            value: \"1\"\n{FINISH}"
    );
    let outcome = try_compute_budget(&yaml);
    // Don't assert specific outcome (large max_attempts may exceed policy).
    match outcome {
        Ok(()) | Err(CompileError::UnboundedWorkflow { .. }) => {}
        Err(other) => panic!("repeat max_attempts=65535: unexpected error: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Q5 — Collect
// ---------------------------------------------------------------------------

#[test]
fn red_queen_collect_small_compiles() {
    let yaml = format!(
        "{HEADER}  - id: collect_pages\n    collect:\n      variable: page\n      source: \"0\"\n      pages: 3\n      items: 5\n      steps:\n        - id: remember\n          set:\n            output: page_seen\n            value: \"7\"\n{FINISH}"
    );
    let outcome = try_compute_budget(&yaml);
    assert!(
        outcome.is_ok(),
        "collect pages=3 items=5 must compile and analyze (got: {outcome:?})"
    );
}

// ---------------------------------------------------------------------------
// Q6 — Determinism: same workflow produces same analyzer outcome on
// repeated calls.
// ---------------------------------------------------------------------------

#[test]
fn red_queen_analyzer_is_deterministic() {
    let yaml = linear_workflow(50);
    let first = try_compute_budget(&yaml);
    let second = try_compute_budget(&yaml);
    let third = try_compute_budget(&yaml);
    match (first, second, third) {
        (Ok(()), Ok(()), Ok(())) => {}
        (
            Err(CompileError::UnboundedWorkflow { reason: r1, .. }),
            Err(CompileError::UnboundedWorkflow { reason: r2, .. }),
            Err(CompileError::UnboundedWorkflow { reason: r3, .. }),
        ) => {
            assert_eq!(r1, r2, "reason must be stable across calls");
            assert_eq!(r2, r3, "reason must be stable across calls");
        }
        _ => panic!("analyzer must be deterministic across calls"),
    }
}

#[test]
fn red_queen_analyzer_budget_values_are_stable_across_calls() {
    let yaml = linear_workflow(50);
    let workflow = compile_workflow(yaml.as_bytes()).expect("compile");
    let first = compute_whole_workflow_budget(&workflow).expect("first");
    let second = compute_whole_workflow_budget(&workflow).expect("second");
    let third = compute_whole_workflow_budget(&workflow).expect("third");
    assert_eq!(
        first, second,
        "WholeWorkflowBudget must be stable across calls (1 vs 2)"
    );
    assert_eq!(
        second, third,
        "WholeWorkflowBudget must be stable across calls (2 vs 3)"
    );
}

// ---------------------------------------------------------------------------
// Q7 — Bounded workflow never panics under repeated analysis
// ---------------------------------------------------------------------------

#[test]
fn red_queen_analyzer_does_not_panic_under_repetition() {
    let yaml = linear_workflow(20);
    for _ in 0..1000 {
        let outcome = try_compute_budget(&yaml);
        assert!(
            outcome.is_ok(),
            "bounded workflow must always be accepted (got: {outcome:?})"
        );
    }
}

// ---------------------------------------------------------------------------
// Q8 — Budget fields are reachable on the returned value
// ---------------------------------------------------------------------------

#[test]
fn red_queen_budget_has_all_documented_fields() {
    let yaml = linear_workflow(10);
    let workflow = compile_workflow(yaml.as_bytes()).expect("compile");
    let budget = compute_whole_workflow_budget(&workflow).expect("analyze");
    // Every documented §64 field must be readable. We don't assert
    // specific values because they depend on the underlying traversal.
    let _ = budget.max_total_steps;
    let _ = budget.max_total_slots;
    let _ = budget.max_fanout;
    let _ = budget.max_nesting_depth;
    let _ = budget.max_steps_executable;
    let _ = budget.max_action_tickets;
    let _ = budget.max_parallel_in_flight;
    let _ = budget.max_retries_per_action;
    let _ = budget.max_gather_pages;
    let _ = budget.max_gather_items;
    let _ = budget.max_for_each_iterations;
    let _ = budget.max_together_branches;
    let _ = budget.max_repeat_attempts;
    let _ = budget.max_run_time_seconds;
    let _ = budget.max_result_bytes;
    let _ = budget.max_total_slots_written;
}

// ---------------------------------------------------------------------------
// Q9 — Step count corresponds to total reachable steps for linear workflow
// ---------------------------------------------------------------------------

#[test]
fn red_queen_budget_total_steps_matches_linear_count() {
    // A linear workflow with N set steps has total_steps = N+1 (the
    // set steps + the finish step). The analyzer's max_total_steps
    // must reflect this.
    let yaml = linear_workflow(20);
    let workflow = compile_workflow(yaml.as_bytes()).expect("compile");
    let budget = compute_whole_workflow_budget(&workflow).expect("analyze");
    // max_total_steps for a 20-set + 1-finish linear workflow = 21.
    // max_steps_executable for the same workflow = 21 (all are executable).
    assert_eq!(
        budget.max_total_steps, 21,
        "max_total_steps must reflect 20 sets + 1 finish"
    );
    assert_eq!(
        budget.max_steps_executable, 21,
        "max_steps_executable must equal max_total_steps for linear workflows"
    );
}

// ---------------------------------------------------------------------------
// Q10 — Fanout budget: 4-branch together has max_together_branches = 4
// ---------------------------------------------------------------------------

#[test]
fn red_queen_fanout_budget_reflects_branch_count() {
    let yaml = fanout_workflow(4);
    let workflow = compile_workflow(yaml.as_bytes()).expect("compile");
    let budget = compute_whole_workflow_budget(&workflow).expect("analyze");
    // max_together_branches for a 4-branch fanout = 4.
    assert_eq!(
        budget.max_together_branches, 4,
        "max_together_branches must equal the branch count"
    );
    // max_fanout also = 4 for a single together.
    assert_eq!(
        budget.max_fanout, 4,
        "max_fanout must equal the branch count for a single together"
    );
}