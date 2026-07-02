use crate::action::{
    ActionFailure, ActionFailureCode, ActionFailureReport, ActionResumeRejection,
    ActionResumeReport, ActionTicket, RetryPolicy, compute_action_idempotency_key,
};
use crate::diagnostic::{DiagnosticCode, HasSymbolicCode};
use crate::errors::CoreError;
use crate::ids::{ActionId, RunId, SeqNo, StepIdx};
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

fn resume_report(rejection: ActionResumeRejection) -> ActionResumeReport {
    let run = RunId::new(11);
    let seq = SeqNo::new(2);
    let action = ActionId::new(7);
    ActionResumeReport::new(
        rejection,
        ActionTicket {
            run,
            step: StepIdx::new(3),
            seq,
            action,
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(run, seq, action),
            capacity: 1,
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
    let report = resume_report(ActionResumeRejection::ActionMismatch);
    let error = CoreError::ActionResumeRejected { report };

    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1508));
    assert_eq!(error.runtime_code(), Some("ACTION_RESUME_REJECTED"));
    assert_eq!(error.symbolic_code().as_str(), "ACTION_RESUME_REJECTED");
    assert_eq!(
        error.to_string(),
        format!("action resume rejected: {report}")
    );
}

#[test]
fn core_error_action_resume_rejected_wraps_report() {
    let report = resume_report(ActionResumeRejection::OutputMismatch);
    let error = CoreError::ActionResumeRejected { report };
    let CoreError::ActionResumeRejected { report: actual } = error else {
        panic!("expected ActionResumeRejected variant");
    };
    assert_eq!(actual, report);
    assert_eq!(actual.rejection.reason(), "action_resume_output_mismatch");
}
