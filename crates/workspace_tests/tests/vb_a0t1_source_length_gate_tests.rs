#[path = "vb_a0t1_source_length_gate/fixture_sources.rs"]
mod fixture_sources;
#[path = "vb_a0t1_source_length_gate/support.rs"]
mod support;

use std::fs;
use std::time::Duration;

#[test]
fn test_source_length_gate_passes_on_clean_tree() -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_clean_tree(fixture.path())?;
    support::finish_git_fixture(fixture.path())?;

    let envs = support::fixture_env(fixture.path());
    let output = support::run_gate(fixture.path(), &envs, fixture.path())?;

    assert_eq!(output.code, Some(0), "stderr:\n{}", output.stderr);
    assert_eq!(output.stdout, "", "unexpected stdout");
    assert_eq!(output.stderr, "", "unexpected stderr");
    assert!(
        output.elapsed < Duration::from_secs(60),
        "elapsed: {:?}",
        output.elapsed
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_source_length_gate_fails_on_long_function() -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_long_function_tree(fixture.path())?;
    support::finish_git_fixture(fixture.path())?;

    let envs = support::fixture_env(fixture.path());
    let output = support::run_gate(fixture.path(), &envs, fixture.path())?;
    let expected = "crates/vb_core/src/engine.rs:5 hot function has 30 logical lines (limit 25)";

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert_eq!(output.stdout, "", "unexpected stdout");
    assert!(
        output.stderr.contains(expected),
        "stderr:\n{}",
        output.stderr
    );
    assert_eq!(
        output.stderr.matches("hot function has").count(),
        1,
        "stderr:\n{}",
        output.stderr
    );
    assert_eq!(
        output.stderr.contains("crates/vb_core/src/tests.rs"),
        false,
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.elapsed < Duration::from_secs(60),
        "elapsed: {:?}",
        output.elapsed
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_source_length_gate_fails_on_pub_unsafe_long_function() -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_unsafe_long_function_tree(fixture.path())?;
    support::finish_git_fixture(fixture.path())?;

    let envs = support::fixture_env(fixture.path());
    let output = support::run_gate(fixture.path(), &envs, fixture.path())?;
    let expected = "crates/vb_core/src/engine.rs:5 hot function has 30 logical lines (limit 25)";

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert_eq!(output.stdout, "", "unexpected stdout");
    assert!(
        output.stderr.contains(expected),
        "pub unsafe fn over the hot-function budget must be reported as FunctionOverLimit; stderr:\n{}",
        output.stderr
    );
    assert_eq!(
        output.stderr.matches("hot function has").count(),
        1,
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.elapsed < Duration::from_secs(60),
        "elapsed: {:?}",
        output.elapsed
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_source_length_gate_fails_on_long_file() -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_long_file_tree(fixture.path())?;
    support::finish_git_fixture(fixture.path())?;

    let envs = support::fixture_env(fixture.path());
    let output = support::run_gate(fixture.path(), &envs, fixture.path())?;
    let expected =
        "crates/vb_core/src/long_file.rs has 350 physical lines (limit <=300) and no valid";

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert_eq!(output.stdout, "", "unexpected stdout");
    assert!(
        output.stderr.contains(expected),
        "stderr:\n{}",
        output.stderr
    );
    assert_eq!(
        output.stderr.matches("has 350 physical lines").count(),
        1,
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.elapsed < Duration::from_secs(60),
        "elapsed: {:?}",
        output.elapsed
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_full_source_length_pipeline() -> support::TestResult<()> {
    let pipeline_root = support::real_pipeline_root()?;
    let temp = tempfile::TempDir::new()?;
    let state_file = temp.path().join("quarterly-state.jsonl");
    fs::write(&state_file, support::QUARTERLY_2026_Q2)?;

    let envs = vec![(
        "SOURCE_LENGTH_QUARTERLY_STATE",
        state_file.clone().into_os_string(),
    )];
    let output = support::run_gate(&pipeline_root, &envs, temp.path())?;

    assert_eq!(output.code, Some(0), "stderr:\n{}", output.stderr);
    assert_eq!(output.stdout, "", "unexpected stdout");
    assert_eq!(output.stderr, "", "unexpected stderr");
    assert!(
        output.elapsed < Duration::from_secs(60),
        "elapsed: {:?}",
        output.elapsed
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    support::assert_state_file_is_valid_jsonl(&state_file)?;
    let rows = support::state_file_jsonl_row_count(&state_file)?;
    assert_eq!(rows, 1, "current 2026-Q2 baseline must be idempotent");
    Ok(())
}

#[test]
fn test_source_length_gate_uses_default_hot_function_budget_boundaries() -> support::TestResult<()>
{
    let pass_fixture = support::fixture_repo()?;
    support::write_hot_function_tree_with_logical_lines(pass_fixture.path(), 25)?;
    support::finish_git_fixture(pass_fixture.path())?;
    let pass_envs = support::fixture_env_without_budget_overrides(pass_fixture.path());
    let pass_output = support::run_gate(pass_fixture.path(), &pass_envs, pass_fixture.path())?;
    assert_eq!(pass_output.code, Some(0), "stderr:\n{}", pass_output.stderr);
    assert_eq!(
        pass_output.stderr, "",
        "25 logical lines must pass at default limit"
    );

    let fail_fixture = support::fixture_repo()?;
    support::write_hot_function_tree_with_logical_lines(fail_fixture.path(), 26)?;
    support::finish_git_fixture(fail_fixture.path())?;
    let fail_envs = support::fixture_env_without_budget_overrides(fail_fixture.path());
    let fail_output = support::run_gate(fail_fixture.path(), &fail_envs, fail_fixture.path())?;
    let expected = "crates/vb_core/src/engine.rs:5 hot function has 26 logical lines (limit 25)";
    assert_eq!(fail_output.code, Some(1), "stderr:\n{}", fail_output.stderr);
    assert!(
        fail_output.stderr.contains(expected),
        "stderr:\n{}",
        fail_output.stderr
    );
    support::assert_absent_runtime_failure_text(&fail_output.stderr);
    Ok(())
}

#[test]
fn test_source_length_gate_uses_default_file_budget_boundaries() -> support::TestResult<()> {
    let pass_fixture = support::fixture_repo()?;
    support::write_long_file_tree_with_lines(pass_fixture.path(), 300)?;
    support::finish_git_fixture(pass_fixture.path())?;
    let pass_envs = support::fixture_env_without_budget_overrides(pass_fixture.path());
    let pass_output = support::run_gate(pass_fixture.path(), &pass_envs, pass_fixture.path())?;
    assert_eq!(pass_output.code, Some(0), "stderr:\n{}", pass_output.stderr);
    assert_eq!(
        pass_output.stderr, "",
        "300 physical lines must pass at default limit"
    );

    let fail_fixture = support::fixture_repo()?;
    support::write_long_file_tree_with_lines(fail_fixture.path(), 301)?;
    support::finish_git_fixture(fail_fixture.path())?;
    let fail_envs = support::fixture_env_without_budget_overrides(fail_fixture.path());
    let fail_output = support::run_gate(fail_fixture.path(), &fail_envs, fail_fixture.path())?;
    let expected =
        "crates/vb_core/src/long_file.rs has 301 physical lines (limit <=300) and no valid";
    assert_eq!(fail_output.code, Some(1), "stderr:\n{}", fail_output.stderr);
    assert!(
        fail_output.stderr.contains(expected),
        "stderr:\n{}",
        fail_output.stderr
    );
    support::assert_absent_runtime_failure_text(&fail_output.stderr);
    Ok(())
}

#[test]
fn test_source_length_ledger_valid_exception_suppresses_over_limit_file() -> support::TestResult<()>
{
    let fixture = support::fixture_repo()?;
    support::write_long_file_tree(fixture.path())?;
    let ledger = support::source_ledger_text(&[
        "crates/vb_core/src/long_file.rs|owner|split-bead|remove|reason",
    ]);
    support::write_source_ledger(fixture.path(), &ledger)?;
    support::finish_git_fixture(fixture.path())?;

    let envs = support::fixture_env_without_budget_overrides(fixture.path());
    let output = support::run_gate(fixture.path(), &envs, fixture.path())?;

    assert_eq!(output.code, Some(0), "stderr:\n{}", output.stderr);
    assert_eq!(
        output.stderr.contains("has 350 physical lines"),
        false,
        "stderr:\n{}",
        output.stderr
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_source_length_ledger_rejects_stale_duplicate_malformed_and_invalid_path_rows(
) -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_long_file_tree(fixture.path())?;
    let ledger = support::source_ledger_text(&[
        "malformed-row",
        "../outside.rs|owner|split-bead|remove|reason",
        "crates/vb_core/src/lib.rs|owner|split-bead|remove|reason",
        "crates/vb_core/src/long_file.rs|owner|split-bead|remove|reason",
        "crates/vb_core/src/long_file.rs|owner|split-bead|remove|reason",
    ]);
    support::write_source_ledger(fixture.path(), &ledger)?;
    support::finish_git_fixture(fixture.path())?;

    let envs = support::fixture_env_without_budget_overrides(fixture.path());
    let output = support::run_gate(fixture.path(), &envs, fixture.path())?;
    let ledger_path = support::source_ledger_path(fixture.path());

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert!(
        output.stderr.contains(&format!(
            "{}:6 malformed row; expected <file_path>|<owner>|<split_bead>|<removal_plan>|<reason>",
            ledger_path.display()
        )),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.stderr.contains(&format!(
            "{}:7 invalid path; use a normalized repository-relative path",
            ledger_path.display()
        )),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.stderr.contains(&format!(
            "{}:8 stale exception for crates/vb_core/src/lib.rs with 4 physical lines (limit >300)",
            ledger_path.display()
        )),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.stderr.contains(&format!(
            "{}:10 duplicate exception for crates/vb_core/src/long_file.rs",
            ledger_path.display()
        )),
        "stderr:\n{}",
        output.stderr
    );
    assert_eq!(
        output
            .stderr
            .contains("crates/vb_core/src/long_file.rs has 350 physical lines"),
        false,
        "stderr:\n{}",
        output.stderr
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_quarterly_self_test_appends_current_quarter_when_count_does_not_increase(
) -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_dedup_source_exception_fixture(
        fixture.path(),
        &["split-or-retire-before-release"],
    )?;
    support::finish_git_fixture(fixture.path())?;
    let temp = tempfile::TempDir::new()?;
    let state_file = temp.path().join("quarterly-state.jsonl");
    let (current, previous) = support::current_and_previous_quarter_labels()?;
    fs::write(
        &state_file,
        support::quarterly_state_line(&previous, 2, "2026-01-01"),
    )?;

    let output = support::run_gate(
        fixture.path(),
        &support::quarterly_state_env(&state_file),
        temp.path(),
    )?;

    assert_eq!(output.code, Some(0), "stderr:\n{}", output.stderr);
    let rows = support::state_file_lines(&state_file)?;
    assert_eq!(rows.len(), 2, "state rows: {rows:?}");
    let expected_current_row =
        support::quarterly_state_line(&current, 1, &support::current_date()?)
            .trim_end()
            .to_string();
    assert_eq!(rows.get(1), Some(&expected_current_row));
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_quarterly_self_test_is_idempotent_when_current_quarter_already_recorded(
) -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_dedup_source_exception_fixture(
        fixture.path(),
        &["split-or-retire-before-release"],
    )?;
    support::finish_git_fixture(fixture.path())?;
    let temp = tempfile::TempDir::new()?;
    let state_file = temp.path().join("quarterly-state.jsonl");
    let (current, _previous) = support::current_and_previous_quarter_labels()?;
    let original = support::quarterly_state_line(&current, 1, &support::current_date()?);
    fs::write(&state_file, original.clone())?;

    let first = support::run_gate(
        fixture.path(),
        &support::quarterly_state_env(&state_file),
        temp.path(),
    )?;
    let second = support::run_gate(
        fixture.path(),
        &support::quarterly_state_env(&state_file),
        temp.path(),
    )?;

    assert_eq!(first.code, Some(0), "stderr:\n{}", first.stderr);
    assert_eq!(second.code, Some(0), "stderr:\n{}", second.stderr);
    assert_eq!(fs::read_to_string(&state_file)?, original);
    support::assert_absent_runtime_failure_text(&first.stderr);
    support::assert_absent_runtime_failure_text(&second.stderr);
    Ok(())
}

#[test]
fn test_quarterly_self_test_fails_before_append_when_current_count_exceeds_prior(
) -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_dedup_source_exception_fixture(
        fixture.path(),
        &[
            "split-or-retire-before-release",
            "split-or-retire-before-release",
        ],
    )?;
    support::finish_git_fixture(fixture.path())?;
    let temp = tempfile::TempDir::new()?;
    let state_file = temp.path().join("quarterly-state.jsonl");
    let (current, previous) = support::current_and_previous_quarter_labels()?;
    let original = support::quarterly_state_line(&previous, 1, "2026-01-01");
    fs::write(&state_file, original.clone())?;

    let output = support::run_gate(
        fixture.path(),
        &support::quarterly_state_env(&state_file),
        temp.path(),
    )?;

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert!(
        output
            .stderr
            .contains("DEDUP-11 split-or-retire-before-release quarterly self-test FAILED"),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output
            .stderr
            .contains(&format!("Current quarter: {current} with 2 rows")),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.stderr.contains(&format!(
            "  - quarter {previous} recorded 1 rows; current {current} has 2 (+1)"
        )),
        "stderr:\n{}",
        output.stderr
    );
    assert_eq!(
        fs::read_to_string(&state_file)?,
        original,
        "failure must not append a current-quarter row"
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_quarterly_self_test_fails_when_non_marker_exception_rows_grow() -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_dedup_source_exception_fixture(
        fixture.path(),
        &["split-after-current-batch-landing"],
    )?;
    support::finish_git_fixture(fixture.path())?;
    let temp = tempfile::TempDir::new()?;
    let state_file = temp.path().join("quarterly-state.jsonl");
    let (current, previous) = support::current_and_previous_quarter_labels()?;
    let original = support::quarterly_state_line(&previous, 0, "2026-01-01");
    fs::write(&state_file, original.clone())?;

    let output = support::run_gate(
        fixture.path(),
        &support::quarterly_state_env(&state_file),
        temp.path(),
    )?;

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert!(
        output
            .stderr
            .contains("DEDUP-11 split-or-retire-before-release quarterly self-test FAILED"),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output
            .stderr
            .contains(&format!("Current quarter: {current} with 1 rows")),
        "stderr:\n{}",
        output.stderr
    );
    assert_eq!(
        fs::read_to_string(&state_file)?,
        original,
        "failure must not append a current-quarter row"
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_quarterly_self_test_fails_when_current_quarter_count_grows_after_recording(
) -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_dedup_source_exception_fixture(
        fixture.path(),
        &[
            "split-or-retire-before-release",
            "split-or-retire-before-release",
        ],
    )?;
    support::finish_git_fixture(fixture.path())?;
    let temp = tempfile::TempDir::new()?;
    let state_file = temp.path().join("quarterly-state.jsonl");
    let (current, _previous) = support::current_and_previous_quarter_labels()?;
    let original = support::quarterly_state_line(&current, 1, &support::current_date()?);
    fs::write(&state_file, original.clone())?;

    let output = support::run_gate(
        fixture.path(),
        &support::quarterly_state_env(&state_file),
        temp.path(),
    )?;

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert!(
        output
            .stderr
            .contains("DEDUP-11 split-or-retire-before-release quarterly self-test FAILED"),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output
            .stderr
            .contains(&format!("Current quarter: {current} with 2 rows")),
        "stderr:\n{}",
        output.stderr
    );
    assert_eq!(
        fs::read_to_string(&state_file)?,
        original,
        "same-quarter growth failure must not rewrite or append state"
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_quarterly_self_test_does_not_count_comment_markers_as_exception_rows(
) -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_clean_tree(fixture.path())?;
    support::write_split_or_retire_marker_ledgers(fixture.path(), 1, 0)?;
    support::finish_git_fixture(fixture.path())?;
    let temp = tempfile::TempDir::new()?;
    let state_file = temp.path().join("quarterly-state.jsonl");
    let (current, previous) = support::current_and_previous_quarter_labels()?;
    fs::write(
        &state_file,
        support::quarterly_state_line(&previous, 0, "2026-01-01"),
    )?;

    let output = support::run_gate(
        fixture.path(),
        &support::quarterly_state_env(&state_file),
        temp.path(),
    )?;

    assert_eq!(output.code, Some(0), "stderr:\n{}", output.stderr);
    assert_eq!(output.stderr, "", "comments are not active exception rows");
    let rows = support::state_file_lines(&state_file)?;
    assert_eq!(rows.len(), 2, "state rows: {rows:?}");
    let expected_current_row =
        support::quarterly_state_line(&current, 0, &support::current_date()?)
            .trim_end()
            .to_string();
    assert_eq!(rows.get(1), Some(&expected_current_row));
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_source_length_gate_terminates_on_hostile_braces_and_unmatched_quotes(
) -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_adversarial_hot_tree(fixture.path())?;
    support::finish_git_fixture(fixture.path())?;

    let envs = support::fixture_env_without_budget_overrides(fixture.path());
    let output = support::run_gate(fixture.path(), &envs, fixture.path())?;
    let expected = "crates/vb_core/src/engine.rs:5 hot function has 30 logical lines (limit 25)";

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert!(
        output.stderr.contains(expected),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.elapsed < Duration::from_secs(60),
        "elapsed: {:?}",
        output.elapsed
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_source_length_gate_terminates_with_exact_error_on_non_utf8_source(
) -> support::TestResult<()> {
    let fixture = support::fixture_repo()?;
    support::write_non_utf8_tree(fixture.path())?;
    support::finish_git_fixture(fixture.path())?;

    let envs = support::fixture_env_without_budget_overrides(fixture.path());
    let output = support::run_gate(fixture.path(), &envs, fixture.path())?;

    assert_eq!(output.code, Some(1), "stderr:\n{}", output.stderr);
    assert!(
        output
            .stderr
            .contains("failed to read crates/vb_core/src/engine.rs:"),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.stderr.contains("stream did not contain valid UTF-8"),
        "stderr:\n{}",
        output.stderr
    );
    assert!(
        output.elapsed < Duration::from_secs(60),
        "elapsed: {:?}",
        output.elapsed
    );
    support::assert_absent_runtime_failure_text(&output.stderr);
    Ok(())
}

#[test]
fn test_moon_ci_wiring_runs_source_length_self_test_before_source_length_before_test(
) -> support::TestResult<()> {
    let moon = include_str!("../../../.moon.yml");
    let tasks = include_str!("../../../.moon/tasks/all.yml");

    assert!(tasks.contains("  source-length:\n    command: 'bash scripts/check-source-length.sh'\n    deps:\n      - 'source-length-self-test'"));
    assert!(tasks.contains(
        "  source-length-self-test:\n    command: 'bash scripts/check-source-length-tests.sh'"
    ));
    assert!(tasks.contains("      - 'scripts/check-source-length.sh'\n      - 'scripts/check-source-length.rs'\n      - 'scripts/check-source-length-tests.sh'"));
    assert!(
        support::line_index(moon, "  - 'source-length'")?
            < support::line_index(moon, "  - 'test'")?
    );
    Ok(())
}

#[test]
fn test_moon_ci_block_global_evidence_is_exact_when_canonical_ci_is_not_green(
) -> support::TestResult<()> {
    let implementation = include_str!("../../../.beads/tier-a-0-001/implementation.md");
    let command_results =
        include_str!("../../../.beads/tier-a-0-001/state-12-command-results.jsonl");

    assert!(
        implementation.contains("| `moon run :source-length` | PASS | `Tasks: 5 completed` |"),
        "source-length acceptance evidence is missing"
    );
    assert!(
        command_results.contains("\"command\":\"moon run :source-length\"")
            && command_results.contains("\"exit_status\":0"),
        "state-12 command ledger must record moon source-length exit 0"
    );
    assert!(
        implementation.contains("| `moon ci` | FAIL / TIMEOUT / BLOCK_GLOBAL |"),
        "moon ci must either be green or carry exact BLOCK_GLOBAL evidence"
    );
    assert!(
        implementation.contains("velvet-ballistics:miri")
            && implementation.contains("unsupported `statx` under Miri isolation"),
        "BLOCK_GLOBAL moon ci evidence must name the external miri blocker"
    );
    Ok(())
}

#[test]
fn test_out_of_scope_vb_cli_xtask_changes_are_routed_with_touched_package_evidence(
) -> support::TestResult<()> {
    let implementation = include_str!("../../../.beads/tier-a-0-001/implementation.md");

    for path in [
        "crates/vb_cli/src/deliver_sink.rs",
        "xtask/src/main.rs",
        "xtask/src/shell.rs",
    ] {
        assert!(
            implementation.contains(path),
            "implementation evidence must explicitly route out-of-scope touched file {path}"
        );
    }
    assert!(
        implementation.contains(
            "rtk cargo check -p velvet-ballistics -p xtask -p velvet-ballistics-workspace-tests --all-targets --all-features"
        ) && implementation.contains("| PASS | `cargo build (0 crates compiled)`"),
        "out-of-scope vb_cli/xtask routing must include cargo check evidence"
    );
    assert!(
        implementation.contains(
            "rtk cargo clippy -p velvet-ballistics -p xtask --lib --bins --examples --all-features"
        ) && implementation.contains("| PASS | `cargo clippy: No issues found`"),
        "out-of-scope vb_cli/xtask routing must include clippy evidence"
    );
    Ok(())
}

#[test]
fn test_shellcheck_evidence_covers_all_touched_shell_artifacts_with_pinned_image(
) -> support::TestResult<()> {
    let command_results =
        include_str!("../../../.beads/tier-a-0-001/state-12-command-results.jsonl");

    assert!(
        command_results
            .contains("koalaman/shellcheck:stable scripts/check-source-length.sh\",\"completed_at")
            && command_results.contains("\"exit_status\":0"),
        "main source-length shell script must have shellcheck exit 0 evidence"
    );
    assert!(
        command_results.contains(
            "koalaman/shellcheck:stable scripts/check-source-length-tests.sh\",\"completed_at"
        ),
        "source-length self-test shell artifact must also have shellcheck evidence"
    );
    assert!(
        command_results.contains(
            "docker image inspect --format '{{index .RepoDigests 0}} {{.Id}}' koalaman/shellcheck:stable"
        ) && command_results.contains("\"exit_status\":0"),
        "shellcheck evidence must capture the image digest/id, not only a floating tag"
    );
    Ok(())
}

#[test]
fn test_xtask_source_length_gate_resolves_to_check_source_length_script() -> support::TestResult<()>
{
    let gates = include_str!("../../../xtask/src/gates.rs");
    let command_arm = "Gate::SourceLength => &[\"bash\", \"scripts/check-source-length.sh\"]";
    let name_arm = "Gate::SourceLength => \"source-length\"";

    assert_eq!(
        gates.matches(command_arm).count(),
        1,
        "xtask source-length command arm changed"
    );
    assert_eq!(
        gates.matches(name_arm).count(),
        1,
        "xtask source-length name arm changed"
    );
    assert!(gates
        .contains("pub fn run_source_length_gate(bead_id: Option<&str>) -> Result<GateEvidence>"));
    Ok(())
}
