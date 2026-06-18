#![forbid(unsafe_code)]
//! Retry analysis: determine whether a run can be retried.

use vb_storage::JournalEvent;

// ---------------------------------------------------------------------------
// Analysis result
// ---------------------------------------------------------------------------

/// Analysis result produced by [`analyze_retry`].
#[derive(Debug, Clone)]
pub(crate) struct RetryAnalysis {
    pub failed_at_step: Option<u16>,
    pub last_successful_step: Option<u16>,
    pub can_retry: bool,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scan a journal and decide whether a retry is possible.
///
/// Returns structured data; the caller is responsible for formatting.
pub(crate) fn analyze_retry(events: &[JournalEvent]) -> RetryAnalysis {
    let mut last_successful_step: Option<u16> = None;
    let mut failed_step: Option<u16> = None;

    for event in events {
        match event {
            JournalEvent::StepSucceeded { step, .. } => {
                last_successful_step = Some(step.get());
            }
            JournalEvent::ActionFailedEvent { step, .. } => {
                failed_step = Some(step.get());
            }
            _ => {}
        }
    }

    // Check terminal status.
    //
    // Cancelled runs are explicitly distinct from failed runs: cancellation
    // is an intentional operator action, not a failure, and must not be
    // retried silently. Split the two checks so the cancelled branch can
    // return its own typed reason.
    let terminal = events.last();
    let is_failed = matches!(terminal, Some(JournalEvent::RunFailedEvent { .. }));
    let is_cancelled = matches!(terminal, Some(JournalEvent::RunCancelled { .. }));

    if is_cancelled {
        return RetryAnalysis {
            failed_at_step: None,
            last_successful_step,
            can_retry: false,
            reason: "run was cancelled (cancelled runs cannot be retried)".into(),
        };
    }

    if !is_failed {
        return RetryAnalysis {
            failed_at_step: None,
            last_successful_step,
            can_retry: false,
            reason: "run did not fail (no retry needed)".into(),
        };
    }

    let failure_step = failed_step.or(last_successful_step.map(|s| s.saturating_add(1)));

    RetryAnalysis {
        failed_at_step: failure_step,
        last_successful_step,
        can_retry: true,
        reason: String::new(),
    }
}
