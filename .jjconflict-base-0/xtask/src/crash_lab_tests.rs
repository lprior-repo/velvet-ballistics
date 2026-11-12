use super::*;
use xtask::{XtaskCommand, parse_xtask_command};

#[test]
fn minimal_workflow_parses() {
    assert_eq!(parse_workflow("minimal"), Ok(CrashWorkflow::Minimal));
}

#[test]
fn unknown_workflow_fails_closed_with_available_list() {
    assert_eq!(
        parse_workflow("issue_triage"),
        Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "workflow".to_string(),
            reason: "unknown crash-lab workflow: issue_triage; available workflows: minimal"
                .to_string(),
        })
    );
}

#[test]
fn workflow_names_are_case_sensitive() {
    assert!(parse_workflow("MINIMAL").is_err());
    assert!(parse_workflow("").is_err());
}

#[test]
fn action_dispatch_point_parses() {
    assert_eq!(
        parse_crash_point("action_dispatch"),
        Ok(CrashPoint::ActionDispatch)
    );
}

#[test]
fn unknown_point_fails_closed_with_available_list() {
    assert_eq!(
        parse_crash_point("ActionCompletedBeforeSlotWrite"),
        Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "crash-at".to_string(),
            reason: "unknown crash-lab crash point: ActionCompletedBeforeSlotWrite; available crash points: action_dispatch"
                .to_string(),
        })
    );
}

#[test]
fn point_names_are_case_sensitive() {
    assert!(parse_crash_point("Action_Dispatch").is_err());
    assert!(parse_crash_point("").is_err());
}

#[test]
fn selection_with_crash_at_only_selects_one_point() {
    assert_eq!(
        parse_selection(Some("action_dispatch"), false),
        Ok(CrashSelection::One(CrashPoint::ActionDispatch))
    );
}

#[test]
fn selection_with_all_flag_selects_all_points() {
    assert_eq!(parse_selection(None, true), Ok(CrashSelection::All));
}

#[test]
fn selection_with_both_point_and_all_flag_fails_closed() {
    assert_eq!(
        parse_selection(Some("action_dispatch"), true),
        Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "crash-at".to_string(),
            reason: "pass exactly one of --crash-at <point> or --all-crash-points".to_string(),
        })
    );
}

#[test]
fn selection_with_neither_point_nor_all_flag_fails_closed() {
    assert_eq!(
        parse_selection(None, false),
        Err(XtaskCommandError::InvalidInput {
            command: "crash-lab".to_string(),
            input: "crash-at".to_string(),
            reason: "pass --crash-at <point> or --all-crash-points".to_string(),
        })
    );
}

#[test]
fn action_dispatch_point_targets_the_crash_test() {
    assert_eq!(
        CrashPoint::ActionDispatch.target(),
        ("vb_runtime", "action_dispatch_crash_points")
    );
    assert_eq!(CrashPoint::ActionDispatch.name(), "action_dispatch");
    assert_eq!(available_crash_points(), &["action_dispatch"]);
    assert_eq!(all_crash_points(), &[CrashPoint::ActionDispatch]);
}

#[test]
fn crash_lab_routes_to_the_legacy_cli() {
    let parsed = parse_xtask_command([
        "xtask".into(),
        "crash-lab".into(),
        "--workflow".into(),
        "minimal".into(),
        "--crash-at".into(),
        "action_dispatch".into(),
    ]);
    assert_eq!(parsed, Ok(XtaskCommand::Legacy("crash-lab")));
}
