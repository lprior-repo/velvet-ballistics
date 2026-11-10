#![forbid(unsafe_code)]
//! Lifecycle state-transition commands: cancel, resume, retry.

use chrono::Utc;
use vb_core::errors::CoreError;
use vb_core::ids::RunId;
use vb_core::workflow::{LifecycleCommand, LifecycleState, check_lifecycle_transition};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

use super::{EventSeqExt, LifecycleResult, current_state_from_journal};

/// Cancels a running bead.
///
/// # Arguments
///
/// * `run` - The run identifier
/// * `journal` - The journal to write events to
///
/// # Errors
///
/// Returns `CoreError`:
/// - `LifecycleInvalidTransition` if the run is not in a cancelable state
/// - `LifecycleDuplicateRequest` if the run was already cancelled
/// - `LifecycleStaleRequest` if the run state has advanced past the point where cancel is valid
/// - `JournalWriteFailure` if the journal write fails
pub fn cancel(run: RunId, journal: &FjallJournal) -> LifecycleResult<()> {
    let current_state = current_state_from_journal(run, journal)?;

    // Check for duplicate: if already cancelled, return error BEFORE transition check
    if current_state == LifecycleState::Cancelled {
        return Err(CoreError::LifecycleDuplicateRequest {
            code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
            context: "run already cancelled".to_string(),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("cancel"),
        });
    }

    // Check for stale: if state has advanced past Active/WaitingAnswer (terminal states)
    if current_state.is_terminal() {
        return Err(CoreError::LifecycleStaleRequest {
            code: CoreError::LIFECYCLE_STALE_REQUEST_CODE,
            context: format!("run already in terminal state {:?}", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("cancel"),
        });
    }

    // Check if transition is valid (only reached for non-terminal states)
    if !check_lifecycle_transition(current_state, LifecycleCommand::Cancel) {
        return Err(CoreError::LifecycleInvalidTransition {
            code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
            context: format!("cancel not valid from {:?} state", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("cancel"),
        });
    }

    // Write the cancel event
    let next_seq = journal
        .events_for_run(run)
        .map_err(|e| CoreError::JournalWriteFailure {
            code: CoreError::JOURNAL_WRITE_FAILURE_CODE,
            context: format!("failed to read events: {}", e),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?
        .last()
        .map(|e| e.seq().increment())
        .unwrap_or(EventSeq::ZERO);

    let event = JournalEvent::RunCancelled {
        run,
        seq: next_seq,
        attempt: 1,
        reason: None,
    };

    journal
        .append_journaled(&event)
        .map_err(|e| CoreError::JournalWriteFailure {
            code: CoreError::JOURNAL_WRITE_FAILURE_CODE,
            context: e.to_string(),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?;

    Ok(())
}
/// Resumes a cancelled or waiting bead.
///
/// # Arguments
///
/// * `run` - The run identifier
/// * `journal` - The journal to write events to
///
/// # Errors
///
/// Returns `CoreError`:
/// - `LifecycleInvalidTransition` if the run is not in a resumable state
/// - `LifecycleDuplicateRequest` if the run was already resumed
/// - `LifecycleStaleRequest` if the run state has advanced past the point where resume is valid
pub fn resume(run: RunId, journal: &FjallJournal) -> LifecycleResult<()> {
    let current_state = current_state_from_journal(run, journal)?;

    // Check for duplicate: if already active (resumed), return before other checks
    if current_state == LifecycleState::Active {
        return Err(CoreError::LifecycleDuplicateRequest {
            code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
            context: "run already active".to_string(),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("resume"),
        });
    }

    // Resume is valid from Cancelled or WaitingAnswer
    let is_resumable = current_state == LifecycleState::Cancelled
        || current_state == LifecycleState::WaitingAnswer;

    if is_resumable {
        // Valid state, proceed to write event
    } else if current_state == LifecycleState::Completed {
        // Completed is terminal, can't resume
        return Err(CoreError::LifecycleStaleRequest {
            code: CoreError::LIFECYCLE_STALE_REQUEST_CODE,
            context: format!("resume not valid from {:?} state", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("resume"),
        });
    } else {
        // Invalid transition: Pending, Active (caught above), Failed
        return Err(CoreError::LifecycleInvalidTransition {
            code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
            context: format!("resume not valid from {:?} state", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("resume"),
        });
    }

    // Calculate next_seq for the new event
    let next_seq = journal
        .events_for_run(run)
        .map_err(|e| CoreError::JournalWriteFailure {
            code: CoreError::JOURNAL_WRITE_FAILURE_CODE,
            context: format!("failed to read events: {}", e),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?
        .last()
        .map(|e| e.seq().increment())
        .unwrap_or(EventSeq::ZERO);

    // Write the resume event with the correct sequence
    let event = JournalEvent::RunResumed {
        run,
        seq: next_seq,
        timestamp: Utc::now(),
    };

    journal
        .append_journaled(&event)
        .map_err(|e| CoreError::JournalWriteFailure {
            code: CoreError::JOURNAL_WRITE_FAILURE_CODE,
            context: e.to_string(),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?;

    Ok(())
}
/// Retries a failed bead.
///
/// # Arguments
///
/// * `run` - The run identifier
/// * `journal` - The journal to write events to
///
/// # Errors
///
/// Returns `CoreError`:
/// - `LifecycleInvalidTransition` if the run is not in a retryable state
/// - `LifecycleDuplicateRequest` if the run was already retried
/// - `LifecycleStaleRequest` if the run state has advanced past the point where retry is valid
pub fn retry(run: RunId, journal: &FjallJournal) -> LifecycleResult<()> {
    let current_state = current_state_from_journal(run, journal)?;

    // Check for duplicate: if already retried, state is now Active (retry transitions Failed -> Active)
    if current_state == LifecycleState::Active {
        return Err(CoreError::LifecycleDuplicateRequest {
            code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
            context: "run already retried".to_string(),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("retry"),
        });
    }

    // Check for stale: if state is terminal (Completed, Cancelled, Pending), can't retry
    // Only Failed is valid for retry
    if current_state.is_terminal() {
        return Err(CoreError::LifecycleStaleRequest {
            code: CoreError::LIFECYCLE_STALE_REQUEST_CODE,
            context: format!("retry not valid from {:?} state", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("retry"),
        });
    }

    // Check if transition is valid (Failed -> Retry is valid)
    if !check_lifecycle_transition(current_state, LifecycleCommand::Retry) {
        return Err(CoreError::LifecycleInvalidTransition {
            code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
            context: format!("retry not valid from {:?} state", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("retry"),
        });
    }

    // Calculate next_seq for the new event
    let next_seq = journal
        .events_for_run(run)
        .map_err(|e| CoreError::JournalWriteFailure {
            code: CoreError::JOURNAL_WRITE_FAILURE_CODE,
            context: format!("failed to read events: {}", e),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?
        .last()
        .map(|e| e.seq().increment())
        .unwrap_or(EventSeq::ZERO);

    // Write the retry event with the correct sequence
    let event = JournalEvent::RunRetried {
        run,
        seq: next_seq,
        timestamp: Utc::now(),
    };

    journal
        .append_journaled(&event)
        .map_err(|e| CoreError::JournalWriteFailure {
            code: CoreError::JOURNAL_WRITE_FAILURE_CODE,
            context: e.to_string(),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?;

    Ok(())
}
