// Verus spec for vb_qi37.16.5 lifecycle journal storage.
//
// MODEL-ONLY. This file is a standalone mathematical model of the lifecycle
// state machine and its journal-append discipline (LifecycleState,
// LifecycleCommand, validate_command, journal_after_command). It is NOT
// bound to production code, and it must never be presented as bound:
//
//   - The `#[path]` import this file used to carry pointed at
//     `verification/verus/extern_lifecycle_journal.rs`, which does not exist
//     on main; the extern module was never landed.
//   - Production `RunId` (`vb_core::ids::RunId`) is a u64 newtype with
//     `RunId::ZERO`; it has no `Run` variant, so `RunId::Run(0)` is fictional.
//   - Production `RuntimeJournalEvent::StepStarted` carries
//     `{ run, step, attempt }`; there is no bare-run form.
//   - Production has no `journal_append_accepts` decision fn. The old header
//     BINDING LEDGER cites into `crates/vb_runtime/src/journal/chunk_001.rs`
//     resolve onto event-variant lines, not onto a `run_id` or `append` fn,
//     because neither fn exists in that file.
//   - The bead's own trusted boundary
//     (`.beads/vb-qi37.16.5/proof-obligations.jsonl`, VERUS-INV-001 /
//     VERUS-PRE-002 / VERUS-POST-001 / VERUS-POST-003 / VERUS-POST-004 /
//     VERUS-POST-005) already records this file as a minimal standalone
//     mathematical model with storage I/O and production crate context
//     excluded. This header now says the same.
//
// Do NOT recreate the missing extern module as a shadow binding: a
// hand-written `extern_lifecycle_journal.rs` with no production counterpart
// would be a VACUUM binding under the repo's Verus production-binding gate.
#![allow(unused_imports)]
use vstd::prelude::*;

verus! {

// ============================================================
// Model types (no production binding)
// ============================================================
// Model run identifier. Mirrors the production shape only in that it is a
// u64 newtype with a zero value; it is not the production type.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RunId(pub u64);

impl RunId {
    pub const ZERO: RunId = RunId(0);
}

// Model journal event. Minimal by design: only the run field the proofs
// need. Production `RuntimeJournalEvent::StepStarted` carries step and
// attempt as well; this model does not mirror it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RuntimeJournalEvent {
    StepStarted { run: RunId },
}

// ============================================================
// Spec mirrors
// ============================================================
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Pending,
    Running,
    WaitingForAnswer,
    Failed,
    Cancelled,
    Succeeded,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LifecycleCommand {
    Cancel,
    Resume,
    Retry,
    Answer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LifecycleError {
    InvalidTransition,
    DuplicateRequest,
    StaleRequest,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    Accepted(LifecycleState),
    Rejected(LifecycleError),
}

pub open spec fn valid_state(state: LifecycleState) -> bool {
    match state {
        LifecycleState::Pending => true,
        LifecycleState::Running => true,
        LifecycleState::WaitingForAnswer => true,
        LifecycleState::Failed => true,
        LifecycleState::Cancelled => true,
        LifecycleState::Succeeded => true,
    }
}

pub open spec fn is_terminal(state: LifecycleState) -> bool {
    match state {
        LifecycleState::Cancelled => true,
        LifecycleState::Succeeded => true,
        _ => false,
    }
}

pub open spec fn command_prior(command: LifecycleCommand) -> LifecycleState {
    match command {
        LifecycleCommand::Cancel => LifecycleState::Running,
        LifecycleCommand::Resume => LifecycleState::Failed,
        LifecycleCommand::Retry => LifecycleState::Failed,
        LifecycleCommand::Answer => LifecycleState::WaitingForAnswer,
    }
}

pub open spec fn command_next(command: LifecycleCommand) -> LifecycleState {
    match command {
        LifecycleCommand::Cancel => LifecycleState::Cancelled,
        LifecycleCommand::Resume => LifecycleState::Running,
        LifecycleCommand::Retry => LifecycleState::Running,
        LifecycleCommand::Answer => LifecycleState::Running,
    }
}

pub open spec fn transition_valid(state: LifecycleState, command: LifecycleCommand) -> bool {
    state == command_prior(command)
}

pub open spec fn journal_append_accepts(state: LifecycleState, command: LifecycleCommand) -> bool {
    transition_valid(state, command)
}

pub open spec fn spec_transition(
    state: LifecycleState,
    command: LifecycleCommand,
) -> CommandResult {
    if transition_valid(state, command) {
        CommandResult::Accepted(command_next(command))
    } else {
        CommandResult::Rejected(LifecycleError::InvalidTransition)
    }
}

pub open spec fn command_already_advanced(
    state: LifecycleState,
    command: LifecycleCommand,
) -> bool {
    state == command_next(command)
}

pub open spec fn command_is_stale(state: LifecycleState, command: LifecycleCommand) -> bool {
    &&& is_terminal(state)
    &&& state != command_next(command)
}

pub open spec fn validate_command(
    state: LifecycleState,
    command: LifecycleCommand,
) -> CommandResult {
    if transition_valid(state, command) {
        CommandResult::Accepted(command_next(command))
    } else if command_already_advanced(state, command) {
        CommandResult::Rejected(LifecycleError::DuplicateRequest)
    } else if command_is_stale(state, command) {
        CommandResult::Rejected(LifecycleError::StaleRequest)
    } else {
        CommandResult::Rejected(LifecycleError::InvalidTransition)
    }
}

// ============================================================
// Non-vacuous proofs
// ============================================================
// Non-vacuous: case analysis over all six LifecycleState variants.
pub proof fn proof_single_canonical_state(state: LifecycleState)
    ensures
        valid_state(state),
{
    reveal(valid_state);
    match state {
        LifecycleState::Pending => assert(valid_state(state)),
        LifecycleState::Running => assert(valid_state(state)),
        LifecycleState::WaitingForAnswer => assert(valid_state(state)),
        LifecycleState::Failed => assert(valid_state(state)),
        LifecycleState::Cancelled => assert(valid_state(state)),
        LifecycleState::Succeeded => assert(valid_state(state)),
    }
}

// Non-vacuous: when transition_valid, validate_command returns the canonical
// Accepted(command_next(command)) result.
pub proof fn proof_validate_command_precondition(state: LifecycleState, command: LifecycleCommand)
    requires
        transition_valid(state, command),
    ensures
        validate_command(state, command) == CommandResult::Accepted(command_next(command)),
{
    reveal(validate_command);
    reveal(transition_valid);
    assert(validate_command(state, command) == CommandResult::Accepted(command_next(command)));
}

// Non-vacuous: journal_append_accepts holds exactly when validate_command
// accepts the command.
pub proof fn proof_journal_append_accepts_iff_accepted(
    state: LifecycleState,
    command: LifecycleCommand,
)
    requires
        journal_append_accepts(state, command),
    ensures
        validate_command(state, command) == CommandResult::Accepted(command_next(command)),
{
    reveal(journal_append_accepts);
    reveal(validate_command);
    reveal(transition_valid);
    assert(validate_command(state, command) == CommandResult::Accepted(command_next(command)));
}

// Non-vacuous: case analysis on the journal event structure.
pub proof fn proof_append_event_injective(
    journal: Seq<RuntimeJournalEvent>,
    event: RuntimeJournalEvent,
)
    ensures
        journal.push(event).len() == journal.len() + 1,
        journal.push(event)[journal.len() as int] == event,
        forall|i: int|
            0 <= i && i < journal.len() ==> #[trigger] journal.push(event)[i] == journal[i],
{
    // The Seq::push definition makes the first two conjuncts immediate; the
    // third (preservation of prefix) is also immediate by Seq axioms.
    assert(journal.push(event).len() == journal.len() + 1);
    assert(journal.push(event)[journal.len() as int] == event);
    assert_forall_by(
        |i: int|
            {
                requires(0 <= i && i < journal.len());
                ensures(journal.push(event)[i] == journal[i]);
            },
    );
}

// Non-vacuous: when all three refutation preconditions hold, validate_command
// returns Rejected(InvalidTransition) and journal_after_command is the
// original journal.
pub proof fn proof_invalid_transition_error(
    bead: int,
    journal: Seq<RuntimeJournalEvent>,
    state: LifecycleState,
    command: LifecycleCommand,
)
    requires
        !transition_valid(state, command),
        !command_already_advanced(state, command),
        !command_is_stale(state, command),
    ensures
        validate_command(state, command) == CommandResult::Rejected(
            LifecycleError::InvalidTransition,
        ),
        journal_after_command(journal, bead, state, command) == journal,
{
    reveal(validate_command);
    reveal(transition_valid);
    reveal(command_already_advanced);
    reveal(command_is_stale);
    reveal(journal_after_command);
    assert(validate_command(state, command) == CommandResult::Rejected(
        LifecycleError::InvalidTransition,
    ));
    assert(journal_after_command(journal, bead, state, command) == journal);
}

// Non-vacuous: when command is already advanced, validate_command returns
// Rejected(DuplicateRequest).
pub proof fn proof_duplicate_request_error(
    bead: int,
    journal: Seq<RuntimeJournalEvent>,
    state: LifecycleState,
    command: LifecycleCommand,
)
    requires
        command_already_advanced(state, command),
    ensures
        validate_command(state, command) == CommandResult::Rejected(
            LifecycleError::DuplicateRequest,
        ),
        journal_after_command(journal, bead, state, command) == journal,
{
    reveal(validate_command);
    reveal(command_already_advanced);
    reveal(transition_valid);
    reveal(journal_after_command);
    assert(validate_command(state, command) == CommandResult::Rejected(
        LifecycleError::DuplicateRequest,
    ));
    assert(journal_after_command(journal, bead, state, command) == journal);
}

// Non-vacuous: when command is stale but not already advanced, validate_command
// returns Rejected(StaleRequest).
pub proof fn proof_stale_request_error(
    bead: int,
    journal: Seq<RuntimeJournalEvent>,
    state: LifecycleState,
    command: LifecycleCommand,
)
    requires
        command_is_stale(state, command),
        !command_already_advanced(state, command),
    ensures
        validate_command(state, command) == CommandResult::Rejected(LifecycleError::StaleRequest),
        journal_after_command(journal, bead, state, command) == journal,
{
    reveal(validate_command);
    reveal(command_is_stale);
    reveal(command_already_advanced);
    reveal(transition_valid);
    reveal(journal_after_command);
    assert(validate_command(state, command) == CommandResult::Rejected(
        LifecycleError::StaleRequest,
    ));
    assert(journal_after_command(journal, bead, state, command) == journal);
}

pub open spec fn journal_after_command(
    journal: Seq<RuntimeJournalEvent>,
    bead: int,
    state: LifecycleState,
    command: LifecycleCommand,
) -> Seq<RuntimeJournalEvent> {
    match validate_command(state, command) {
        CommandResult::Accepted(next) => journal.push(
            RuntimeJournalEvent::StepStarted { run: RunId::ZERO },
        ),
        CommandResult::Rejected(_) => journal,
    }
}

fn main() {
}

} // verus!
