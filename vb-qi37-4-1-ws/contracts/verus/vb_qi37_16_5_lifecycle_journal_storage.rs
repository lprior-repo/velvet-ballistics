#![allow(unused_imports)]
use vstd::prelude::*;

verus! {

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RuntimeJournalEvent {
    pub bead: int,
    pub command: LifecycleCommand,
    pub from: LifecycleState,
    pub to: LifecycleState,
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

pub open spec fn spec_transition(state: LifecycleState, command: LifecycleCommand) -> CommandResult {
    if transition_valid(state, command) {
        CommandResult::Accepted(command_next(command))
    } else {
        CommandResult::Rejected(LifecycleError::InvalidTransition)
    }
}

pub open spec fn command_already_advanced(state: LifecycleState, command: LifecycleCommand) -> bool {
    state == command_next(command)
}

pub open spec fn command_is_stale(state: LifecycleState, command: LifecycleCommand) -> bool {
    is_terminal(state) && state != command_next(command)
}

pub open spec fn validate_command(state: LifecycleState, command: LifecycleCommand) -> CommandResult {
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

pub open spec fn journal_after_command(
    journal: Seq<RuntimeJournalEvent>,
    bead: int,
    state: LifecycleState,
    command: LifecycleCommand,
) -> Seq<RuntimeJournalEvent> {
    match validate_command(state, command) {
        CommandResult::Accepted(next) => journal.push(RuntimeJournalEvent { bead, command, from: state, to: next }),
        CommandResult::Rejected(_) => journal,
    }
}

proof fn proof_single_canonical_state(state: LifecycleState)
    ensures
        valid_state(state),
{
}

proof fn proof_validate_command_precondition(state: LifecycleState, command: LifecycleCommand)
    requires
        transition_valid(state, command),
    ensures
        validate_command(state, command) == CommandResult::Accepted(command_next(command)),
{
}

proof fn proof_append_event_injective(journal: Seq<RuntimeJournalEvent>, event: RuntimeJournalEvent)
    ensures
        journal.push(event).len() == journal.len() + 1,
        journal.push(event)[journal.len() as int] == event,
        forall|i: int| 0 <= i && i < journal.len() ==> #[trigger] journal.push(event)[i] == journal[i],
{
}

proof fn proof_invalid_transition_error(bead: int, journal: Seq<RuntimeJournalEvent>, state: LifecycleState, command: LifecycleCommand)
    requires
        !transition_valid(state, command),
        !command_already_advanced(state, command),
        !command_is_stale(state, command),
    ensures
        validate_command(state, command) == CommandResult::Rejected(LifecycleError::InvalidTransition),
        journal_after_command(journal, bead, state, command) == journal,
{
}

proof fn proof_duplicate_request_error(bead: int, journal: Seq<RuntimeJournalEvent>, state: LifecycleState, command: LifecycleCommand)
    requires
        command_already_advanced(state, command),
    ensures
        validate_command(state, command) == CommandResult::Rejected(LifecycleError::DuplicateRequest),
        journal_after_command(journal, bead, state, command) == journal,
{
}

proof fn proof_stale_request_error(bead: int, journal: Seq<RuntimeJournalEvent>, state: LifecycleState, command: LifecycleCommand)
    requires
        command_is_stale(state, command),
        !command_already_advanced(state, command),
    ensures
        validate_command(state, command) == CommandResult::Rejected(LifecycleError::StaleRequest),
        journal_after_command(journal, bead, state, command) == journal,
{
}

fn main() {
}

} // verus!
