use crate::action::{
    ActionFailure, ActionFailureCode, ActionFailureReport, ActionResumeRejection, RetryPolicy,
};
use crate::diagnostic::{DiagnosticCode, HasSymbolicCode};
use crate::errors::CoreError;
use crate::ids::{ActionId, StepIdx};
use crate::value::Taint;

fn failure_report() -> ActionFailureReport {
    ActionFailureReport::new(
        StepIdx::new(3),
        ActionId::new(7),
        ActionFailure {
            code: ActionFailureCode::Timeout,
            retry_policy: RetryPolicy::Retryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        },
    )
}

#[test]
fn core_error_action_failed_diagnostic_runtime_and_display() {
    let report = failure_report();
    let error = CoreError::ActionFailed {
        report: report.clone(),
    };

    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1507));
    assert_eq!(error.runtime_code(), Some("ACTION_FAILED"));
    assert_eq!(error.symbolic_code().as_str(), "ACTION_FAILED");
    assert_eq!(error.to_string(), report.to_string());
}

#[test]
fn core_error_action_failed_wraps_report() {
    let report = failure_report();
    let error = CoreError::ActionFailed {
        report: report.clone(),
    };
    let CoreError::ActionFailed { report: actual } = error else {
        panic!("expected ActionFailed variant");
    };
    assert_eq!(actual, report);
}

#[test]
fn core_error_action_resume_rejected_diagnostic_runtime_and_display() {
    let rejection = ActionResumeRejection::ActionMismatch;
    let error = CoreError::ActionResumeRejected { rejection };

    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1508));
    assert_eq!(error.runtime_code(), Some("ACTION_RESUME_REJECTED"));
    assert_eq!(error.symbolic_code().as_str(), "ACTION_RESUME_REJECTED");
    assert_eq!(
        error.to_string(),
        "action resume rejected: action_resume_action_mismatch"
    );
}

#[test]
fn core_error_action_resume_rejected_wraps_rejection() {
    let rejection = ActionResumeRejection::OutputMismatch;
    let error = CoreError::ActionResumeRejected { rejection };
    let CoreError::ActionResumeRejected { rejection: actual } = error else {
        panic!("expected ActionResumeRejected variant");
    };
    assert_eq!(actual, rejection);
    assert_eq!(actual.reason(), "action_resume_output_mismatch");
}
