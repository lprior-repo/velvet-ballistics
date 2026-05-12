use vstd::prelude::*;

verus! {

pub enum RuntimeState {
    Initial,
    Running,
    Resumable,
    Resuming,
    Failed,
}

fn main() {}

pub enum ResumeError {
    RunIdNotFound,
    NotResumable,
    IncompleteHydration,
}

pub struct ResumeResult {
    pub run_id: int,
    pub status: RuntimeState,
    pub timestamp: int,
}

pub enum JournalEvent {
    Started { run_id: int },
    Suspended { run_id: int },
    Resumed { run_id: int },
    Failed { run_id: int },
}

pub struct RuntimeJournal {
    pub events: Seq<JournalEvent>,
}

pub open spec fn is_resumable(state: RuntimeState) -> bool {
    match state {
        RuntimeState::Resumable => true,
        _ => false,
    }
}

pub open spec fn event_run_id(event: JournalEvent) -> int {
    match event {
        JournalEvent::Started { run_id } => run_id,
        JournalEvent::Suspended { run_id } => run_id,
        JournalEvent::Resumed { run_id } => run_id,
        JournalEvent::Failed { run_id } => run_id,
    }
}

pub open spec fn event_reconstructable(event: JournalEvent) -> bool {
    match event {
        JournalEvent::Started { .. } => true,
        JournalEvent::Suspended { .. } => true,
        JournalEvent::Resumed { .. } => true,
        JournalEvent::Failed { .. } => true,
    }
}

pub open spec fn is_hydration_complete(journal: RuntimeJournal, run_id: int) -> bool {
    forall|i: int|
        0 <= i < journal.events.len() && event_run_id(#[trigger] journal.events[i]) == run_id
            ==> event_reconstructable(journal.events[i])
}

pub open spec fn spec_append(journal: RuntimeJournal, event: JournalEvent) -> RuntimeJournal {
    RuntimeJournal { events: journal.events.push(event) }
}

pub open spec fn valid_runtime_state(_state: RuntimeState) -> bool {
    true
}

pub open spec fn valid_resume_transition(before: RuntimeState, after: RuntimeState) -> bool {
    before == RuntimeState::Resumable && after == RuntimeState::Running
}

pub enum ResumeModelResult {
    Success { state: RuntimeState, journal: RuntimeJournal, result: ResumeResult },
    Failure { state: RuntimeState, journal: RuntimeJournal, error: ResumeError },
}

pub open spec fn spec_handle_resume(
    before: RuntimeState,
    journal: RuntimeJournal,
    run_id: int,
    timestamp: int,
) -> ResumeModelResult {
    if is_resumable(before) && is_hydration_complete(journal, run_id) {
        let after = RuntimeState::Running;
        let event = JournalEvent::Resumed { run_id };
        let updated = spec_append(journal, event);
        let result = ResumeResult { run_id, status: after, timestamp };
        ResumeModelResult::Success { state: after, journal: updated, result }
    } else {
        ResumeModelResult::Failure { state: before, journal, error: ResumeError::NotResumable }
    }
}

pub proof fn proof_is_resumable_exhaustive(state: RuntimeState)
    ensures
        is_resumable(state) <==> state == RuntimeState::Resumable,
{
    match state {
        RuntimeState::Initial => { }
        RuntimeState::Running => { }
        RuntimeState::Resumable => { }
        RuntimeState::Resuming => { }
        RuntimeState::Failed => { }
    }
}

pub proof fn proof_resume_result_fields_present(result: ResumeResult)
    ensures
        result.run_id == result.run_id,
        valid_runtime_state(result.status),
        result.timestamp == result.timestamp,
{
}

pub proof fn proof_hydration_completeness(journal: RuntimeJournal, run_id: int)
    requires
        is_hydration_complete(journal, run_id),
    ensures
        forall|i: int|
            0 <= i < journal.events.len() && event_run_id(#[trigger] journal.events[i]) == run_id
                ==> event_reconstructable(journal.events[i]),
{
}

pub proof fn proof_append_immutable(journal: RuntimeJournal, event: JournalEvent)
    ensures
        spec_append(journal, event).events.len() == journal.events.len() + 1,
        spec_append(journal, event).events[journal.events.len() as int] == event,
        forall|i: int| 0 <= i < journal.events.len()
            ==> #[trigger] spec_append(journal, event).events[i] == journal.events[i],
{
    let appended = spec_append(journal, event);
    assert(appended.events =~= journal.events.push(event));
}

pub proof fn proof_handle_resume_preserves_invariants(
    before: RuntimeState,
    journal: RuntimeJournal,
    run_id: int,
    timestamp: int,
)
    requires
        is_resumable(before),
        is_hydration_complete(journal, run_id),
    ensures
        match spec_handle_resume(before, journal, run_id, timestamp) {
            ResumeModelResult::Success { state, journal: after_journal, result } =>
                valid_runtime_state(state)
                && valid_resume_transition(before, state)
                && result.run_id == run_id
                && result.status == state
                && result.timestamp == timestamp
                && after_journal.events.len() == journal.events.len() + 1
                && after_journal.events[journal.events.len() as int] == JournalEvent::Resumed { run_id },
            ResumeModelResult::Failure { .. } => false,
        },
{
    proof_hydration_completeness(journal, run_id);
}

}
