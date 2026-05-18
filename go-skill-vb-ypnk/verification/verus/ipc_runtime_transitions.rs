// Obligations: VERUS-IPC-003..005. Production linkage remains
// REFINE-IPC-003..005.
// Pure transition predicates for terminal races, timer eligibility, and
// shutdown monotonicity. I/O, wall clock scheduling, and journal writes are
// intentionally outside this model.

use vstd::prelude::*;

verus! {

pub open spec fn none_terminal(state: int) -> bool { state == 0 }
pub open spec fn completed_terminal(state: int) -> bool { state == 1 }
pub open spec fn cancelled_terminal(state: int) -> bool { state == 2 }
pub open spec fn terminal_state(state: int) -> bool {
    completed_terminal(state) || cancelled_terminal(state)
}

pub open spec fn legal_terminal_transition(before: int, after: int) -> bool {
    (none_terminal(before) && terminal_state(after))
        || (terminal_state(before) && after == before)
}

pub open spec fn timer_eligible(run_exists: bool, terminal: int, cancelled: bool) -> bool {
    run_exists && none_terminal(terminal) && !cancelled
}

pub open spec fn admission_open(state: int) -> bool { state == 0 }
pub open spec fn shutting_down(state: int) -> bool { state == 1 }
pub open spec fn shutdown_closed(state: int) -> bool { state == 2 }
pub open spec fn shutdown_state(state: int) -> bool {
    admission_open(state) || shutting_down(state) || shutdown_closed(state)
}

pub open spec fn shutdown_monotone(before: int, after: int) -> bool {
    shutdown_state(before) && shutdown_state(after) && before <= after
}

pub proof fn single_terminal_winner(before: int, after: int)
    requires
        legal_terminal_transition(before, after),
        terminal_state(before),
    ensures
        after == before,
{
    assert(legal_terminal_transition(before, after));
    assert(terminal_state(before));
}

pub proof fn stale_terminal_event_rejected(before: int)
    requires
        terminal_state(before),
    ensures
        legal_terminal_transition(before, before),
{
    assert(terminal_state(before));
    assert(legal_terminal_transition(before, before));
}

pub proof fn timer_requires_eligible_run(run_exists: bool, terminal: int, cancelled: bool)
    requires
        timer_eligible(run_exists, terminal, cancelled),
    ensures
        run_exists,
        none_terminal(terminal),
        !cancelled,
{
    assert(timer_eligible(run_exists, terminal, cancelled));
}

pub proof fn timer_cannot_mutate_terminal_state(terminal: int, cancelled: bool)
    requires
        terminal_state(terminal),
    ensures
        !timer_eligible(true, terminal, cancelled),
{
    assert(terminal_state(terminal));
    assert(!none_terminal(terminal));
}

pub proof fn shutdown_monotonic(before: int, after: int)
    requires
        shutdown_monotone(before, after),
    ensures
        before <= after,
        shutdown_state(before),
        shutdown_state(after),
{
    assert(shutdown_monotone(before, after));
}

pub proof fn reject_submit_after_shutdown_boundary(state: int)
    requires
        shutting_down(state) || shutdown_closed(state),
    ensures
        !admission_open(state),
{
    assert(state == 1 || state == 2);
    assert(!admission_open(state));
}

fn main() {}

} // verus!
