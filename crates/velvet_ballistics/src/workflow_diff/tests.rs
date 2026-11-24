use std::path::{Path, PathBuf};

use super::compare::Change;
use super::execute;
use super::report::DiffReport;
use super::text::{LineChange, diff_lines};

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

const BASE: &str = "version: velvet-ballistics/v1
name: demo
when: {manual: {}}
secrets: {token: MY_TOKEN}
steps:
  - id: seed
    set: {output: base, value: '1'}
  - id: act
    do: {action: '1', input: '0'}
    try_again: {max_attempts: 3, delay: 1s}
  - id: done
    finish: {result: '$base'}
";

pub(super) fn write(dir: &Path, name: &str, text: &str) -> Result<PathBuf, std::io::Error> {
    let path = dir.join(name);
    std::fs::write(&path, text)?;
    Ok(path)
}

pub(super) fn diff_texts(old: &str, new: &str) -> Result<DiffReport, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let old_path = write(dir.path(), "old.yaml", old)?;
    let new_path = write(dir.path(), "new.yaml", new)?;
    Ok(execute(&new_path, Some(&old_path), None)?)
}

pub(super) fn messages<'a>(report: &'a DiffReport, category: &str) -> Vec<&'a str> {
    report
        .changes
        .iter()
        .filter(|change| change.category == category)
        .map(|change| change.message.as_str())
        .collect()
}

#[test]
fn identical_workflows_have_no_semantic_changes() -> TestResult {
    let report = diff_texts(BASE, BASE)?;
    assert!(!report.has_semantic_changes(), "{:?}", report.changes);
    assert_eq!(report.old_digest, report.new_digest);
    assert_eq!(report.old_steps, 3);
    assert_eq!(report.new_steps, 3);
    assert_eq!(report.text.as_ref().map(|d| d.changes.len()), Some(0));
    Ok(())
}

#[test]
fn comment_only_edit_is_a_text_change_but_not_a_semantic_change() -> TestResult {
    let commented = format!("# reviewed\n{BASE}");
    let report = diff_texts(BASE, &commented)?;
    assert!(!report.has_semantic_changes(), "{:?}", report.changes);
    let text = report.text.ok_or("text diff expected")?;
    assert_eq!(
        text.changes,
        vec![LineChange {
            added: true,
            line: 1,
            text: "# reviewed".into()
        }]
    );
    Ok(())
}

#[test]
fn added_step_changes_step_count_steps_and_control_flow() -> TestResult {
    let extended = BASE.replace(
        "  - id: done",
        "  - id: extra\n    set: {output: more, value: '2'}\n  - id: done",
    );
    let report = diff_texts(BASE, &extended)?;
    assert!(report.has_semantic_changes());
    assert_eq!((report.old_steps, report.new_steps), (3, 4));
    assert_eq!(
        messages(&report, "step_count"),
        ["step count changed: 3 -> 4"]
    );
    assert_eq!(messages(&report, "steps"), ["step added: extra:set_const"]);
    assert_eq!(
        messages(&report, "control_flow"),
        [
            "edge removed: act:do -> done:finish [next]",
            "edge added: act:do -> extra:set_const [next]",
            "edge added: extra:set_const -> done:finish [next]",
        ]
    );
    Ok(())
}

#[test]
fn retry_policy_change_reports_old_and_new_policy() -> TestResult {
    let tuned = BASE.replace("max_attempts: 3, delay: 1s", "max_attempts: 2, delay: 2s");
    let report = diff_texts(BASE, &tuned)?;
    assert_eq!(
        messages(&report, "retry_policy"),
        ["step act:do changed: max_attempts=3 delay_ms=1000 -> max_attempts=2 delay_ms=2000"]
    );
    assert_eq!(report.changes.len(), 1, "{:?}", report.changes);
    Ok(())
}

#[test]
fn removing_retry_policy_is_reported_as_removed() -> TestResult {
    let bare = BASE.replace("    try_again: {max_attempts: 3, delay: 1s}\n", "");
    let report = diff_texts(BASE, &bare)?;
    assert_eq!(
        messages(&report, "retry_policy"),
        ["step act:do removed (was max_attempts=3 delay_ms=1000)"]
    );
    Ok(())
}

#[test]
fn secret_requirement_and_action_swap_are_separate_dimensions() -> TestResult {
    let swapped = BASE
        .replace("MY_TOKEN", "OTHER_TOKEN")
        .replace("action: '1'", "action: '3'")
        .replace("    try_again: {max_attempts: 3, delay: 1s}\n", "");
    let report = diff_texts(BASE, &swapped)?;
    assert_eq!(
        messages(&report, "secret_usage"),
        ["secret token changed: MY_TOKEN -> OTHER_TOKEN"]
    );
    let actions = messages(&report, "action_contract");
    assert_eq!(actions.len(), 1, "{actions:?}");
    let swap = actions.first().copied().ok_or("action change")?;
    assert!(
        swap.starts_with("step act:do changed: noop (id 1)"),
        "{swap}"
    );
    assert!(swap.contains("-> send (id 3)"), "{swap}");
    assert!(swap.contains("side_effect=sends"), "{swap}");
    Ok(())
}

#[test]
fn constant_only_change_is_caught_by_digest_comparison() -> TestResult {
    let report = diff_texts(BASE, &BASE.replace("value: '1'", "value: '7'"))?;
    assert_ne!(report.old_digest, report.new_digest);
    assert_eq!(report.changes.len(), 1, "{:?}", report.changes);
    assert_eq!(
        report
            .changes
            .first()
            .map(|change: &Change| change.category),
        Some("digest")
    );
    Ok(())
}

#[test]
fn spelling_only_edit_changes_the_digest_but_is_not_a_semantic_change() -> TestResult {
    let respelled = BASE.replace("delay: 1s", "delay: 1000ms");
    let report = diff_texts(BASE, &respelled)?;
    assert_ne!(
        report.old_digest, report.new_digest,
        "spelling reaches the digest"
    );
    assert!(report.content_equal);
    assert!(!report.has_semantic_changes(), "{:?}", report.changes);
    let rendered = report.render_text().join("\n");
    assert!(rendered.contains("source spelling only"), "{rendered}");
    assert_eq!(report.to_value()["digest"]["content_equal"], true);
    Ok(())
}

#[test]
fn missing_baseline_file_is_an_error_naming_the_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let new_path = write(dir.path(), "new.yaml", BASE)?;
    let missing = dir.path().join("gone.yaml");
    let error = execute(&new_path, Some(&missing), None)
        .err()
        .ok_or("missing baseline must fail")?;
    assert!(error.contains("gone.yaml"), "{error}");
    assert!(error.contains("error reading"), "{error}");
    Ok(())
}

#[test]
fn uncompilable_baseline_is_an_error_naming_the_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let new_path = write(dir.path(), "new.yaml", BASE)?;
    let broken = write(dir.path(), "broken.yaml", "name: [unterminated\n")?;
    let error = execute(&new_path, Some(&broken), None)
        .err()
        .ok_or("broken baseline must fail")?;
    assert!(error.contains("broken.yaml"), "{error}");
    Ok(())
}

#[test]
fn oversized_source_is_refused_before_compilation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let new_path = write(dir.path(), "new.yaml", BASE)?;
    let huge = write(dir.path(), "huge.yaml", &"# padding\n".repeat(200_000))?;
    let error = execute(&new_path, Some(&huge), None)
        .err()
        .ok_or("oversized baseline must fail")?;
    assert!(error.contains("source limit"), "{error}");
    Ok(())
}

#[test]
fn text_diff_aligns_lines_instead_of_marking_everything_changed() {
    let old = "a\nb\nc\nd\ne\n";
    let new = "a\nc\nd\nx\ne\n";
    let diff = diff_lines(old, new);
    let summary: Vec<(bool, usize, &str)> = diff
        .changes
        .iter()
        .map(|c| (c.added, c.line, c.text.as_str()))
        .collect();
    assert_eq!(summary, [(false, 2, "b"), (true, 4, "x")]);
    assert!(!diff.coarse);
}

#[test]
fn text_diff_detects_a_moved_line_as_remove_plus_add() {
    let diff = diff_lines("a\nb\nc\n", "b\nc\na\n");
    let summary: Vec<(bool, usize, &str)> = diff
        .changes
        .iter()
        .map(|c| (c.added, c.line, c.text.as_str()))
        .collect();
    assert_eq!(summary, [(false, 1, "a"), (true, 3, "a")]);
}

#[test]
fn text_diff_of_oversized_middle_is_flagged_coarse_not_dropped() {
    let old: String = (0..2100).map(|i| format!("old {i}\n")).collect();
    let new: String = (0..2100).map(|i| format!("new {i}\n")).collect();
    let diff = diff_lines(&old, &new);
    assert!(diff.coarse);
    assert_eq!((diff.count(false), diff.count(true)), (2100, 2100));
}

#[test]
fn report_value_is_an_envelope_with_machine_readable_changes() -> TestResult {
    let tuned = BASE.replace("max_attempts: 3", "max_attempts: 2");
    let value = diff_texts(BASE, &tuned)?.to_value();
    assert_eq!(value["kind"], "WorkflowDiff");
    assert_eq!(value["schema_version"], crate::cli_envelope::SCHEMA_VERSION);
    assert_eq!(value["semantic_changes"], true);
    assert_eq!(value["digest"]["equal"], false);
    assert_eq!(value["semantic"]["changes"][0]["category"], "retry_policy");
    assert_eq!(value["text"]["available"], true);
    assert_eq!(value["text"]["removed"], 1);
    Ok(())
}

#[test]
fn text_rendering_lists_each_semantic_change_with_its_category() -> TestResult {
    let tuned = BASE.replace("max_attempts: 3", "max_attempts: 2");
    let lines = diff_texts(BASE, &tuned)?.render_text();
    assert!(
        lines.iter().any(|l| l.starts_with("semantic changes: 1")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("  [retry_policy] step act:do")),
        "{lines:?}"
    );
    let clean = diff_texts(BASE, BASE)?.render_text();
    assert!(
        clean.iter().any(|l| l.starts_with("no semantic changes")),
        "{clean:?}"
    );
    Ok(())
}

#[test]
fn workflow_diff_is_classified_as_a_storage_mode_command() {
    use crate::args::{Command, OutputFormat};
    use crate::mode_error::{CommandMode, command_mode};
    let command = Command::WorkflowDiff {
        workflow: PathBuf::from("new.yaml"),
        against: None,
        db: Some(PathBuf::from("journal")),
        output: OutputFormat::Text,
    };
    assert_eq!(command_mode(&command), CommandMode::Storage);
}
