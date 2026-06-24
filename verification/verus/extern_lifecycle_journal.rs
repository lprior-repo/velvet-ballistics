// SPDX-License-Identifier: MIT
//
// Extern surface for vb_qi37_16_5_lifecycle_journal_storage Verus spec.
// Imports the production RuntimeJournalEvent and RuntimeJournal::append:
//   - vb_runtime::journal::RuntimeJournalEvent
//     at crates/vb_runtime/src/journal/chunk_001.rs:15
//   - vb_runtime::journal::RuntimeJournal::append
//     at crates/vb_runtime/src/journal/chunk_001.rs:212-242

#![forbid(unsafe_code)]
#![allow(dead_code)]

/// Production-bound mirror of vb_core::ids::RunId.
/// The production RunId wraps a u64 newtype; the spec mirror uses i64 for SMT.
pub enum RunId {
    Run(i64),
}

/// Production-bound mirror of vb_runtime::journal::RuntimeJournalEvent
/// (the variant set is open in production; we model the canonical subset
/// exercised by the lifecycle state machine).
pub enum RuntimeJournalEvent {
    RunSubmitted { run: RunId },
    RunAdmission { run: RunId },
    RunFinished { run: RunId },
    RunFailed { run: RunId },
    RunCancelled { run: RunId },
    RunKilled { run: RunId },
    ActionScheduled { run: RunId },
    ActionCompleted { run: RunId },
    ActionFailed { run: RunId },
    WaitScheduled { run: RunId },
    WaitResolved { run: RunId },
    AskScheduled { run: RunId },
    AskAnswered { run: RunId },
    AskTimedOut { run: RunId },
    SlotWritten { run: RunId },
    StepStarted { run: RunId },
    StepSucceeded { run: RunId },
    Resumed { run: RunId },
}

impl RuntimeJournalEvent {
    /// Production mirror of `RuntimeJournalEvent::run_id` at
    /// crates/vb_runtime/src/journal/chunk_001.rs:185-208.
    pub fn run_id(&self) -> RunId {
        match self {
            RuntimeJournalEvent::RunSubmitted { run }
            | RuntimeJournalEvent::RunAdmission { run }
            | RuntimeJournalEvent::RunFinished { run }
            | RuntimeJournalEvent::RunFailed { run }
            | RuntimeJournalEvent::RunCancelled { run }
            | RuntimeJournalEvent::RunKilled { run }
            | RuntimeJournalEvent::ActionScheduled { run }
            | RuntimeJournalEvent::ActionCompleted { run }
            | RuntimeJournalEvent::ActionFailed { run }
            | RuntimeJournalEvent::WaitScheduled { run }
            | RuntimeJournalEvent::WaitResolved { run }
            | RuntimeJournalEvent::AskScheduled { run }
            | RuntimeJournalEvent::AskAnswered { run }
            | RuntimeJournalEvent::AskTimedOut { run }
            | RuntimeJournalEvent::SlotWritten { run }
            | RuntimeJournalEvent::StepStarted { run }
            | RuntimeJournalEvent::StepSucceeded { run }
            | RuntimeJournalEvent::Resumed { run } => match run {
                RunId::Run(_) => RunId::Run(0),
            },
        }
    }
}

/// Production-bound mirror of the `RuntimeJournal::append` port at
/// crates/vb_runtime/src/journal/chunk_001.rs:212-242. Pure projection:
/// the journal accepts the event iff the lifecycle transition is valid.
pub fn journal_append_accepts(
    state: i64,           // LifecycleState encoded as int
    command: i64,         // LifecycleCommand encoded as int
) -> bool {
    // command_prior mapping (from the spec):
    //   Cancel(0)  -> Running(1)
    //   Resume(1)  -> Failed(3)
    //   Retry(2)   -> Failed(3)
    //   Answer(3)  -> WaitingForAnswer(2)
    let prior = match command {
        0 => 1, // Cancel -> Running
        1 => 3, // Resume -> Failed
        2 => 3, // Retry -> Failed
        3 => 2, // Answer -> WaitingForAnswer
        _ => -1,
    };
    state == prior
}
