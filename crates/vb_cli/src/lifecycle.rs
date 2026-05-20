#![forbid(unsafe_code)]
//! Lifecycle command surface for bead state management.
//!
//! This module provides the CLI-facing lifecycle commands (cancel, resume,
//! retry, answer) and the journal replay functionality for state recovery.
//!
//! ## State Machine
//!
//! - `Pending`: Run accepted but not yet active
//! - `Active`: Run is executing
//! - `WaitingAnswer`: Run blocked waiting for external answer
//! - `Cancelled`: Run was cancelled
//! - `Completed`: Run finished successfully
//! - `Failed`: Run encountered an error
//!
//! ## Valid Transitions
//!
//! | From State     | Command | Valid |
//! |----------------|---------|-------|
//! | Active         | Cancel  | Yes   |
//! | WaitingAnswer  | Cancel  | Yes   |
//! | WaitingAnswer  | Resume  | Yes   |
//! | Failed         | Retry   | Yes   |
//! | WaitingAnswer  | Answer  | Yes   |

use chrono::Utc;
use vb_core::errors::CoreError;
use vb_core::ids::RunId;
use vb_core::workflow::{LifecycleCommand, LifecycleState, RunState, check_lifecycle_transition};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

/// Result type for lifecycle operations using CoreError.
pub type LifecycleResult<T> = Result<T, CoreError>;

/// In-memory run state tracker.
///
/// Tracks the current state of each run. State is derived from journal
/// replay on startup and maintained in memory during operation.
#[derive(Debug, Default)]
struct RunStateTracker {
    /// Map from run ID to lifecycle state.
    states: std::collections::HashMap<RunId, LifecycleState>,
}

impl RunStateTracker {
    /// Sets the state for a run.
    fn set_state(&mut self, run: RunId, state: LifecycleState) {
        self.states.insert(run, state);
    }
}

// Global state tracker - in production this would be properly managed
// but for integration testing purposes we need in-memory state
static TRACKER: std::sync::LazyLock<std::sync::Mutex<RunStateTracker>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(RunStateTracker::default()));

/// Acquires the tracker lock mutably or returns a storage unavailable error.
fn with_tracker_mut<F, T>(run: RunId, f: F) -> Result<T, CoreError>
where
    F: FnOnce(&mut RunStateTracker) -> Result<T, CoreError>,
{
    let mut tracker = TRACKER
        .lock()
        .map_err(|_| CoreError::LifecycleStorageUnavailable {
            code: CoreError::LIFECYCLE_STORAGE_UNAVAILABLE_CODE,
            context: "tracker lock poisoned".to_string(),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?;
    f(&mut tracker)
}

/// Derives the current lifecycle state for a run directly from the journal.
///
/// This is the primary state lookup used by lifecycle commands. Unlike
/// `replay()` which builds global state, this derives state for a single run
/// by reading its event sequence from the journal.
fn current_state_from_journal(
    run: RunId,
    journal: &FjallJournal,
) -> LifecycleResult<LifecycleState> {
    let events = journal
        .events_for_run(run)
        .map_err(|e| CoreError::ReplayCorruption {
            code: CoreError::REPLAY_CORRUPTION_CODE,
            context: format!("failed to read events for run {:?}: {}", run, e),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?;
    Ok(derive_lifecycle_state_from_events(&events))
}

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

    // Update state
    with_tracker_mut(run, |t| {
        t.set_state(run, LifecycleState::Cancelled);
        Ok(())
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

    // Update state to Active
    with_tracker_mut(run, |t| {
        t.set_state(run, LifecycleState::Active);
        Ok(())
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

    // Update state to Active
    with_tracker_mut(run, |t| {
        t.set_state(run, LifecycleState::Active);
        Ok(())
    })?;

    Ok(())
}

/// Provides an answer to a waiting bead.
///
/// # Arguments
///
/// * `run` - The run identifier
/// * `answer` - The answer content
/// * `journal` - The journal to write events to
///
/// # Errors
///
/// Returns `CoreError`:
/// - `LifecycleInvalidTransition` if the run is not waiting for an answer
/// - `LifecycleDuplicateRequest` if the run already received an answer
/// - `LifecycleStaleRequest` if the run state has advanced past the point where answer is valid
pub fn answer(run: RunId, answer: String, journal: &FjallJournal) -> LifecycleResult<()> {
    let current_state = current_state_from_journal(run, journal)?;

    // Check for duplicate: if already answered, state is Completed
    if current_state == LifecycleState::Completed {
        return Err(CoreError::LifecycleDuplicateRequest {
            code: CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE,
            context: "run already answered".to_string(),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("answer"),
        });
    }

    // Check state: only WaitingAnswer is valid for answer
    if current_state == LifecycleState::WaitingAnswer {
        // Valid state, proceed
    } else if current_state == LifecycleState::Pending {
        // Pending never reached WaitingAnswer - invalid transition
        return Err(CoreError::LifecycleInvalidTransition {
            code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
            context: format!("answer not valid from {:?} state", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("answer"),
        });
    } else {
        // Active, Failed, Cancelled - has passed WaitingAnswer or is terminal, stale request
        return Err(CoreError::LifecycleStaleRequest {
            code: CoreError::LIFECYCLE_STALE_REQUEST_CODE,
            context: format!("answer not valid from {:?} state", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("answer"),
        });
    }

    // Check if transition is valid (WaitingAnswer -> Answer is valid) - redundant but required
    if !check_lifecycle_transition(current_state, LifecycleCommand::Answer) {
        return Err(CoreError::LifecycleInvalidTransition {
            code: CoreError::LIFECYCLE_INVALID_TRANSITION_CODE,
            context: format!("answer not valid from {:?} state", current_state),
            timestamp: Utc::now(),
            bead_id: Some(run),
            command: Some("answer"),
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

    // Write the answer event
    // Note: ConstValue doesn't support String, so we encode the answer as a symbol
    // In production, this would be properly encoded
    let answer_symbol = vb_core::ids::SymbolId::new(
        answer.bytes().fold(0u32, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(u32::from(b))
        }) % u32::MAX,
    );
    let event = JournalEvent::RunAnswered {
        run,
        seq: next_seq,
        slot_idx: vb_core::ids::SlotIdx::new(0), // Default slot for answer
        answer: vb_core::value::ConstValue::Symbol(answer_symbol),
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

    // Update state to Completed
    with_tracker_mut(run, |t| {
        t.set_state(run, LifecycleState::Completed);
        Ok(())
    })?;

    Ok(())
}

/// Replays the journal to reconstruct run states.
///
/// # Arguments
///
/// * `journal` - The journal to replay
///
/// # Errors
///
/// Returns `CoreError`:
/// - `ReplayCorruption` if the journal replay fails due to corruption or sequence gaps
pub fn replay(journal: &FjallJournal) -> LifecycleResult<Vec<RunState>> {
    // Acquire tracker lock to sync with in-memory state during replay
    let mut tracker = TRACKER.lock().map_err(|_| CoreError::ReplayCorruption {
        code: CoreError::REPLAY_CORRUPTION_CODE,
        context: "tracker lock poisoned".to_string(),
        timestamp: Utc::now(),
        bead_id: None,
    })?;

    // Enumerate all runs from the journal header keyspace
    let headers = journal
        .run_headers()
        .map_err(|e| CoreError::ReplayCorruption {
            code: CoreError::REPLAY_CORRUPTION_CODE,
            context: format!("failed to read run headers: {}", e),
            timestamp: Utc::now(),
            bead_id: None,
        })?;

    // For each run, replay events to determine final lifecycle state
    for header in &headers {
        let events =
            journal
                .events_for_run(header.run)
                .map_err(|e| CoreError::ReplayCorruption {
                    code: CoreError::REPLAY_CORRUPTION_CODE,
                    context: format!("replay corruption for run {:?}: {}", header.run, e),
                    timestamp: Utc::now(),
                    bead_id: Some(header.run),
                })?;

        // Derive final state from event sequence
        let final_state = derive_lifecycle_state_from_events(&events);
        tracker.set_state(header.run, final_state);
    }

    // Collect final states for all tracked runs
    let states: Vec<RunState> = tracker
        .states
        .iter()
        .map(|(&run_id, &lifecycle)| RunState { run_id, lifecycle })
        .collect();

    Ok(states)
}

/// Derives the final lifecycle state from a sequence of journal events.
///
/// The last event in the sequence determines the final state:
/// - `RunCancelled` → Cancelled
/// - `RunResumed` → Active
/// - `RunRetried` → Active
/// - `RunAnswered` → Completed
/// - `RunFinished` → Completed
/// - `RunFailedEvent` → Failed
///
/// If no events exist, defaults to Pending.
fn derive_lifecycle_state_from_events(events: &[vb_storage::JournalEvent]) -> LifecycleState {
    events
        .last()
        .map(|e| match e {
            vb_storage::JournalEvent::RunCancelled { .. } => LifecycleState::Cancelled,
            vb_storage::JournalEvent::RunResumed { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::RunRetried { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::RunAnswered { .. } => LifecycleState::Completed,
            vb_storage::JournalEvent::RunFinished { .. } => LifecycleState::Completed,
            vb_storage::JournalEvent::RunFailedEvent { .. } => LifecycleState::Failed,
            vb_storage::JournalEvent::RunAccepted { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::RunAdmission { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::StepStarted { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::StepSucceeded { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::ActionScheduled { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::SlotWrittenEvent { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::ActionCompletedEvent { .. } => LifecycleState::Active,
            vb_storage::JournalEvent::ActionFailedEvent { .. } => LifecycleState::Failed,
            vb_storage::JournalEvent::WaitScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            vb_storage::JournalEvent::AskScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            vb_storage::JournalEvent::AskAnsweredEvent { .. } => LifecycleState::WaitingAnswer,
            vb_storage::JournalEvent::RetryScheduledEvent { .. } => LifecycleState::Active,
            _ => LifecycleState::Active,
        })
        .unwrap_or(LifecycleState::Pending)
}

// Extension trait for EventSeq increment
trait EventSeqExt {
    fn increment(self) -> Self;
}

impl EventSeqExt for EventSeq {
    fn increment(self) -> Self {
        Self::new(self.get().saturating_add(1))
    }
}

// TEST INFRASTRUCTURE — NOT PRODUCTION API
// These helpers bypass journal and are for integration test state setup only.
pub mod test_helpers {
    use super::*;

    /// Sets the lifecycle state for a run directly in the tracker.
    /// **TEST USE ONLY** — bypasses journal, not representative of real lifecycle.
    ///
    /// Required because integration tests must establish valid prior state (PRE-002)
    /// without driving the full lifecycle, which needs complete runtime infra.
    #[allow(unreachable_pub)]
    pub fn set_lifecycle_state(run: RunId, state: LifecycleState) {
        if let Ok(mut tracker) = TRACKER.lock() {
            tracker.set_state(run, state);
        }
    }

    /// Resets all run states in the tracker. **TEST USE ONLY.**
    #[allow(unreachable_pub)]
    pub fn reset_tracker() {
        if let Ok(mut tracker) = TRACKER.lock() {
            tracker.states.clear();
        }
    }

    /// Creates a minimal run header in the journal so that run_headers() returns the run.
    /// **TEST USE ONLY** — for setting up replay test scenarios.
    ///
    /// This is needed because cancel/resume/retry/answer write events but not headers.
    /// Without a header, replay's run_headers() iteration skips the run.
    #[allow(unreachable_pub)]
    pub fn create_run_header(journal: &FjallJournal, run: RunId) {
        use vb_core::WorkflowDigest;
        use vb_core::WorkflowId;
        let header = vb_storage::RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0x42u8; 32]),
            status: 1,
            accepted_at_ms: 0,
        };
        if journal.put_run_header(&header).is_err() {}
    }
}
